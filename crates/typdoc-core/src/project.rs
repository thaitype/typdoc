use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::argument::DocumentArg;
use crate::body::{self, Heading};
use crate::config::{
    CONFIG_FILE, Collection, Config, Level, Namespace, RefBase, Report, Rules, config_file,
};
use crate::document::{Document, Value};
use crate::env::Env;
use crate::error::Error;
use crate::frontmatter;
use crate::index::{Entry as Indexed, Index, Member};
use crate::lines::Position;
use crate::links::{self, BodyLink, BodyLinks};
use crate::query::{self, Condition, Dir, FieldRef, PlainCondition, Quant, RefCondition, RefField};
use crate::refs;
use crate::schema::{self, Auto, Field, FieldType, Resolved};
use crate::scope::{self, Scope, Source};
use crate::template::{Step, Template};
use crate::validate::{self, DocName, Finding, Severity, ValidateScope};

/// The folder that holds `.typdoc/config.json`: `TYPDOC_DIR` when it is set, and otherwise
/// the nearest one above the current directory, that directory included.
pub fn discover(env: &dyn Env) -> Result<PathBuf, Error> {
    let cwd = env.current_dir().map_err(Error::io_at(Path::new(".")))?;
    if let Some(dir) = env.var("TYPDOC_DIR").filter(|v| !v.is_empty()) {
        let dir = cwd.join(dir);
        return if config_file(&dir).is_file() {
            Ok(dir)
        } else {
            Err(Error::NoProjectAt { dir })
        };
    }
    cwd.ancestors()
        .find(|dir| config_file(dir).is_file())
        .map(Path::to_owned)
        .ok_or(Error::NoProject { from: cwd })
}

/// The headings of one document, named as the design names a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toc {
    /// Relative to the project the document belongs to.
    pub path: String,
    pub namespace: String,
    /// Present only when the schema has a code.
    pub key: Option<String>,
    /// The alias this document was reached through, when it belongs to an imported project.
    pub project: Option<String>,
    pub headings: Vec<Heading>,
}

/// The name of a document at the other end of a reference, once it is known to exist: `path`,
/// `namespace` and `key` (only for a coded document), the same three parts `get` and `toc` name
/// a document with, plus `project` when the document belongs to an imported project (the alias
/// it was imported under; `None` for a document of this project).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefName {
    pub path: String,
    pub namespace: String,
    pub key: Option<String>,
    pub project: Option<String>,
}

/// What one written ref names, once it is looked up: the document it resolves to, or why it
/// does not (`refs::Reason` under the design's own ids, `"import-absent"` included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefOutcome {
    Resolved(RefName),
    Unresolved(&'static str),
}

/// `refs`' two directions: the refs a document holds, or the refs that hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefsDirection {
    Out,
    In,
}

/// One entry of a `refs` report: the document at the other end (the target for `Out`, the
/// document that holds the ref for `In`, always resolved since it was found by reading it),
/// the field that holds it (`"$body"` for a body link), the text as written, and a position
/// only a body link carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsReference {
    pub other: RefOutcome,
    pub field: String,
    pub written: String,
    pub position: Option<Position>,
}

/// The report of a `refs` run: the document asked about (always resolved, since `refs` reads it
/// the same way `get` and `toc` do), the direction, and its references in the design's order
/// (the fields in document order, each value as written, then `$body` by position; for `In`,
/// grouped by the holder's own path, lexicographically, and then the same order within it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsReport {
    pub document: RefName,
    pub direction: RefsDirection,
    pub refs: Vec<RefsReference>,
}

/// One entry of `Project::incoming_refs`: an outgoing reference found anywhere in the project,
/// with the document that holds it (`holder`) and that document's position in `self.collections`
/// (`collection`), for `refby.*` to filter by field and by where it resolves to, and to read
/// `holder` under its own schema without reading its file again.
struct IncomingRef {
    holder: Document,
    collection: usize,
    reference: RefsReference,
}

/// What `evaluate_ref_condition` needs about the candidate `me` and the project, bundled so the
/// method itself takes one context argument instead of one per piece: `me`'s own path, index
/// entry, parsed fields and body links (read once per candidate by `Project::list`'s own loop),
/// the project's coded schemas (for classifying a bare-key ref), and the precomputed reverse
/// index `refby.*` reads, when at least one `refby.*` condition needs it.
struct RefEvalCtx<'a> {
    me_path: &'a str,
    me_entry: &'a Indexed,
    me_fields: &'a [(String, Value)],
    me_body: &'a BodyLinks,
    codes: &'a BTreeSet<String>,
    incoming: Option<&'a [IncomingRef]>,
}

/// One `--sort field[:asc|:desc]` of a `list` run, already parsed: `field` the same grammar a
/// condition's own field uses (a pseudo-field or a name), `desc` for `:desc`, `false` (`asc`) by
/// default.
#[derive(Debug, Clone)]
pub struct SortKey {
    pub field: FieldRef,
    pub desc: bool,
}

/// `list`'s filter, already parsed by the caller: `--collection` and `--code` (empty means every
/// collection), the `--where` conditions (ANDed), and the `--sort` keys in priority order.
/// `--limit`, `--fields` and `--ids` change nothing about which documents match or their order
/// (design, JSON output: "a flag that... limits what is listed... never [changes] the values of
/// an item"), so they are not part of the filter `list` itself reads; the caller cuts the result
/// for `--limit` and chooses what to print for `--fields`/`--ids`.
#[derive(Debug)]
pub struct ListFilter<'a> {
    pub collections: &'a [String],
    pub codes: &'a [String],
    pub wheres: &'a [Condition],
    pub sort: &'a [SortKey],
}

/// `list`'s result: the matched documents, sorted and cut to `--limit` by nothing here (the
/// caller does that, see `ListFilter`'s own doc), and every dangling ref a `ref.*` condition
/// reached while evaluating `--where`, one line each, ready to print to stderr as written
/// (design, Query, "Reached documents": "dangling refs also warn on stderr"). Empty when no
/// `ref.*` condition reached one, which is the ordinary case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListResult {
    pub documents: Vec<Document>,
    pub dangling_refs: Vec<String>,
}

pub struct Project {
    root: PathBuf,
    config: Config,
    index: Index,
    collections: Vec<Loaded>,
    /// `schema.valid`, gathered once at load: it is always on and never per-document, so its
    /// level is always `error` and there is nothing left to compute once `validate` runs.
    schema_findings: Vec<Finding>,
    /// `filename.pattern`'s candidates, gathered once at load: the path and namespace of every
    /// file directly in a coded collection's folder that fits no collection there. It is
    /// configurable, so its level is decided when `validate` runs, not here.
    stray_files: Vec<(String, String)>,
    /// Every alias of `config.imports` (the project's own, merged with the machine file's),
    /// resolved once at load: absent on this machine, or the imported project itself, loaded
    /// with its own imports left unread (design: "imports of imports are ignored" — one level
    /// only). An alias absent from this map names neither a sibling namespace nor an import, and
    /// a ref or argument using it as an import prefix is `bad-prefix`.
    imports: BTreeMap<String, ImportState>,
}

/// What one configured import resolves to, once `${NAME}` is substituted and the location is
/// checked for a project of its own.
pub(crate) enum ImportState {
    Absent(crate::imports::Absence),
    Loaded(Box<Project>),
}

/// The whole-project context `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved`
/// read beside a document's own frontmatter, bundled because the three always travel together
/// from `Project::validate` through `check_entry` to `check_refs`.
struct RefProject<'a> {
    /// The code of every coded schema in the project (the bare-key ref form's "the code exists
    /// in this project" condition).
    codes: BTreeSet<String>,
    /// The name and code of every collection's schema, by the collection's position.
    schemas: Vec<refs::SchemaInfo<'a>>,
    /// A written ref that no longer resolves, to the current key or path of the document that
    /// recorded moving away from it (`auto: moves`).
    moved: BTreeMap<String, String>,
}

/// The context every checked destination of one document (`check_body_destination`'s callers)
/// shares: everything about the document and its rule levels that stays the same across every
/// link, image and definition `check_body` walks, so a caller passes one reference instead of
/// the same ten values on every call.
struct BodyDocContext<'a> {
    name: &'a DocName<'a>,
    doc_path: &'a str,
    doc_text: &'a str,
    ctx: &'a refs::Ctx<'a>,
    ignore: &'a [Template],
    moved: &'a BTreeMap<String, String>,
    collection: &'a Rules,
    strict: bool,
    links_level: Option<Severity>,
    anchors_level: Option<Severity>,
}

struct Loaded {
    name: String,
    schema: Resolved,
    /// The schema file the collection names directly, relative to the project folder: `schema`
    /// resolved, not a parent reached through `extends`. Kept so `schema.valid`'s duplicate
    /// name and code check can name the file a conflict is with.
    schema_path: String,
    /// The collection file's own `validation`, merged over the project's `validation.global`.
    validation: Rules,
    /// How a relative ref in a document of this collection is resolved: from the document's own
    /// folder or from its namespace's. The sibling-prefixed path form ignores this; only the
    /// unprefixed form reads it (design.md's Refs table).
    ref_base: RefBase,
}

/// The report of a `validate` run, in the shape the design's summary and findings hold, before
/// the CLI turns it into JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateReport {
    pub scope: ValidateScope,
    pub strict: bool,
    /// Sorted, each once.
    pub namespaces: Vec<String>,
    /// 0 for `Schemas`.
    pub documents: usize,
    /// Only for `Paths`: sorted, each once, matching the `path` of every finding.
    pub paths: Option<Vec<String>>,
    /// In the order the design guarantees.
    pub findings: Vec<Finding>,
}

impl Project {
    /// Loads the project at `root`, its schemas, its index, and every import it configures,
    /// followed one level (design: "imports of imports are ignored" — an imported project's own
    /// `imports` are read for `schema.valid`'s name-collision check but never resolved into a
    /// further `Project`).
    pub fn load(root: &Path, env: &dyn Env) -> Result<Project, Error> {
        Project::load_inner(root, env, true)
    }

    fn load_inner(root: &Path, env: &dyn Env, follow_imports: bool) -> Result<Project, Error> {
        let mut report = Report::default();
        let config = Config::load(root, &mut report)?;
        let mut loaded = Vec::new();
        let mut members = Vec::new();
        let mut coded: BTreeMap<String, &str> = BTreeMap::new();
        let mut schema_state = schema::Checked::default();
        let mut schema_findings = Vec::new();
        for collection in &config.collections {
            let template = read_template(collection, &mut report);
            let schema_load = match schema::load(root, collection, &mut report) {
                Ok(Some(schema_load)) => schema_load,
                Ok(None) => continue,
                Err(unreadable) => {
                    // A fault with no id yet does not hide the config errors that have one.
                    report.finish()?;
                    return Err(unreadable);
                }
            };
            for found in schema::check(&schema_load, &mut schema_state) {
                schema_findings.push(validate::schema_finding(
                    &found.path,
                    found.field.as_deref(),
                    found.message,
                ));
            }
            let schema_path = schema_load.chain[0].path.clone();
            let schema = schema::merge(&schema_load.chain);
            if schema.code.is_some() {
                match coded.entry(schema::identity(collection)) {
                    Entry::Vacant(free) => {
                        free.insert(&collection.name);
                    }
                    Entry::Occupied(first) => report.add(
                        "config.coded-schema-shared",
                        &collection.path,
                        format!(
                            "the schema `{}` has a code and the collection `{}` uses it already: one coded schema serves one collection",
                            collection.schema,
                            first.get()
                        ),
                    ),
                }
            }
            let Some(template) = template else { continue };
            match template.bind(&collection.pattern, schema.code.as_deref()) {
                Ok(template) => members.push(Member {
                    name: collection.name.clone(),
                    template,
                }),
                Err(message) => {
                    report.add("config.match-template", &collection.path, message);
                    continue;
                }
            }
            loaded.push(Loaded {
                name: collection.name.clone(),
                schema,
                schema_path,
                validation: collection.validation.clone(),
                ref_base: collection.ref_base,
            });
        }
        // The machine file's own path is found now, while `report` is still open, so a bad
        // `TYPDOC_CONFIG_DIR` (`config.config-dir`) joins every other config error this project
        // has, in the one object `report.finish()` below turns them into — never read as a
        // second, later failure. `follow_imports` is false for an imported project (one level
        // only), so its own machine file is never searched for; its `imports` still contributed
        // to `schema_findings` above (`schema.valid`'s name-collision check reads every alias,
        // not only the followed ones).
        let machine_file = follow_imports
            .then(|| crate::imports::imports_file_path(env, &mut report))
            .flatten();
        report.finish()?;
        schema_findings.extend(duplicate_schema_findings(&loaded));
        for alias in config.imports.keys() {
            if schema::is_scheme_name(alias) {
                schema_findings.push(validate::schema_finding(
                    CONFIG_FILE,
                    None,
                    format!(
                        "the import name `{alias}` has the shape of a URL scheme, and the two would be told apart wrongly"
                    ),
                ));
            }
        }
        validate::order(&mut schema_findings);
        let index = Index::build(root, &config.namespaces, &members)?;
        let stray_files = crate::index::stray_files(root, &config.namespaces, &members)?;
        let imports = if follow_imports {
            let machine = crate::imports::read_machine_file(machine_file.as_ref())?;
            let merged = crate::imports::merge(&config.imports, machine);
            let mut resolved = BTreeMap::new();
            for (alias, raw) in &merged {
                resolved.insert(alias.clone(), resolve_import(root, raw, env)?);
            }
            resolved
        } else {
            BTreeMap::new()
        };
        Ok(Project {
            root: root.to_owned(),
            config,
            index,
            collections: loaded,
            schema_findings,
            stray_files,
            imports,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// This project's own namespaces, for `refs.rs`'s resolution of a ref that lands here after
    /// crossing an import (`resolve_into_import`, `classify_body`'s import branch).
    pub(crate) fn namespaces(&self) -> &[Namespace] {
        &self.config.namespaces
    }

    pub(crate) fn index_ref(&self) -> &Index {
        &self.index
    }

    pub(crate) fn root_ref(&self) -> &Path {
        &self.root
    }

    /// The code of every coded schema in this project, for the bare-key sub-form a ref keeps
    /// once it has crossed into this project through an import.
    pub(crate) fn codes(&self) -> BTreeSet<String> {
        self.project_codes()
    }

    /// The namespaces a command reads. `prefix` is the namespace an argument names.
    pub fn scope(
        &self,
        prefix: Option<&str>,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<Scope, Error> {
        // Owns each loaded import's namespace names for the length of this call, so
        // `ImportListing` below can borrow them: `scope::choose` never outlives this function.
        let loaded: BTreeMap<&str, Vec<String>> = self
            .imports
            .iter()
            .filter_map(|(alias, state)| match state {
                ImportState::Loaded(project) => Some((
                    alias.as_str(),
                    project
                        .config
                        .namespaces
                        .iter()
                        .map(|n| n.name.clone())
                        .collect(),
                )),
                ImportState::Absent(_) => None,
            })
            .collect();
        let listing: Vec<scope::ImportListing> = self
            .imports
            .keys()
            .map(|alias| scope::ImportListing {
                alias,
                namespaces: loaded.get(alias.as_str()).map(Vec::as_slice),
            })
            .collect();
        scope::choose(
            &self.config.namespaces,
            &self.root,
            prefix,
            flag,
            env,
            &listing,
        )
    }

    /// The imported project a `project::` argument prefix names, or why it cannot be read right
    /// now: an alias this project does not configure is bad arguments, the same reading a
    /// `namespace:` prefix naming no sibling already gets (ticket 4/7's own choice); one absent
    /// on this machine is bad arguments too, naming why — an explicit `project::` argument is a
    /// direct request for that document, and the design's own escape for a required import
    /// (`imports.absent` set to `error` in CI) exists for refs, which are implicit; an argument
    /// typed by hand deserves the loud answer immediately, the same choice `--namespace
    /// 'alias::*'` already makes (`scope::select`'s own doc).
    fn imported(&self, alias: &str) -> Result<&Project, Error> {
        match self.imports.get(alias) {
            None => {
                let known: Vec<&str> = self.imports.keys().map(String::as_str).collect();
                Err(Error::BadArgument(crate::imports::unknown_alias_message(
                    alias, &known,
                )))
            }
            Some(ImportState::Absent(absence)) => Err(Error::BadArgument(format!(
                "the import `{alias}` is absent on this machine: {}",
                absence.message()
            ))),
            Some(ImportState::Loaded(project)) => Ok(project),
        }
    }

    pub fn get(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Document, Error> {
        if let Some(alias) = arg.project_prefix() {
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg.namespace_prefix())?;
            let mut document = imported.get(&arg.without_project_prefix(), &inner, env)?;
            document.project = Some(alias.to_owned());
            return Ok(document);
        }
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let bad = |message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        };
        let collection = &self.collections[entry.collection];
        let fields = match frontmatter::block(&text).map_err(bad)? {
            Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
            None => Vec::new(),
        };
        Ok(Document {
            path,
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            code: collection.schema.code.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            project: None,
            fields,
        })
    }

    /// `list`: every document of `scope` whose collection is selected and whose fields satisfy
    /// every `--where` condition, sorted by `--sort` and then, breaking every tie, in key or path
    /// order (`docs/design-decision-phase-1/_tickets/15-json-output-shape.md`, "Already decided
    /// elsewhere and not reopened"). `--limit` is not read here: the design reports `total` before
    /// it and says a flag that limits what is listed never changes an item's values, so cutting
    /// the result is the caller's job, done after this returns the whole match, in order.
    ///
    /// **The scope-wide field check this ticket owns.** The design's Names and scope paragraph
    /// makes a field name unknown to *every* schema in scope an error, while a document whose own
    /// schema merely lacks the field counts as absent; `query::evaluate` sees one schema at a time
    /// and cannot tell those two apart (ticket 13's report). So every plain `--where` condition's
    /// own field is checked here, once, against every schema `--collection`/`--code` (or, absent
    /// those, every collection) selects, before any document is read; only once that has passed
    /// does a document whose own schema happens to lack the field get to fall out as absent,
    /// resolved directly by the op (`!=` satisfied, everything else failed) rather than by calling
    /// `evaluate` with a schema that would misreport it as the scope-wide error.
    ///
    /// **`ref.*`/`refby.*` conditions (ticket 15) get the same upfront check, with their own
    /// scope.** `f` must be a ref/ref[] field (or `$body`) somewhere in the *whole project*, not
    /// only the collections `--collection`/`--code` select: arrows leave that scope freely. Once
    /// `f` is known, the condition after it is checked against the scope the design gives that
    /// condition — the schemas named by `f`'s `target` after `ref.*(f)`, the schemas that declare
    /// `f` after `refby.*(f)`, and every schema of the project for `$body` in either direction —
    /// never against `selected`, which is the plain condition's scope, not this one's.
    ///
    /// **Dangling refs warn (ticket 15).** `ref.*` counts an arrow from the value written even
    /// when it does not resolve (see `evaluate_ref_condition`), and the design says so counting
    /// still warns: "dangling refs also warn on stderr". `refby.*` never contributes a warning,
    /// since an arrow it follows always comes from a document that exists.
    pub fn list(&self, scope: &Scope, filter: &ListFilter) -> Result<ListResult, Error> {
        let selected = self.select_collections(filter.collections, filter.codes)?;
        for condition in filter.wheres {
            match condition {
                Condition::Plain(plain) => {
                    if let FieldRef::Named(name) = &plain.field
                        && !selected
                            .iter()
                            .any(|&i| self.collections[i].schema.field(name).is_some())
                    {
                        return Err(Error::BadArgument(
                            query::QueryError::UnknownField(name.clone()).to_string(),
                        ));
                    }
                }
                Condition::Ref(ref_condition) => self.check_ref_condition_scope(ref_condition)?,
            }
        }
        let codes = self.project_codes();
        // Real body links are only read per candidate when some `ref.*(f)` condition is present
        // (`refby.*` never reads "me"'s own body: it reads every *other* document's outgoing
        // refs, gathered once below): an empty `BodyLinks` gives `document_out_refs` nothing to
        // add under `$body`, which is exactly right for a condition whose own field is a name,
        // never `$body`.
        let needs_own_body = filter
            .wheres
            .iter()
            .any(|c| matches!(c, Condition::Ref(r) if r.dir == Dir::Ref));
        let incoming = if filter
            .wheres
            .iter()
            .any(|c| matches!(c, Condition::Ref(r) if r.dir == Dir::RefBy))
        {
            Some(self.incoming_refs(&codes)?)
        } else {
            None
        };
        let mut matched: Vec<(usize, Document)> = Vec::new();
        let mut dangling_refs: Vec<String> = Vec::new();
        for (path, entry) in self.index.iter() {
            if !selected.contains(&entry.collection) {
                continue;
            }
            let namespace = &self.config.namespaces[entry.namespace].name;
            if !scope.contains(namespace) {
                continue;
            }
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            // A document whose frontmatter block cannot be parsed has no fields to filter, sort
            // or print by; it is left out of the result, the same as it contributes no finding
            // to `validate` beyond `frontmatter.parse` itself and no entry to `refs --reverse`'s
            // scan (design: "no other rule is evaluated for that file").
            let Some(fields) = parsed_fields(&text, &collection.schema) else {
                continue;
            };
            let doc = Document {
                path: path.to_owned(),
                namespace: namespace.clone(),
                key: entry.key.clone(),
                code: collection.schema.code.clone(),
                collection: collection.name.clone(),
                schema: collection.schema.name.clone(),
                project: None,
                fields: fields.clone(),
            };
            let body = if needs_own_body {
                links::scan(&text).expect("frontmatter.parse already refused an unclosed block")
            } else {
                BodyLinks::default()
            };
            let ref_ctx = RefEvalCtx {
                me_path: path,
                me_entry: entry,
                me_fields: &fields,
                me_body: &body,
                codes: &codes,
                incoming: incoming.as_deref(),
            };
            let mut keep = true;
            for condition in filter.wheres {
                let matches = match condition {
                    Condition::Plain(plain) => condition_matches(plain, &collection.schema, &doc)?,
                    Condition::Ref(ref_condition) => {
                        self.evaluate_ref_condition(ref_condition, &ref_ctx, &mut dangling_refs)?
                    }
                };
                if !matches {
                    keep = false;
                    break;
                }
            }
            if keep {
                matched.push((entry.collection, doc));
            }
        }
        matched.sort_by(|(ca, a), (cb, b)| {
            for key in filter.sort {
                let ord = sort_compare(
                    key,
                    &self.collections[*ca].schema,
                    a,
                    &self.collections[*cb].schema,
                    b,
                );
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            compare_identity(a, b)
        });
        // More than one candidate, or more than one `ref.*` condition on the same field, can
        // reach the same dangling ref; sorted and deduplicated so a warning is not printed twice
        // for one.
        dangling_refs.sort();
        dangling_refs.dedup();
        Ok(ListResult {
            documents: matched.into_iter().map(|(_, doc)| doc).collect(),
            dangling_refs,
        })
    }

    /// `list`, widened to the imports `scope.imports` names (`--namespace 'chief::*'`): this
    /// project's own match (`Project::list`, unchanged) plus, for each `(alias, namespaces)`,
    /// that import's own match under its own namespaces — computed by calling `list` again on
    /// the imported project itself (design: "a document... in `list` alike, including a `list`
    /// that reaches an imported project"), so the whole of `--where`/`--sort`/the scope-wide
    /// field check runs exactly as it does for this project, against that project's own schemas.
    /// Every returned `Document` is tagged with `project: Some(alias)`; the combined set is then
    /// sorted once more as a whole (`Project::list` already sorted each half on its own, but a
    /// combined list needs one order, and `sort_compare`/`compare_identity` take an explicit
    /// schema rather than a `self.collections` index, so this works the same for either half).
    /// `scope.namespaces` empty and `scope.imports` non-empty is the ordinary case of `--namespace
    /// 'chief::*'` alone; `dangling_refs` from an import are prefixed with its alias, since a
    /// bare path from another project is ambiguous with this one's.
    pub fn list_all(&self, scope: &Scope, filter: &ListFilter) -> Result<ListResult, Error> {
        let mut result = self.list(scope, filter)?;
        if scope.imports.is_empty() {
            return Ok(result);
        }
        for (alias, namespaces) in &scope.imports {
            let Some(ImportState::Loaded(imported)) = self.imports.get(alias) else {
                unreachable!(
                    "scope::select only ever names an alias this project has loaded: {alias}"
                )
            };
            let import_scope = Scope {
                source: scope.source,
                namespaces: namespaces.clone(),
                imports: Vec::new(),
            };
            let mut theirs = imported.list(&import_scope, filter)?;
            for doc in &mut theirs.documents {
                doc.project = Some(alias.clone());
            }
            result.documents.extend(theirs.documents);
            result.dangling_refs.extend(
                theirs
                    .dangling_refs
                    .into_iter()
                    .map(|w| format!("{alias}::{w}")),
            );
        }
        result.documents.sort_by(|a, b| {
            for key in filter.sort {
                let schema_a = self.schema_for(a);
                let schema_b = self.schema_for(b);
                let ord = sort_compare(key, schema_a, a, schema_b, b);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            compare_identity(a, b)
        });
        result.dangling_refs.sort();
        result.dangling_refs.dedup();
        Ok(result)
    }

    /// The schema of the document `doc` was read under, whichever project it belongs to
    /// (`doc.project`): used only by `list_all`'s combined sort, which cannot reuse `Project::
    /// list`'s own `self.collections[index]` lookup once documents from more than one project are
    /// mixed together.
    fn schema_for(&self, doc: &Document) -> &Resolved {
        let project = match &doc.project {
            None => self,
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => imported,
                _ => self,
            },
        };
        project
            .schema_of_collection(&doc.collection)
            .unwrap_or_else(|| {
                panic!(
                    "list_all only ever builds a Document from a real collection: {} has none named {}",
                    doc.path, doc.collection
                )
            })
    }

    fn schema_of_collection(&self, name: &str) -> Option<&Resolved> {
        self.collections
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.schema)
    }

    /// The collections `--collection`/`--code` select, by their position in `self.collections`:
    /// every collection whose name is in `collections` or whose schema's code is in `codes`, a
    /// union of the two ways of naming one (design: "`--code` is a shorthand that selects the
    /// collections whose schema has that code"). Every collection when both are empty. A name or
    /// a code that matches nothing is refused, the same way an unknown namespace already is,
    /// since it is more likely a typo than an intentional empty scope.
    fn select_collections(
        &self,
        collections: &[String],
        codes: &[String],
    ) -> Result<Vec<usize>, Error> {
        if collections.is_empty() && codes.is_empty() {
            return Ok((0..self.collections.len()).collect());
        }
        let mut selected = BTreeSet::new();
        for name in collections {
            let found = self
                .collections
                .iter()
                .position(|c| &c.name == name)
                .ok_or_else(|| {
                    Error::BadArgument(format!("`{name}` is not a collection of this project"))
                })?;
            selected.insert(found);
        }
        for code in codes {
            let matches: Vec<usize> = self
                .collections
                .iter()
                .enumerate()
                .filter(|(_, c)| c.schema.code.as_deref() == Some(code.as_str()))
                .map(|(i, _)| i)
                .collect();
            if matches.is_empty() {
                return Err(Error::BadArgument(format!(
                    "`{code}` is not the code of any schema in this project"
                )));
            }
            selected.extend(matches);
        }
        Ok(selected.into_iter().collect())
    }

    /// Every collection of the whole project whose schema declares `field` as `ref` or `ref[]`,
    /// by position in `self.collections` — never narrowed to `--collection`/`--code`'s
    /// `selected`, since arrows a `ref.*`/`refby.*` condition follows are not bound by it. `$body`
    /// is declared by every schema implicitly (design: "for `$body` in either it is every schema
    /// in this project and its imports"), so it returns every collection without checking one.
    fn schemas_declaring_ref_field(&self, field: &RefField) -> Vec<usize> {
        let RefField::Named(name) = field else {
            return (0..self.collections.len()).collect();
        };
        self.collections
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                matches!(
                    c.schema.field(name).map(|f| &f.kind),
                    Some(FieldType::Ref | FieldType::RefList)
                )
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The scope the design gives the condition after `ref.*(f)`/`refby.*(f)` (Names and scope):
    /// the schemas named by `f`'s `target` after `ref.*(f)` (every schema of the project when the
    /// target is `"*"`, absent, or a value `schema.valid` already reports as invalid — ticket 10's
    /// reading of an invalid `target` reused here: it places no restriction, since the fault is
    /// already reported once under `schema.valid`), the schemas that declare `f` after
    /// `refby.*(f)` (`declaring`, already computed by the caller), and every schema of the project
    /// for `$body` in either direction.
    fn ref_condition_inner_scope(
        &self,
        dir: Dir,
        field: &RefField,
        declaring: &[usize],
    ) -> Vec<usize> {
        if matches!(field, RefField::Body) {
            return (0..self.collections.len()).collect();
        }
        if matches!(dir, Dir::RefBy) {
            return declaring.to_vec();
        }
        let RefField::Named(name) = field else {
            unreachable!("RefField::Body already returned above")
        };
        let mut unrestricted = false;
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for &i in declaring {
            let target = self.collections[i]
                .schema
                .field(name)
                .and_then(|f| f.target.as_ref());
            match target {
                None | Some(schema::Target::Any) | Some(schema::Target::Other(_)) => {
                    unrestricted = true;
                }
                Some(schema::Target::Schemas(list)) => {
                    names.extend(list.iter().map(String::as_str));
                }
            }
        }
        if unrestricted {
            return (0..self.collections.len()).collect();
        }
        self.collections
            .iter()
            .enumerate()
            .filter(|(_, c)| names.contains(c.schema.name.as_str()))
            .map(|(i, _)| i)
            .collect()
    }

    /// The upfront half of a `ref.*`/`refby.*` condition's own scope-wide check, run once before
    /// any document is read (`Project::list`'s own doc comment explains why): `f` itself must be
    /// declared as a ref/ref[] field somewhere in the project (skipped for `$body`, which is
    /// always valid), and, when the condition carries an inner `.EXPR`, that condition's own named
    /// field must be declared by at least one schema of the scope `ref_condition_inner_scope`
    /// gives it.
    fn check_ref_condition_scope(&self, condition: &RefCondition) -> Result<(), Error> {
        let declaring = self.schemas_declaring_ref_field(&condition.field);
        if !matches!(condition.field, RefField::Body) && declaring.is_empty() {
            return Err(Error::BadArgument(
                query::QueryError::UnknownField(condition.field.name().to_owned()).to_string(),
            ));
        }
        let Some(inner) = &condition.inner else {
            return Ok(());
        };
        let FieldRef::Named(name) = &inner.field else {
            return Ok(());
        };
        let inner_scope =
            self.ref_condition_inner_scope(condition.dir, &condition.field, &declaring);
        if !inner_scope
            .iter()
            .any(|&i| self.collections[i].schema.field(name).is_some())
        {
            return Err(Error::BadArgument(
                query::QueryError::UnknownField(name.clone()).to_string(),
            ));
        }
        Ok(())
    }

    /// One `ref.*`/`refby.*` condition, evaluated against the candidate `me` named by `ctx`: the
    /// arrows `condition.field` names, counted from the value written (dangling refs included
    /// for `ref.*`; `refby.*` arrows always come from a document that exists), each checked
    /// against `condition.inner` when there is one — existence alone otherwise — and combined by
    /// `condition.quant`. Every dangling ref a `ref.*` arrow reaches is appended to `warnings`
    /// (design: "dangling refs also warn on stderr"), whether or not `condition.inner` ends up
    /// mattering to the result — the arrow was still walked and found dangling.
    fn evaluate_ref_condition(
        &self,
        condition: &RefCondition,
        ctx: &RefEvalCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<bool, Error> {
        let wanted = condition.field.name();
        match condition.dir {
            Dir::Ref => {
                let refs: Vec<RefsReference> = self
                    .document_out_refs(
                        ctx.me_path,
                        ctx.me_entry,
                        ctx.me_fields,
                        ctx.me_body,
                        ctx.codes,
                    )
                    .into_iter()
                    .filter(|r| r.field == wanted)
                    .collect();
                let mut results = Vec::with_capacity(refs.len());
                for r in &refs {
                    if let RefOutcome::Unresolved(reason_id) = &r.other {
                        warnings.push(format!(
                            "{}: the ref `{}` in `{}` does not resolve ({reason_id})",
                            ctx.me_path, r.written, r.field
                        ));
                    }
                    results.push(match &condition.inner {
                        None => true,
                        Some(inner) => match &r.other {
                            RefOutcome::Unresolved(_) => inner.op.absent_result(),
                            RefOutcome::Resolved(target) => {
                                self.evaluate_reached_target(target, inner)?
                            }
                        },
                    });
                }
                Ok(combine_quant(condition.quant, results.into_iter()))
            }
            Dir::RefBy => {
                let incoming = ctx
                    .incoming
                    .expect("Project::list precomputes this whenever a refby condition is present");
                let mut results = Vec::new();
                for item in incoming {
                    if item.reference.field != wanted {
                        continue;
                    }
                    let RefOutcome::Resolved(target) = &item.reference.other else {
                        continue;
                    };
                    if target.path != ctx.me_path {
                        continue;
                    }
                    results.push(match &condition.inner {
                        None => true,
                        Some(inner) => condition_matches(
                            inner,
                            &self.collections[item.collection].schema,
                            &item.holder,
                        )?,
                    });
                }
                Ok(combine_quant(condition.quant, results.into_iter()))
            }
        }
    }

    /// One document reached by an outgoing arrow that resolved: read under its own schema when
    /// it is indexed (an ordinary collection member), or, when it resolved outside every
    /// collection (reachable only through `target: "*"`, e.g. a README), a real file with no
    /// schema — every named field is then unknown to it and reads as absent, the same rule a
    /// document whose own schema simply lacks a field already follows, while its pseudo-fields
    /// (`path`, `namespace`) still apply, since the file genuinely exists (design: "Reached
    /// documents are read under their own schema"). A target whose own frontmatter block cannot
    /// be parsed has nothing to test either and is read the same way: absent, never an error, the
    /// same as it is left out of `list`'s own candidates and given no other finding (design: "no
    /// other rule is evaluated for that file").
    /// `evaluate_reached`, dispatched to the project `target` actually names: this project when
    /// `target.project` is `None`, the imported project when it names one — read under that
    /// project's own collections and index, the same as if the query had been run there
    /// directly, since a reached document is "read under its own schema" regardless of which
    /// project asked (design, Query, "Reached documents"). The pattern that would make this
    /// unreachable (`self.imports.get(alias)` finding no loaded import) cannot occur: `target`
    /// only ever carries an alias `ref_name_of_resolved` already confirmed is loaded, the same
    /// invariant `resolve_import_outcome` documents; the fallback reads this project instead of
    /// panicking, since a query condition is not the place to crash a whole run over it.
    fn evaluate_reached_target(
        &self,
        target: &RefName,
        inner: &PlainCondition,
    ) -> Result<bool, Error> {
        match &target.project {
            None => self.evaluate_reached(&target.path, inner),
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => {
                    imported.evaluate_reached(&target.path, inner)
                }
                _ => self.evaluate_reached(&target.path, inner),
            },
        }
    }

    fn evaluate_reached(&self, path: &str, inner: &PlainCondition) -> Result<bool, Error> {
        let Some(entry) = self.index.get(path) else {
            let name = self.ref_name_of(path);
            let empty_schema = Resolved::new(String::new(), None, BTreeMap::new());
            let doc = Document {
                path: path.to_owned(),
                namespace: name.namespace,
                key: None,
                code: None,
                collection: String::new(),
                schema: String::new(),
                project: None,
                fields: Vec::new(),
            };
            return condition_matches(inner, &empty_schema, &doc);
        };
        let collection = &self.collections[entry.collection];
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        let Some(fields) = parsed_fields(&text, &collection.schema) else {
            return Ok(inner.op.absent_result());
        };
        let doc = Document {
            path: path.to_owned(),
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            code: collection.schema.code.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            project: None,
            fields,
        };
        condition_matches(inner, &collection.schema, &doc)
    }

    /// Every outgoing reference of every document of the project, each with the document that
    /// holds it (`holder`) and its position in `self.collections` (`collection`): `refby.*`'s own
    /// reverse index, built once per `list` call that needs it and then filtered per candidate and
    /// per `refby.*` condition, the same computation `refs --reverse` does without a field or a
    /// target filter (`Project::refs`'s own reverse branch).
    fn incoming_refs(&self, codes: &BTreeSet<String>) -> Result<Vec<IncomingRef>, Error> {
        let mut holders: Vec<(&str, &Indexed)> = self.index.iter().collect();
        holders.sort_by_key(|(path, _)| *path);
        let mut out = Vec::new();
        for (path, entry) in holders {
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let Some((fields, body)) = parsed_fields_and_body(&text, &collection.schema) else {
                continue;
            };
            let holder = Document {
                path: path.to_owned(),
                namespace: self.config.namespaces[entry.namespace].name.clone(),
                key: entry.key.clone(),
                code: collection.schema.code.clone(),
                collection: collection.name.clone(),
                schema: collection.schema.name.clone(),
                project: None,
                fields: fields.clone(),
            };
            for reference in self.document_out_refs(path, entry, &fields, &body, codes) {
                out.push(IncomingRef {
                    holder: holder.clone(),
                    collection: entry.collection,
                    reference,
                });
            }
        }
        Ok(out)
    }

    /// The headings of a document's body, in the order of `line`.
    pub fn toc(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Toc, Error> {
        if let Some(alias) = arg.project_prefix() {
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg.namespace_prefix())?;
            let mut toc = imported.toc(&arg.without_project_prefix(), &inner, env)?;
            toc.project = Some(alias.to_owned());
            return Ok(toc);
        }
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let headings = body::headings(&text).map_err(|message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        })?;
        Ok(Toc {
            path,
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            project: None,
            headings,
        })
    }

    /// `refs`: outgoing refs by default, or, with `reverse`, every ref of the whole project that
    /// resolves to the document asked about, the design's `refby` index. `--reverse` scans this
    /// project only. A `project::` argument's own outgoing refs delegate entirely to the
    /// imported project's `refs` (read exactly as if `typdoc` ran inside it), but `--reverse`
    /// with a `project::` argument is refused: the design also wants incoming refs *from this
    /// project* into that document ("refby sees refs from... also the imported projects", read
    /// from the importer's side), which needs this project's own reverse scan to widen into the
    /// import too — not built this ticket, and delegating only to the import's own reverse scan
    /// would silently under-report rather than say so. `field` keeps only the refs held in that
    /// field, `"$body"` for body links, in either direction.
    ///
    /// The document asked about is read the same way `get` and `toc` read theirs (`Project::
    /// resolve`, under `scope`): a broken frontmatter block fails the whole command, matching
    /// `get`. `reverse` scans every document of the project regardless of `scope`, since the
    /// design gives `refby` no scope of its own ("scans every namespace of this project and the
    /// projects it imports"): a document elsewhere in the project whose own frontmatter cannot
    /// be parsed contributes nothing to that scan, the same way it contributes no `refs.resolve`
    /// or `body.*` finding to a whole-project `validate` (design: "no other rule is evaluated
    /// for that file").
    pub fn refs(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        reverse: bool,
        field: Option<&str>,
        env: &dyn Env,
    ) -> Result<RefsReport, Error> {
        if let Some(alias) = arg.project_prefix() {
            if reverse {
                return Err(Error::BadArgument(format!(
                    "`{alias}::` with --reverse is not built yet: refs from this project into an imported document are not scanned; run refs --reverse inside the imported project instead"
                )));
            }
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg.namespace_prefix())?;
            let mut report =
                imported.refs(&arg.without_project_prefix(), &inner, false, field, env)?;
            report.document.project = Some(alias.to_owned());
            return Ok(report);
        }
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let document = self.ref_name_of(&path);
        let codes = self.project_codes();
        let bad = |message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        };
        let collection = &self.collections[entry.collection];
        let fields = match frontmatter::block(&text).map_err(bad)? {
            Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
            None => Vec::new(),
        };
        let body = links::scan(&text).expect("frontmatter.parse already refused an unclosed block");
        let own = self.document_out_refs(&path, entry, &fields, &body, &codes);

        if !reverse {
            let refs = filtered(own, field);
            return Ok(RefsReport {
                document,
                direction: RefsDirection::Out,
                refs,
            });
        }

        // The design orders `in` by the holder's own `path`, lexicographically (`Order`'s rule
        // for `findings` applied the same way to a reference's holder). `Index::iter` promises
        // no order of its own ("the caller sorts what it needs sorted"), so this collects and
        // sorts explicitly rather than leaning on the incidental order a `BTreeMap` happens to
        // give today.
        let mut holders: Vec<(&str, &Indexed)> = self.index.iter().collect();
        holders.sort_by_key(|(other_path, _)| *other_path);
        let mut refs = Vec::new();
        for (other_path, other_entry) in holders {
            let other_collection = &self.collections[other_entry.collection];
            let other_text =
                fs::read_to_string(&other_entry.file).map_err(Error::io_at(&other_entry.file))?;
            let Some((other_fields, other_body)) =
                parsed_fields_and_body(&other_text, &other_collection.schema)
            else {
                continue;
            };
            let outgoing =
                self.document_out_refs(other_path, other_entry, &other_fields, &other_body, &codes);
            for reference in filtered(outgoing, field) {
                let RefOutcome::Resolved(target) = &reference.other else {
                    continue;
                };
                if target.path != path {
                    continue;
                }
                refs.push(RefsReference {
                    other: RefOutcome::Resolved(self.ref_name_of(other_path)),
                    field: reference.field,
                    written: reference.written,
                    position: reference.position,
                });
            }
        }
        Ok(RefsReport {
            document,
            direction: RefsDirection::In,
            refs,
        })
    }

    /// The outgoing references of one document already read, in the design's order for `out`:
    /// its `ref`/`ref[]` fields in the order they appear, each value in the order it is written,
    /// then its body links (`$body`) by position. Kept whether or not each one resolves: `refs`
    /// itself needs the unresolved ones for `out`, and `refs --reverse` needs to call this once
    /// per document of the project and keep only the entries that resolve to the one asked
    /// about (the design's own words for `refby`: the same index `refs --reverse` uses).
    fn document_out_refs(
        &self,
        path: &str,
        entry: &Indexed,
        fields: &[(String, Value)],
        body: &BodyLinks,
        codes: &BTreeSet<String>,
    ) -> Vec<RefsReference> {
        let collection = &self.collections[entry.collection];
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
        };
        let mut refs = Vec::new();
        for (field_name, value) in fields {
            let Some(field) = collection.schema.field(field_name) else {
                continue;
            };
            if !matches!(field.kind, FieldType::Ref | FieldType::RefList)
                || !crate::coerce::fits(&field.kind, value)
            {
                continue;
            }
            for written in ref_values(value) {
                let other = match refs::resolve_one(written, &ctx) {
                    Ok(resolved) => RefOutcome::Resolved(self.ref_name_of_resolved(&resolved)),
                    Err(reason) => RefOutcome::Unresolved(reason_id(&reason)),
                };
                refs.push(RefsReference {
                    other,
                    field: field_name.clone(),
                    written: written.to_owned(),
                    position: None,
                });
            }
        }
        let mut links: Vec<&BodyLink> = body.links.iter().collect();
        links.sort_by_key(|link| (link.line, link.col));
        for link in links {
            // `target: None` is `[t]()` or `[t](#anchor)`: no path at all, so nothing outside
            // this document is named. Ticket 11 reads it as "a self-reference with nothing to
            // check" for `body.links`/`body.anchors`, and the design's own References paragraph
            // only ever speaks of "the document at the other end" — there is none here, so it is
            // not a ref and is left out of `$body` entirely, the same way a URL-scheme link
            // (`BodyDestination::Skip`) is: both name nothing this project can point `refby` at.
            let Some(target) = &link.target else {
                continue;
            };
            let other = match refs::classify_body(target, &ctx) {
                refs::BodyDestination::Skip => continue,
                refs::BodyDestination::BadPrefix => RefOutcome::Unresolved("bad-prefix"),
                refs::BodyDestination::ImportAbsent(_) => RefOutcome::Unresolved("import-absent"),
                refs::BodyDestination::Path(joined) => {
                    match refs::resolve_path(&joined, &self.index, &self.root) {
                        Ok(resolved) => RefOutcome::Resolved(self.ref_name_of(&resolved.path)),
                        Err(_) => RefOutcome::Unresolved("not-found"),
                    }
                }
                refs::BodyDestination::Import { alias, path } => {
                    self.resolve_import_outcome(&alias, &path)
                }
            };
            refs.push(RefsReference {
                other,
                field: "$body".to_owned(),
                written: link.written.clone(),
                position: Some(Position {
                    line: link.line,
                    col: link.col,
                }),
            });
        }
        refs
    }

    /// The name (`path`, `namespace`, `key`) of a document at `path`: looked up in the index
    /// when it is one, which is the only place a coded document's `key` comes from. A path that
    /// resolved but is outside every collection (`target: "*"` accepts one, e.g. a README) has
    /// no entry of its own; its namespace is the one whose folder is the longest prefix of
    /// `path`, or the namespace with no folder (`default`) when none matches, since namespaces
    /// never nest (config rule) and every project has exactly one namespace with an empty
    /// folder.
    fn ref_name_of(&self, path: &str) -> RefName {
        ref_name_in(&self.config.namespaces, &self.index, None, path)
    }

    /// A `refs::Resolved` already known to exist, named: this project's own naming when it
    /// stayed inside this project (`resolved.project` is `None`), the imported project's own
    /// naming when a `name::` prefix carried it across an import (`Some(alias)` — the same
    /// alias `resolve_into_import` tagged it with). `document_out_refs`' own bug before this
    /// existed: calling `self.ref_name_of(&resolved.path)` unconditionally read the *path*
    /// right (a path already fully resolved does not change) but always named it in *this*
    /// project's namespaces, so a frontmatter ref that crossed an import printed no `project`
    /// and the wrong `namespace` whenever the two projects' namespace lists differed.
    fn ref_name_of_resolved(&self, resolved: &refs::Resolved) -> RefName {
        match &resolved.project {
            None => self.ref_name_of(&resolved.path),
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => ref_name_in(
                    imported.namespaces(),
                    imported.index_ref(),
                    Some(alias),
                    &resolved.path,
                ),
                _ => self.ref_name_of(&resolved.path),
            },
        }
    }

    /// A body link's `BodyDestination::Import { alias, path }`, resolved: the alias is already
    /// known to be a loaded import (`refs::classify_body` only builds this variant for one), so
    /// this reads that import's own index and root, never this project's. Whether the target
    /// carries the linked `#anchor`, when there is one, is not checked (`check_body_destination`
    /// leaves the resolved path unread further for this case): that would mean reading the
    /// imported project's headings under its own rules, which is not built this ticket.
    fn resolve_import_outcome(&self, alias: &str, path: &str) -> RefOutcome {
        let Some(ImportState::Loaded(imported)) = self.imports.get(alias) else {
            unreachable!(
                "classify_body builds BodyDestination::Import only for a loaded import: {alias}"
            )
        };
        match refs::resolve_path(path, imported.index_ref(), imported.root_ref()) {
            Ok(resolved) => RefOutcome::Resolved(ref_name_in(
                imported.namespaces(),
                imported.index_ref(),
                Some(alias),
                &resolved.path,
            )),
            Err(_) => RefOutcome::Unresolved("not-found"),
        }
    }

    /// The report of `validate`: `Schemas` when `schemas_only`, `Paths` when `args` is not
    /// empty, and `All` (every document in scope) otherwise. Combining `schemas_only` with
    /// arguments is a caller error and is refused before this runs (design: "combining either
    /// with arguments is bad arguments"), so it is not checked again here.
    pub fn validate(
        &self,
        args: &[DocumentArg],
        schemas_only: bool,
        strict: bool,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<ValidateReport, Error> {
        if schemas_only {
            let scope = self.scope(None, flag, env)?;
            reject_import_scope(&scope)?;
            let mut findings = self.schema_findings.clone();
            findings.extend(self.shadowed_names_findings(strict));
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::Schemas,
                strict,
                namespaces: scope.namespaces,
                documents: 0,
                paths: None,
                findings,
            });
        }
        if args.is_empty() {
            let scope = self.scope(None, flag, env)?;
            reject_import_scope(&scope)?;
            let (ref_project, acyclic) = self.ref_project()?;
            let mut findings = self.schema_findings.clone();
            findings.extend(self.shadowed_names_findings(strict));
            findings.extend(acyclic);
            let mut namespaces = BTreeSet::new();
            let mut documents = 0usize;
            for (path, entry) in self.index.iter() {
                let namespace = &self.config.namespaces[entry.namespace].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                documents += 1;
                let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
                findings.extend(self.check_entry(path, entry, &text, strict, &ref_project));
            }
            // An overlapping path has no one collection to check its frontmatter against, so it
            // was never checked (contract item 8's reasoning for a document whose block cannot
            // be parsed does not reach this far: that document at least had a schema to check
            // it with). `documents` stays a count of documents whose frontmatter was looked at;
            // the namespace is still added, since the scope did cover it.
            for (path, namespace_idx, collections) in self.index.overlaps() {
                let namespace = &self.config.namespaces[namespace_idx].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                findings.push(validate::overlap_finding(
                    path,
                    namespace,
                    overlap_message(collections),
                ));
            }
            for (path, namespace) in &self.stray_files {
                if !scope.contains(namespace) {
                    continue;
                }
                if let Some(level) = validate::effective_level(
                    Level::Error,
                    "filename.pattern",
                    &self.config.validation,
                    &Rules::new(),
                    strict,
                ) {
                    findings.push(validate::stray_file_finding(
                        level,
                        path,
                        namespace,
                        "this file fits no collection's match template in this folder".to_owned(),
                    ));
                }
            }
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::All,
                strict,
                namespaces: namespaces.into_iter().collect(),
                documents,
                paths: None,
                findings,
            });
        }
        let (ref_project, acyclic) = self.ref_project()?;
        let mut findings = Vec::new();
        let mut namespaces = BTreeSet::new();
        let mut paths = BTreeSet::new();
        // An overlapping path named directly is not refused here the way `get` and `toc` refuse
        // it (unlike them, `validate` exists to say what is wrong with a project, and design
        // line 567 names only two things that stop it before any report — an ambiguous key and
        // an argument that names no document — and an overlap is neither). It contributes a
        // `collections.overlap` finding, the same shape the whole-project scan gives it, and no
        // other argument's report is lost over it. It was never checked against a schema (there
        // is none to check it with), so, like the whole-project scan, it does not add to
        // `documents`; kept apart from `paths` for the same reason, so `paths` keeps meaning
        // "the path of each document checked" and stays in step with `documents.len()`. Tracked
        // by its own set so naming it twice reports it once.
        let mut overlapping = BTreeSet::new();
        for arg in args {
            if let Some(alias) = arg.project_prefix() {
                // Ownership rule 1 (design, Refs → Ownership rules): "A file is validated only
                // by the namespace that owns it." An imported project is read-only here and
                // owns its own documents; `validate` never checks them on this project's behalf.
                return Err(Error::BadArgument(format!(
                    "`{alias}::` cannot be validated from here: a file is validated only by the project that owns it"
                )));
            }
            let scope = self.scope(arg.namespace_prefix(), flag, env)?;
            reject_import_scope(&scope)?;
            if let DocumentArg::Path { path, .. } = arg
                && let Some((namespace_idx, collections)) = self.index.overlap(path)
            {
                if overlapping.insert(path.clone()) {
                    let namespace = &self.config.namespaces[namespace_idx].name;
                    namespaces.insert(namespace.clone());
                    findings.push(validate::overlap_finding(
                        path,
                        namespace,
                        overlap_message(collections),
                    ));
                }
                continue;
            }
            let (path, entry, text) = self.resolve(arg, &scope, env)?;
            if paths.insert(path.clone()) {
                let namespace = &self.config.namespaces[entry.namespace].name;
                namespaces.insert(namespace.clone());
                findings.extend(self.check_entry(&path, entry, &text, strict, &ref_project));
            }
        }
        // `refs.acyclic` is always on and its cycles are project-wide, but only a cycle that
        // passes through one of the documents actually named is reported here: unlike the
        // whole-project scan, every finding in this scope is about a document the caller named,
        // and a cycle elsewhere in the project is that other document's own `validate` run to
        // report, not this one's.
        findings.extend(
            acyclic
                .into_iter()
                .filter(|finding| paths.contains(&finding.path)),
        );
        validate::order(&mut findings);
        Ok(ValidateReport {
            scope: ValidateScope::Paths,
            strict,
            namespaces: namespaces.into_iter().collect(),
            documents: paths.len(),
            paths: Some(paths.into_iter().collect()),
            findings,
        })
    }

    /// The findings of one document already read: `frontmatter.parse`, `frontmatter.types` and
    /// `frontmatter.unknown`, at the level the collection's own `validation`, merged over the
    /// project's `validation.global`, and `strict` give them; plus `keys.unique` when its key is
    /// also used by another document of the same namespace. Always on, so `strict` does not
    /// change it. `collections.overlap` is never checked here: a path it is true of is not in
    /// the index at all (`Index::build`), so this is never reached for one; it is a finding of
    /// the whole-project scan instead. `refs` gives `refs.resolve`, `refs.target`,
    /// `refs.codedByPath` and `refs.moved`, skipped when `frontmatter.parse` already fired for
    /// this document (the design: "no other rule is evaluated for that file").
    fn check_entry(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        strict: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let namespace = &self.config.namespaces[entry.namespace].name;
        let name = DocName {
            path,
            namespace,
            collection: &collection.name,
            key: entry.key.as_deref(),
        };
        let mut findings = validate::check_document(
            text,
            &collection.schema,
            &self.config.validation,
            &collection.validation,
            strict,
            &name,
        );
        if let Some(key) = &entry.key
            && let Some(group) = self.index.key_group(entry.namespace, key)
            && group.len() > 1
        {
            let others: Vec<&str> = group
                .iter()
                .map(String::as_str)
                .filter(|other| *other != path)
                .collect();
            findings.push(validate::duplicate_key_finding(
                path,
                namespace,
                &collection.name,
                key,
                format!(
                    "the key `{key}` is also used in this namespace by `{}`",
                    others.join("`, `")
                ),
            ));
        }
        if !findings.iter().any(|f| f.rule == "frontmatter.parse") {
            findings.extend(self.check_refs(path, entry, text, &name, strict, ref_project));
            findings.extend(self.check_body(path, entry, text, &name, strict, ref_project));
        }
        findings
    }

    /// `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved` for every `ref` and
    /// `ref[]` field of one document already known to parse. A value that does not fit its
    /// field's type is skipped: `frontmatter.types` already reported it, and a value that is not
    /// text or a list of text names nothing a ref form could read.
    fn check_refs(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        name: &DocName,
        strict: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let fields = match parsed_fields(text, &collection.schema) {
            Some(fields) => fields,
            None => return Vec::new(),
        };
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes: &ref_project.codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
        };
        let mut findings = Vec::new();
        for (field_name, value) in &fields {
            let Some(field) = collection.schema.field(field_name) else {
                continue;
            };
            if !matches!(field.kind, FieldType::Ref | FieldType::RefList)
                || !crate::coerce::fits(&field.kind, value)
            {
                continue;
            }
            for written in ref_values(value) {
                match refs::resolve_one(written, &ctx) {
                    Err(reason) => findings.extend(self.unresolved_ref_finding(
                        name,
                        field_name,
                        written,
                        reason,
                        &ref_project.moved,
                        &collection.validation,
                        strict,
                    )),
                    // `ref_project.schemas` indexes this project's own collections; `resolved.
                    // collection`, once a ref has crossed into an import (`resolved.project`
                    // is `Some`), indexes that other project's collections instead, which this
                    // project has no list of. `refs.target` and `refs.codedByPath` are checked
                    // only for a ref that stayed inside this project: extending either across an
                    // import needs the imported project's own schema names (`target`'s qualified
                    // form, `"memory::learning"`) or its own coded schemas, neither read here,
                    // and reading `ref_project.schemas[index]` with an index from a different
                    // project's list would be wrong at best and out of bounds at worst.
                    Ok(resolved) if resolved.project.is_some() => {}
                    Ok(resolved) => {
                        if !refs::target_allowed(
                            field.target.as_ref(),
                            &resolved,
                            &ref_project.schemas,
                        ) {
                            findings.push(validate::finding(
                                name,
                                Severity::Error,
                                "refs.target",
                                Some(field_name),
                                format!(
                                    "the ref `{written}` targets `{}`, which `target` does not allow",
                                    resolved.path
                                ),
                            ));
                        }
                        let coded = resolved
                            .collection
                            .is_some_and(|index| ref_project.schemas[index].code.is_some());
                        if coded
                            && resolved.via == refs::Via::Path
                            && let Some(level) = validate::effective_level(
                                Level::Warn,
                                "refs.codedByPath",
                                &self.config.validation,
                                &collection.validation,
                                strict,
                            )
                        {
                            findings.push(validate::finding(
                                name,
                                level,
                                "refs.codedByPath",
                                Some(field_name),
                                format!(
                                    "the ref `{written}` names a coded document by path: use its key instead"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        findings
    }

    /// `body.links`, `body.anchors` and `body.mentions` for one document already known to parse:
    /// every checked link and reference definition `links::scan` finds, resolved the same way a
    /// frontmatter path-form ref is (`refs::classify_body`, `refs::resolve_path`), and every
    /// mention `links::mentions` finds, looked up by key. Nothing here is computed when all three
    /// rules are `off`, since a document with a large body would otherwise be scanned for no
    /// reason.
    fn check_body(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        name: &DocName,
        strict: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let links_level = validate::effective_level(
            Level::Error,
            "body.links",
            &self.config.validation,
            &collection.validation,
            strict,
        );
        let anchors_level = validate::effective_level(
            Level::Error,
            "body.anchors",
            &self.config.validation,
            &collection.validation,
            strict,
        );
        let mentions_level = validate::effective_level(
            Level::Off,
            "body.mentions",
            &self.config.validation,
            &collection.validation,
            strict,
        );
        if links_level.is_none() && anchors_level.is_none() && mentions_level.is_none() {
            return Vec::new();
        }

        let scanned =
            links::scan(text).expect("frontmatter.parse already refused an unclosed block");
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes: &ref_project.codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
        };
        let link_options = rule_options(
            "body.links",
            &self.config.validation,
            &collection.validation,
        );
        let ignore: Vec<Template> = ignore_globs(&link_options)
            .into_iter()
            .filter_map(|glob| Template::parse(&glob).ok())
            .collect();

        let doc = BodyDocContext {
            name,
            doc_path: path,
            doc_text: text,
            ctx: &ctx,
            ignore: &ignore,
            moved: &ref_project.moved,
            collection: &collection.validation,
            strict,
            links_level,
            anchors_level,
        };
        let mut findings = Vec::new();
        // A reference-style occurrence (`is_reference`) is checked once already, at its
        // definition below: the design's own words, "the uses are not reported separately".
        for link in scanned.links.iter().filter(|link| !link.is_reference) {
            self.check_body_destination(
                &link.written,
                link.target.as_deref(),
                link.anchor.as_deref(),
                Position {
                    line: link.line,
                    col: link.col,
                },
                None,
                &doc,
                &mut findings,
            );
        }
        for def in &scanned.definitions {
            self.check_body_destination(
                &def.written,
                def.target.as_deref(),
                def.anchor.as_deref(),
                Position {
                    line: def.line,
                    col: def.col,
                },
                Some(def.uses),
                &doc,
                &mut findings,
            );
        }
        if let Some(level) = links_level {
            for dup in &scanned.duplicate_definitions {
                findings.push(validate::finding_at(
                    name,
                    level,
                    "body.links",
                    None,
                    Position {
                        line: dup.line,
                        col: dup.col,
                    },
                    format!(
                        "already defined at line {}; this definition is ignored",
                        dup.first_line
                    ),
                ));
            }
            for suspect in &scanned.suspects {
                if matches!(
                    refs::classify_body(&suspect.inner, &ctx),
                    refs::BodyDestination::Skip
                ) {
                    continue;
                }
                let message = if suspect.definition {
                    format!(
                        "this line looks like a reference definition but is not one (an unescaped space?): write <{}> or use %20",
                        suspect.inner
                    )
                } else {
                    format!(
                        "this looks like a link but is not one (an unescaped space?): write <{}> or use %20",
                        suspect.inner
                    )
                };
                findings.push(validate::finding_at(
                    name,
                    level,
                    "body.links",
                    None,
                    Position {
                        line: suspect.line,
                        col: suspect.col,
                    },
                    message,
                ));
            }
        }
        if let Some(level) = mentions_level {
            let options = rule_options(
                "body.mentions",
                &self.config.validation,
                &collection.validation,
            );
            let inline_code = bool_option(&options, "inlineCode", true);
            let fenced_code = bool_option(&options, "fencedCode", false);
            let mentions = links::mentions(text, inline_code, fenced_code)
                .expect("frontmatter.parse already refused an unclosed block");
            for mention in mentions {
                if self.mention_missing(&mention.written, entry.namespace, &ref_project.codes)
                    != Some(true)
                {
                    continue;
                }
                let position = Position {
                    line: mention.line,
                    col: mention.col,
                };
                match self.moved_outcome(
                    name,
                    &mention.written,
                    &mention.written,
                    position,
                    &ref_project.moved,
                    &collection.validation,
                    strict,
                ) {
                    Some(Some(finding)) => findings.push(finding),
                    Some(None) => {}
                    None => findings.push(validate::finding_at(
                        name,
                        level,
                        "body.mentions",
                        None,
                        position,
                        format!("{} not found", mention.written),
                    )),
                }
            }
        }
        findings
    }

    /// One checked destination (a link, an image, or a reference definition — `uses` is `Some`
    /// only for the latter, so its finding carries the design's `(used N times)`): resolved the
    /// same way a frontmatter path-form ref is, then, if it resolves and carries a `#anchor`,
    /// checked against the target's own headings. `None` for `target` is `[t](#local)`: the
    /// document names itself, so there is nothing for `body.links` to check and `body.anchors`
    /// reads this document's own headings. `doc` is the same for every destination of one
    /// document; only `written`, `target`, `anchor`, `position` and `uses` change call to call.
    #[allow(
        clippy::too_many_arguments,
        reason = "five parts genuinely vary per destination (written, target, anchor, position,
    uses); doc and findings are the one context and the one output every call shares. Bundling
    the five would hide which one a caller is passing, the same reasoning `unresolved_ref_finding`
    already gives for a comparable list"
    )]
    fn check_body_destination(
        &self,
        written: &str,
        target: Option<&str>,
        anchor: Option<&str>,
        position: Position,
        uses: Option<usize>,
        doc: &BodyDocContext,
        findings: &mut Vec<Finding>,
    ) {
        let name = doc.name;
        // A destination that does not resolve but matches a recorded `auto: moves` entry is
        // `refs.moved`, not `body.links`, exactly as a frontmatter ref already reads it (design:
        // "Refs come from two places, frontmatter fields and body links" — the Refs table's
        // `refs.moved` row is written for "a ref", not for one of the two places alone). Looked
        // up by `target`, the anchor-free path (`moved` never records a `#anchor`), not by
        // `written`: `[t](old.md#section)` must be looked up as `old.md`, or a moved target with
        // an anchor would miss this and silently read as an ordinary `body.links` finding.
        let missing = |findings: &mut Vec<Finding>, lookup: &str| match self.moved_outcome(
            name,
            lookup,
            written,
            position,
            doc.moved,
            doc.collection,
            doc.strict,
        ) {
            Some(Some(finding)) => findings.push(finding),
            Some(None) => {}
            None => {
                if let Some(level) = doc.links_level {
                    findings.push(validate::finding_at(
                        name,
                        level,
                        "body.links",
                        None,
                        position,
                        missing_target_message(written, uses),
                    ));
                }
            }
        };
        let resolved_path = match target {
            None => Some(doc.doc_path.to_owned()),
            Some(target) => match refs::classify_body(target, doc.ctx) {
                refs::BodyDestination::Skip => return,
                refs::BodyDestination::BadPrefix => {
                    missing(findings, target);
                    None
                }
                refs::BodyDestination::ImportAbsent(absence) => {
                    if let Some(level) = self.imports_absent_level(doc.collection, doc.strict) {
                        findings.push(validate::finding_at(
                            name,
                            level,
                            "imports.absent",
                            None,
                            position,
                            format!(
                                "the link `{target}` does not resolve: {}",
                                absence.message()
                            ),
                        ));
                    }
                    None
                }
                refs::BodyDestination::Path(joined) => {
                    if doc.ignore.iter().any(|glob| ignore_matches(glob, &joined)) {
                        return;
                    }
                    match refs::resolve_path(&joined, &self.index, &self.root) {
                        Ok(resolved) => Some(resolved.path),
                        Err(_) => {
                            missing(findings, target);
                            None
                        }
                    }
                }
                refs::BodyDestination::Import { alias, path } => {
                    if doc.ignore.iter().any(|glob| ignore_matches(glob, &path)) {
                        return;
                    }
                    match self.resolve_import_outcome(&alias, &path) {
                        // The document exists in the imported project; whether the `#anchor`
                        // (if any) exists there too is not checked (see `resolve_import_outcome`'s
                        // own doc): `resolved_path` stays `None`, so the anchor check below never
                        // runs for this destination.
                        RefOutcome::Resolved(_) => None,
                        RefOutcome::Unresolved(_) => {
                            missing(findings, target);
                            None
                        }
                    }
                }
            },
        };
        let (Some(level), Some(anchor), Some(target_path)) =
            (doc.anchors_level, anchor, &resolved_path)
        else {
            return;
        };
        let headings = self.target_headings(target_path, doc.doc_path, doc.doc_text);
        let found = headings
            .iter()
            .any(|heading| heading.slug.to_lowercase() == anchor.to_lowercase());
        if !found {
            findings.push(validate::finding_at(
                name,
                level,
                "body.anchors",
                None,
                position,
                format!("the heading `#{anchor}` does not exist in `{target_path}`"),
            ));
        }
    }

    /// The headings of `target_path`, this document's own when it names itself, another
    /// document's read fresh from disk otherwise. A target that cannot be read or parsed is
    /// treated as having no headings, so an anchor into it is loudly reported rather than
    /// silently passed: `body.anchors` only reaches a target `body.links` already resolved, so
    /// this is the read failing after the existence check already passed, not a missing file.
    fn target_headings(&self, target_path: &str, doc_path: &str, doc_text: &str) -> Vec<Heading> {
        if target_path == doc_path {
            return body::headings(doc_text).unwrap_or_default();
        }
        let text = match self.index.get(target_path) {
            Some(entry) => fs::read_to_string(&entry.file).ok(),
            None => fs::read_to_string(self.root.join(target_path)).ok(),
        };
        text.and_then(|text| body::headings(&text).ok())
            .unwrap_or_default()
    }

    /// Whether a mention needs a finding: `None` when its code is not a known code of this
    /// project (design: "not a known code" mentions, such as `UTF-8` or `SHA-256`, are never
    /// checked); `Some(true)` when it does not resolve; `Some(false)` when it does. A prefixed
    /// mention naming no sibling namespace, and one written with `::` (an import, ticket 17's),
    /// both read as "not found": the design gives mentions one outcome for every way a lookup can
    /// fail, unlike a ref's `bad-prefix`.
    fn mention_missing(
        &self,
        written: &str,
        doc_namespace: usize,
        codes: &BTreeSet<String>,
    ) -> Option<bool> {
        let key = written.rsplit(':').next().unwrap_or(written);
        if !codes.contains(refs::code_of(key)) {
            return None;
        }
        if written.contains("::") {
            return Some(true);
        }
        let namespace = match written.rsplit_once(':') {
            Some((prefix, _)) => match self.namespace_index(prefix) {
                Some(namespace) => namespace,
                None => return Some(true),
            },
            None => doc_namespace,
        };
        Some(self.index.key(namespace, key).is_none())
    }

    /// The `refs.moved` outcome for a body destination or a mention that did not otherwise
    /// resolve: `None` when `lookup` matches no recorded move, so the caller reports its own
    /// ordinary missing finding; `Some(None)` when it does but `refs.moved` is configured `off`,
    /// so nothing at all is reported (a rule set to `off` produces no finding, not a fallback to
    /// a different one); `Some(Some(finding))` otherwise. The frontmatter equivalent,
    /// `unresolved_ref_finding`, folds this together with the `refs.resolve` fallback it also
    /// owns; a body destination's and a mention's fallback messages differ from each other and
    /// from a frontmatter ref's, so this stays the one shared part and each caller builds its own
    /// fallback.
    ///
    /// `lookup` is matched against `auto: moves`' recorded values, plain paths and keys that
    /// never carry a `#anchor`; `display` is what the message names. A frontmatter ref and a
    /// mention pass the same string for both; a body link does not, since `[t](old.md#section)`
    /// must be looked up as `old.md`, not `old.md#section`, or a moved target with an anchor
    /// would miss `refs.moved` and silently fall through to `body.links` instead.
    #[allow(
        clippy::too_many_arguments,
        reason = "each part is independent context the two callers (a body destination, a
    mention) already hold; bundling them would hide which one changes between the two, the same
    reasoning `unresolved_ref_finding` already gives for a comparable list"
    )]
    fn moved_outcome(
        &self,
        name: &DocName,
        lookup: &str,
        display: &str,
        position: Position,
        moved: &BTreeMap<String, String>,
        collection: &Rules,
        strict: bool,
    ) -> Option<Option<Finding>> {
        let new_id = moved.get(lookup)?;
        Some(
            validate::effective_level(
                Level::Error,
                "refs.moved",
                &self.config.validation,
                collection,
                strict,
            )
            .map(|level| {
                validate::finding_at(
                    name,
                    level,
                    "refs.moved",
                    None,
                    position,
                    format!("the ref `{display}` no longer resolves: it was moved to `{new_id}`"),
                )
            }),
        )
    }

    /// `refs.moved` when `written` matches a recorded move, `refs.resolve` otherwise (`None` only
    /// when `refs.moved` fires but is configured `off`): the one place that decides between the
    /// two ordinary-missing-target findings, so `check_refs` reads as one branch per outcome of
    /// `resolve_one` rather than a decision nested inside it.
    #[allow(
        clippy::too_many_arguments,
        reason = "each part is independent context a caller already holds (the document's name,
    which field and ref, why it failed, the project's moved records, the collection's own rule
    levels, strict); bundling them would hide which one changes across the two call sites that
    would use it, `check_refs` and a future `body.mentions` (ticket 11, also `refs.moved`)"
    )]
    fn unresolved_ref_finding(
        &self,
        name: &DocName,
        field_name: &str,
        written: &str,
        reason: refs::Reason,
        moved: &BTreeMap<String, String>,
        collection: &Rules,
        strict: bool,
    ) -> Option<Finding> {
        if let Some(new_id) = moved.get(written) {
            let level = validate::effective_level(
                Level::Error,
                "refs.moved",
                &self.config.validation,
                collection,
                strict,
            )?;
            return Some(validate::finding(
                name,
                level,
                "refs.moved",
                Some(field_name),
                format!("the ref `{written}` no longer resolves: it was moved to `{new_id}`"),
            ));
        }
        if let refs::Reason::ImportAbsent(absence) = &reason {
            return self.imports_absent_level(collection, strict).map(|level| {
                validate::finding(
                    name,
                    level,
                    "imports.absent",
                    Some(field_name),
                    format!(
                        "the ref `{written}` does not resolve: {}",
                        absence.message()
                    ),
                )
            });
        }
        Some(validate::finding(
            name,
            Severity::Error,
            "refs.resolve",
            Some(field_name),
            reason_message(written, &reason),
        ))
    }

    /// The level `imports.absent` is reported at, once `validation.global`, `collection`'s own
    /// `validation` and `strict` are merged (default `warn`): `None` means the rule is `off`, so
    /// a ref into an absent import is reported under neither it nor `refs.resolve` — a rule set
    /// to `off` produces no finding, not a fallback to a different one, the same reading
    /// `moved_outcome` already gives `refs.moved`.
    fn imports_absent_level(&self, collection: &Rules, strict: bool) -> Option<Severity> {
        validate::effective_level(
            Level::Warn,
            "imports.absent",
            &self.config.validation,
            collection,
            strict,
        )
    }

    /// The code of every coded schema in the project, for the bare-key ref form's "the code
    /// exists in this project" condition (design.md's Refs table).
    fn project_codes(&self) -> BTreeSet<String> {
        self.collections
            .iter()
            .filter_map(|collection| collection.schema.code.clone())
            .collect()
    }

    /// The name and code of every collection's schema, by the collection's position, for
    /// `refs.target` and `refs.codedByPath`.
    fn schema_infos(&self) -> Vec<refs::SchemaInfo<'_>> {
        self.collections
            .iter()
            .map(|collection| refs::SchemaInfo {
                name: &collection.schema.name,
                code: collection.schema.code.as_deref(),
            })
            .collect()
    }

    /// The whole-project context `check_entry` and `check_refs` read beside a document's own
    /// frontmatter (`codes`, `schemas`, and the `refs.moved` map `prescan_refs` builds), bundled
    /// into one reference since the three always travel together from here down to `check_refs`;
    /// `refs.acyclic`'s findings are returned alongside rather than folded in, since whether they
    /// are reported depends on the scope (see `validate`'s two callers of this).
    fn ref_project(&self) -> Result<(RefProject<'_>, Vec<Finding>), Error> {
        let codes = self.project_codes();
        let schemas = self.schema_infos();
        let (moved, acyclic) = self.prescan_refs(&codes)?;
        Ok((
            RefProject {
                codes,
                schemas,
                moved,
            },
            acyclic,
        ))
    }

    /// `names.shadowed`: a namespace name that is also an import alias, so `name:` and `name::`
    /// reach different documents. A fact about the config, not about a document, so it needs no
    /// document read and is the same in every scope that reports it (`All` and `Schemas`, the
    /// same two `schema_findings` reaches, never `Paths`: see `check_entry`'s callers).
    fn shadowed_names_findings(&self, strict: bool) -> Vec<Finding> {
        let Some(level) = validate::effective_level(
            Level::Warn,
            "names.shadowed",
            &self.config.validation,
            &Rules::new(),
            strict,
        ) else {
            return Vec::new();
        };
        self.config
            .namespaces
            .iter()
            .map(|namespace| namespace.name.as_str())
            .filter(|name| self.config.imports.contains_key(*name))
            .map(|name| {
                validate::names_shadowed_finding(
                    level,
                    CONFIG_FILE,
                    format!(
                        "the name `{name}` is both a sibling namespace and an import alias: `{name}:` and `{name}::` reach different documents"
                    ),
                )
            })
            .collect()
    }

    /// The whole-project pass `refs.moved` and `refs.acyclic` both need before any single
    /// document's refs can be judged: a moved record can be recorded in a document outside the
    /// scope of the run that reads it, and a cycle can pass through documents outside it too.
    /// Reads every document once, distinct from the read `check_entry`'s caller already does for
    /// the ones in scope: cheap next to a network call, and this crate makes none (contract item
    /// 21, "no promised figure for the cost of a run").
    ///
    /// Returns the map from a written ref that no longer resolves to the current key or path of
    /// the document that recorded moving away from it (`auto: moves`), and the `refs.acyclic`
    /// findings of every cycle found through a field marked `acyclic` (one finding per document
    /// on a cycle, per field, since each one's own edge is what is wrong with it).
    fn prescan_refs(
        &self,
        codes: &BTreeSet<String>,
    ) -> Result<(BTreeMap<String, String>, Vec<Finding>), Error> {
        let mut moved: BTreeMap<String, String> = BTreeMap::new();
        let mut edges: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (path, entry) in self.index.iter() {
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let Some(fields) = parsed_fields(&text, &collection.schema) else {
                continue;
            };
            let ctx = refs::Ctx {
                doc_namespace: entry.namespace,
                doc_path: path,
                ref_base: collection.ref_base,
                namespaces: &self.config.namespaces,
                codes,
                index: &self.index,
                root: &self.root,
                imports: &self.imports,
            };
            let identity = entry.key.clone().unwrap_or_else(|| path.to_owned());
            for (field_name, value) in &fields {
                let Some(field) = collection.schema.field(field_name) else {
                    continue;
                };
                if field.auto == Some(Auto::Moves)
                    && field.kind == FieldType::List
                    && let Value::List(items) = value
                {
                    for item in items {
                        moved
                            .entry(item.clone())
                            .or_insert_with(|| identity.clone());
                    }
                }
                if field.is_acyclic()
                    && matches!(field.kind, FieldType::Ref | FieldType::RefList)
                    && crate::coerce::fits(&field.kind, value)
                {
                    for written in ref_values(value) {
                        if let Ok(resolved) = refs::resolve_one(written, &ctx) {
                            edges
                                .entry(field_name.clone())
                                .or_default()
                                .push((path.to_owned(), resolved.path));
                        }
                    }
                }
            }
        }
        let mut findings = Vec::new();
        for (field_name, field_edges) in &edges {
            for cyclic_path in refs::cyclic_nodes(field_edges) {
                let Some(entry) = self.index.get(&cyclic_path) else {
                    continue;
                };
                let collection = &self.collections[entry.collection];
                let namespace = &self.config.namespaces[entry.namespace].name;
                let name = DocName {
                    path: &cyclic_path,
                    namespace,
                    collection: &collection.name,
                    key: entry.key.as_deref(),
                };
                findings.push(validate::finding(
                    &name,
                    Severity::Error,
                    "refs.acyclic",
                    Some(field_name),
                    format!("a cycle passes through `{field_name}`"),
                ));
            }
        }
        // Not sorted here: the caller merges this into a larger set of findings and orders that
        // once, so sorting this slice first would only be thrown away.
        Ok((moved, findings))
    }

    /// The path a document argument names, its place in the index, and the text of the file.
    /// A path is looked up as it stands, relative to the project folder, whether or not it
    /// carries a namespace prefix: a prefix only chooses scope (validated by the caller through
    /// `Project::scope` before this runs) and does not change what the path is read against,
    /// since the path already names the file (ticket 4). A key is resolved against the
    /// namespaces in `scope`, since the same key can be issued once in each of several.
    fn resolve(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        env: &dyn Env,
    ) -> Result<(String, &Indexed, String), Error> {
        let path = match arg {
            DocumentArg::Path { path, .. } => {
                if let Some((_, collections)) = self.index.overlap(path) {
                    return Err(Error::Config {
                        file: self.root.join(path),
                        message: format!(
                            "{}, so there is no one schema to read it with: see collections.overlap in a validate report",
                            overlap_message(collections)
                        ),
                    });
                }
                if self.index.get(path).is_none() {
                    return Err(self.not_found(path, env));
                }
                path.clone()
            }
            DocumentArg::Key { namespace, key, .. } => {
                self.resolve_key(namespace.as_deref(), key, scope)?
            }
        };
        let entry = self
            .index
            .get(&path)
            .expect("the path was just looked up above");
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        Ok((path, entry, text))
    }

    /// The path of the document `key` names, in the namespace `prefix` names or, absent that,
    /// in every namespace of `scope`. More than one match is `Error::AmbiguousKey`; none is
    /// `Error::NotFound`, with no `./name` hint, since a key names no place on disk.
    fn resolve_key(&self, prefix: Option<&str>, key: &str, scope: &Scope) -> Result<String, Error> {
        let found: Vec<(String, String)> = scope
            .namespaces
            .iter()
            .filter_map(|name| self.namespace_index(name).map(|index| (name, index)))
            .filter_map(|(name, index)| {
                self.index
                    .key(index, key)
                    .map(|path| (name.clone(), path.to_owned()))
            })
            .collect();
        match found.len() {
            0 => Err(Error::NotFound {
                path: printed_key(prefix, key),
                hint: false,
            }),
            1 => Ok(found.into_iter().next().expect("checked above").1),
            _ => Err(Error::AmbiguousKey {
                key: key.to_owned(),
                candidates: found
                    .into_iter()
                    .map(|(namespace, _)| format!("{namespace}:{key}"))
                    .collect(),
            }),
        }
    }

    fn namespace_index(&self, name: &str) -> Option<usize> {
        self.config.namespaces.iter().position(|n| n.name == name)
    }

    /// A path relative to the project that names nothing in the index; the message names
    /// `./path`, relative to the current directory, when a file is there, as a suggestion and
    /// not a substitution.
    fn not_found(&self, path: &str, env: &dyn Env) -> Error {
        let hint = env.current_dir().is_ok_and(|cwd| cwd.join(path).is_file());
        Error::NotFound {
            path: path.to_owned(),
            hint,
        }
    }
}

/// The fields of a document already read, or `None` when its block cannot be parsed: the ref
/// rules read no document `frontmatter.parse` has already refused (design: "no other rule is
/// evaluated for that file"), and this is the one place that reads a block a second time to get
/// them, since `validate::check_document` does not hand its own parse back out.
fn parsed_fields(text: &str, schema: &Resolved) -> Option<Vec<(String, Value)>> {
    let block = frontmatter::block(text).ok()??;
    frontmatter::fields(block, schema).ok()
}

/// `list`'s per-document half of the scope-wide field check `Project::list` owns: a condition
/// whose field is unknown to `schema` in scope (checked before this ever runs) reaches this only
/// because *some other* schema in scope defines it, so `doc` is simply absent that field — never
/// `query::evaluate`'s `UnknownField`, which would misreport a document whose own schema lacks a
/// field the scope otherwise knows. Absence resolves by the op alone, exactly as the design's
/// Absence and negation paragraph gives it for a missing field: `!=` is satisfied, `=` in any
/// form and every ordering comparison fail. A pseudo-field is never unknown to a schema (it
/// applies, or does not, independently of one), so it always reaches `evaluate` below. Also the
/// inner condition of a `ref.*`/`refby.*` reached at a real document (ticket 15): the scope check
/// for that inner condition is `check_ref_condition_scope`'s job, not this one's, exactly as the
/// outer, plain-condition scope check is `Project::list`'s and not this function's.
fn condition_matches(
    condition: &PlainCondition,
    schema: &Resolved,
    doc: &Document,
) -> Result<bool, Error> {
    if let FieldRef::Named(name) = &condition.field
        && schema.field(name).is_none()
    {
        return Ok(condition.op.absent_result());
    }
    query::evaluate(condition, schema, doc).map_err(|e| Error::BadArgument(e.to_string()))
}

/// The design's table for `ref.*`/`refby.*`: `all` true when every item is, and true on an empty
/// set ("true when empty"); `any` true when some item is, and false on an empty set ("false when
/// empty"); `none` true when no item is, and true on an empty set ("true when empty"). One
/// function for both "does this arrow satisfy the inner condition" (`Some(inner)`, one bool per
/// arrow) and "does an arrow merely exist" (`inner: None`, a `true` per arrow already baked into
/// `items` by the caller), since the quantifier's own meaning does not change between them.
fn combine_quant(quant: Quant, items: impl Iterator<Item = bool>) -> bool {
    let mut items = items;
    match quant {
        Quant::All => items.all(|matched| matched),
        Quant::Any => items.any(|matched| matched),
        Quant::None => !items.any(|matched| matched),
    }
}

/// A sort key's value, comparable independently of type: `Missing` is greater than every other
/// variant so it sorts last however the comparison is later reversed for `:desc` (Sorting under
/// `list`, "Missing value | Last, in both directions"). Two fields of the same name whose schemas
/// disagree on type compare as `Equal` (falling through to the next sort key, or to key/path
/// order): the design gives sorting rules per declared type and says nothing about two schemas
/// giving one field name different types, so this is read as a tie rather than an arbitrary
/// cross-type order.
enum SortValue {
    Missing,
    Number(f64),
    Bool(bool),
    Date(NaiveDate),
    Datetime(DateTime<FixedOffset>),
    /// Position in the schema's own `values`, for an `enum` field.
    EnumPos(usize),
    /// A `key`: the part before the trailing digits, and the digits read as a number, so `WF-2`
    /// sorts before `WF-10` (Sorting under `list`).
    Key(String, Option<u64>),
    Text(String),
}

impl SortValue {
    fn cmp_same_type(&self, other: &SortValue) -> Ordering {
        match (self, other) {
            (SortValue::Missing, SortValue::Missing) => Ordering::Equal,
            (SortValue::Number(a), SortValue::Number(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (SortValue::Bool(a), SortValue::Bool(b)) => a.cmp(b),
            (SortValue::Date(a), SortValue::Date(b)) => a.cmp(b),
            (SortValue::Datetime(a), SortValue::Datetime(b)) => a.cmp(b),
            (SortValue::EnumPos(a), SortValue::EnumPos(b)) => a.cmp(b),
            (SortValue::Key(pa, na), SortValue::Key(pb, nb)) => {
                pa.cmp(pb).then_with(|| cmp_key_number(*na, *nb))
            }
            (SortValue::Text(a), SortValue::Text(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

fn cmp_key_number(a: Option<u64>, b: Option<u64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
    }
}

/// `key.field`'s value on `doc`, read under `doc`'s own schema, as a `SortValue`: a pseudo-field
/// other than `key` is always present and sorts as `Text`; `key` and a named field read as
/// `Missing` when `doc` does not carry one. A named field's declared type picks the variant
/// (Sorting under `list`); a value that does not fit it (kept as written, `coerce` already
/// returning `None` for it) sorts as `Missing`, the same as a genuinely absent field, since there
/// is no typed value to compare.
///
/// **A field name no schema in scope declares is not checked here, on purpose.** The design's
/// Names and scope paragraph ties its "unknown to every schema is an error" rule to "a plain
/// condition" — a `--where`/`--if` expression, the seam ticket 13 built and `Project::list`
/// checks scope-wide before evaluating any document (see `list`'s own doc comment). Sorting has
/// its own paragraph and its own table, and neither states a validation rule: a name no schema
/// declares simply means `doc.fields` never has it, so every document reads `Missing` here and
/// the sort falls through to the next key, or to key/path order. This differs from `--where` in
/// what a mistake costs: an unrecognised `--where` field could silently change which documents
/// match (or look like an empty result); an unrecognised `--sort` field changes nothing about
/// `total`, `truncated` or which documents are returned, only their order, so there is no result
/// that reads as more complete or more correct than it is.
fn sort_value(field: &FieldRef, schema: &Resolved, doc: &Document) -> SortValue {
    match field {
        FieldRef::Path => SortValue::Text(doc.path.clone()),
        FieldRef::Namespace => SortValue::Text(doc.namespace.clone()),
        FieldRef::Collection => SortValue::Text(doc.collection.clone()),
        FieldRef::Schema => SortValue::Text(doc.schema.clone()),
        FieldRef::Code => doc.code.clone().map_or(SortValue::Missing, SortValue::Text),
        FieldRef::Key => match &doc.key {
            Some(key) => key_sort_value(key),
            None => SortValue::Missing,
        },
        FieldRef::Named(name) => {
            let Some(schema_field) = schema.field(name) else {
                return SortValue::Missing;
            };
            let Some(value) = query::field_value(field, doc) else {
                return SortValue::Missing;
            };
            named_sort_value(schema_field, &value)
        }
    }
}

fn key_sort_value(key: &str) -> SortValue {
    let split = key
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    if split == key.len() {
        return SortValue::Key(key.to_owned(), None);
    }
    let (prefix, digits) = key.split_at(split);
    SortValue::Key(prefix.to_owned(), digits.parse::<u64>().ok())
}

fn named_sort_value(field: &Field, value: &Value) -> SortValue {
    match &field.kind {
        FieldType::Number => match value {
            Value::Number(n) => n.as_f64().map_or(SortValue::Missing, SortValue::Number),
            _ => SortValue::Missing,
        },
        FieldType::Bool => match value {
            Value::Bool(b) => SortValue::Bool(*b),
            _ => SortValue::Missing,
        },
        FieldType::Date => match value {
            Value::Date(text) => {
                query::parse_date(text).map_or(SortValue::Missing, SortValue::Date)
            }
            _ => SortValue::Missing,
        },
        FieldType::Datetime => match value {
            Value::Datetime(text) => {
                query::parse_datetime(text).map_or(SortValue::Missing, SortValue::Datetime)
            }
            _ => SortValue::Missing,
        },
        FieldType::Enum => match value {
            Value::Text(text) => field
                .values
                .as_deref()
                .and_then(|values| values.iter().position(|v| v == text))
                .map_or(SortValue::Missing, SortValue::EnumPos),
            _ => SortValue::Missing,
        },
        // `list`, `ref` and `ref[]` have no rule of their own in the Sorting table; read as
        // plain text of the value as written, the same bucket as `string`, since the design
        // never singles them out and a fallback that always ties would make `--sort` on such a
        // field indistinguishable from not sorting at all.
        _ => match value {
            Value::Text(text) => SortValue::Text(text.clone()),
            Value::List(items) => SortValue::Text(items.join(",")),
            _ => SortValue::Missing,
        },
    }
}

/// One `--sort` key's contribution to the order of `a` (under `schema_a`) against `b` (under
/// `schema_b`): `Missing` always sorts last, in front of and unaffected by `:desc`, and only a
/// comparison of two present values is reversed for it (Sorting under `list`, "Missing value |
/// Last, in both directions").
fn sort_compare(
    key: &SortKey,
    schema_a: &Resolved,
    a: &Document,
    schema_b: &Resolved,
    b: &Document,
) -> Ordering {
    let va = sort_value(&key.field, schema_a, a);
    let vb = sort_value(&key.field, schema_b, b);
    match (
        matches!(va, SortValue::Missing),
        matches!(vb, SortValue::Missing),
    ) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let ord = va.cmp_same_type(&vb);
            if key.desc { ord.reverse() } else { ord }
        }
    }
}

/// The tiebreak every `list` result falls back to, `--sort` given or not: key order (by code,
/// then numerically) for a coded document, path order otherwise. A document with a key compared
/// against one without falls back to comparing the key's text against the path's text, since the
/// design gives no rule for that mix and it is rare enough that any total order is defensible.
fn compare_identity(a: &Document, b: &Document) -> Ordering {
    match (&a.key, &b.key) {
        (Some(ka), Some(kb)) => key_sort_value(ka).cmp_same_type(&key_sort_value(kb)),
        (None, None) => a.path.cmp(&b.path),
        (Some(ka), None) => ka.as_str().cmp(b.path.as_str()),
        (None, Some(kb)) => a.path.as_str().cmp(kb.as_str()),
    }
}

/// The fields and body links of a document already read, for `refs --reverse`'s scan of every
/// other document of the project: `None` exactly when `frontmatter.parse` would fire for it (an
/// unclosed block, or a block that is not valid YAML), so such a document contributes nothing to
/// the reverse index, the same as it contributes no finding to `validate` (design: "no other
/// rule is evaluated for that file"). Unlike `parsed_fields` above (used where a caller already
/// knows `frontmatter.parse` did not fire, so its own collapsing of "no block" and "broken
/// block" into one `None` is safe), this tells a file with no block at all — a legitimate,
/// ref-less document — apart from one that is broken, since only the latter should also skip the
/// body scan below.
fn parsed_fields_and_body(
    text: &str,
    schema: &Resolved,
) -> Option<(Vec<(String, Value)>, BodyLinks)> {
    let block = frontmatter::block(text).ok()?;
    let fields = match block {
        None => Vec::new(),
        Some(block) => frontmatter::fields(block, schema).ok()?,
    };
    let body = links::scan(text).expect("frontmatter.parse already refused an unclosed block");
    Some((fields, body))
}

/// The written ref or refs a field's already-typed value holds: one for `ref`, each item of the
/// list for `ref[]`. Called only once `coerce::fits` has shown the value matches the field's
/// kind, so the other variants of `Value` never reach here.
fn ref_values(value: &Value) -> Vec<&str> {
    match value {
        Value::Text(text) => vec![text.as_str()],
        Value::List(items) => items.iter().map(String::as_str).collect(),
        Value::Number(_) | Value::Bool(_) | Value::Date(_) | Value::Datetime(_) => Vec::new(),
    }
}

/// The message for a ref that does not resolve under `refs.resolve`, naming why. A reason of
/// `ImportAbsent` never reaches this: `check_refs` reports it under `imports.absent` instead,
/// before `unresolved_ref_finding` would call `reason_message` (see `Project::
/// imports_absent_level`); the arm stays here so the match is exhaustive and correct if a future
/// caller ever reaches it.
fn reason_message(written: &str, reason: &refs::Reason) -> String {
    match reason {
        refs::Reason::NotFound => format!("the ref `{written}` does not resolve: not found"),
        refs::Reason::BadPrefix => {
            format!("the ref `{written}` does not resolve: its prefix names no namespace")
        }
        refs::Reason::ImportAbsent(absence) => {
            format!(
                "the ref `{written}` does not resolve: {}",
                absence.message()
            )
        }
    }
}

/// `refs::Reason` under the design's own `unresolved` ids.
fn reason_id(reason: &refs::Reason) -> &'static str {
    match reason {
        refs::Reason::NotFound => "not-found",
        refs::Reason::BadPrefix => "bad-prefix",
        refs::Reason::ImportAbsent(_) => "import-absent",
    }
}

/// `refs`'s `--field` filter, kept for `"$body"` on a body link and for a named field on a
/// frontmatter ref alike: `None` keeps everything.
fn filtered(refs: Vec<RefsReference>, field: Option<&str>) -> Vec<RefsReference> {
    match field {
        Some(field) => refs.into_iter().filter(|r| r.field == field).collect(),
        None => refs,
    }
}

/// `body.links`' message for a destination that does not resolve, matching the design's own
/// example (`"link target missing: ../x.md"`); a definition's finding also carries how many
/// places used it (`uses` is `Some` only there), since it is checked once regardless of use.
fn missing_target_message(written: &str, uses: Option<usize>) -> String {
    match uses {
        None => format!("link target missing: {written}"),
        Some(1) => format!("link target missing: {written} (used 1 time)"),
        Some(n) => format!("link target missing: {written} (used {n} times)"),
    }
}

/// A rule's own options, `global`'s then `collection`'s merged key by key: design line 618, "A
/// collection file merges key by key, so it states only what differs," read as holding inside
/// one rule's own setting and not only for which rule names a collection's `validation` states —
/// a collection that overrides only `level` keeps every option the global setting gave the same
/// rule, and an option a collection does state replaces only that option, never the whole set.
/// `level` itself already merges this way (`effective_level`); this is the same rule for options.
fn rule_options(
    rule: &str,
    global: &Rules,
    collection: &Rules,
) -> serde_json::Map<String, serde_json::Value> {
    let mut merged = global
        .get(rule)
        .map(|setting| setting.options.clone())
        .unwrap_or_default();
    if let Some(setting) = collection.get(rule) {
        merged.extend(setting.options.clone());
    }
    merged
}

fn bool_option(
    options: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: bool,
) -> bool {
    options
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

/// `body.links`' `ignore` option: globs of relative targets to skip, matched after
/// percent-decoding (design's `body.links` row). A value that is not a string, or missing
/// entirely, contributes nothing, since `config.rule-unknown` already refuses any other shape for
/// the option when the config loads.
fn ignore_globs(options: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    options
        .get("ignore")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `path` (relative to the project folder) matches an `ignore` glob such as
/// `assets/**`: the same template syntax `match` already reads (`Template::parse`), so `**`
/// stands for any number of whole path segments and `*` for any run of characters within one,
/// rather than a second glob syntax with its own rules.
fn ignore_matches(pattern: &Template, path: &str) -> bool {
    fn go(steps: &[Step], parts: &[&str]) -> bool {
        match steps.split_first() {
            None => parts.is_empty(),
            Some((Step::Name(segment), rest)) => matches!(
                parts.split_first(),
                Some((first, more)) if segment.matches(first) && go(rest, more)
            ),
            Some((Step::Folders, rest)) => (0..=parts.len()).any(|skip| go(rest, &parts[skip..])),
        }
    }
    let parts: Vec<&str> = path.split('/').collect();
    go(pattern.steps(), &parts)
}

/// The message a `collections.overlap` finding and a refused direct read of the same path share:
/// which collections matched it.
fn overlap_message(collections: &[String]) -> String {
    format!(
        "this file is matched by more than one collection: `{}`",
        collections.join("`, `")
    )
}

/// The text an ambiguous or missing key is reported by: the prefix the argument gave, if any,
/// put back in front of it.
fn printed_key(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(namespace) => format!("{namespace}:{key}"),
        None => key.to_owned(),
    }
}

/// Refuses a `--namespace`/`TYPDOC_NAMESPACE` value that named an import, for any `validate` run
/// — the whole-project scan, `--schemas`, and named arguments alike: none of the three ever
/// reads an import (a named argument is resolved by path or key against `self.resolve`, which
/// reads only `self.index`, regardless of `scope`; the whole-project scan reads `self.index.
/// iter()`; `--schemas` reads only findings gathered at load), so silently accepting the scope
/// would report on nothing named, in a shape (a report with `documents: 0`, an empty `paths`
/// entry, or a clean `findings: []`) that reads as "checked and clean" rather than "not checked"
/// — the same silent-incompleteness the audit invariant elsewhere in this design exists to
/// catch. `--namespace 'chief::*'` is for `list`, which does read imports (`Project::list_all`);
/// ownership rule 1 keeps `validate` from ever reading one, `project::` arguments included (see
/// `validate`'s own refusal of those, just above each of this function's three call sites: the
/// named-argument loop, and the two whole-project branches).
fn reject_import_scope(scope: &Scope) -> Result<(), Error> {
    if scope.imports.is_empty() {
        return Ok(());
    }
    let named: Vec<&str> = scope
        .imports
        .iter()
        .map(|(alias, _)| alias.as_str())
        .collect();
    Err(Error::BadArgument(format!(
        "--namespace named an import ({}), and validate never reads one: a file is validated only by the project that owns it",
        named.join(", ")
    )))
}

/// The scope a `project::` argument's own `namespace:` prefix chooses inside `imported`: the
/// named namespace when there is a prefix; `imported`'s one namespace when it has exactly one
/// and there is none; otherwise bad arguments (design: "a ref into a project with several
/// namespaces must name one" — `chief::WF-5` is refused there whatever the key, unconditionally,
/// not only when it happens to be ambiguous; ticket 10's report reads a ref the same way, and an
/// argument follows it for the same reason).
fn imported_scope(imported: &Project, namespace: Option<&str>) -> Result<Scope, Error> {
    if let Some(name) = namespace {
        let index = imported.namespace_index(name).ok_or_else(|| {
            let known: Vec<&str> = imported
                .config
                .namespaces
                .iter()
                .map(|n| n.name.as_str())
                .collect();
            Error::BadArgument(format!(
                "`{name}` is not a namespace of the imported project, which has: {}",
                known.join(", ")
            ))
        })?;
        return Ok(Scope {
            source: Source::Prefix,
            namespaces: vec![imported.config.namespaces[index].name.clone()],
            imports: Vec::new(),
        });
    }
    if imported.config.namespaces.len() == 1 {
        return Ok(Scope {
            source: Source::Everything,
            namespaces: vec![imported.config.namespaces[0].name.clone()],
            imports: Vec::new(),
        });
    }
    Err(Error::BadArgument(
        "the imported project has more than one namespace: name one, e.g. `alias::namespace:key`"
            .to_owned(),
    ))
}

/// The name (`path`, `namespace`, `key`, `project`) of a document at `path` in the project whose
/// `namespaces` and `index` are given: looked up in the index when it is one, which is the only
/// place a coded document's `key` comes from. A path that resolved but is outside every
/// collection (`target: "*"` accepts one, e.g. a README) has no entry of its own; its namespace
/// is the one whose folder is the longest prefix of `path`, or the namespace with no folder
/// (`default`) when none matches, since namespaces never nest (config rule) and every project has
/// exactly one namespace with an empty folder. `project` is the alias `path` was reached through,
/// `None` for a document of the project doing the reaching; shared between `Project::ref_name_of`
/// (`project: None`, this project) and the import-resolving code in `refs.rs` (`project: Some
/// (alias)`), so the two read a document's name the same way whichever side of an import it is.
fn ref_name_in(
    namespaces: &[crate::config::Namespace],
    index: &Index,
    project: Option<&str>,
    path: &str,
) -> RefName {
    let project = project.map(str::to_owned);
    if let Some(entry) = index.get(path) {
        return RefName {
            path: path.to_owned(),
            namespace: namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            project,
        };
    }
    let namespace = namespaces
        .iter()
        .filter(|space| {
            !space.folder.is_empty()
                && (path == space.folder || path.starts_with(&format!("{}/", space.folder)))
        })
        .max_by_key(|space| space.folder.len())
        .or_else(|| namespaces.iter().find(|space| space.folder.is_empty()))
        .expect("every project has a namespace with an empty folder when none matches by prefix");
    RefName {
        path: path.to_owned(),
        namespace: namespace.name.clone(),
        key: None,
        project,
    }
}

/// One configured import, resolved: `${NAME}` substituted in `raw` (an unset or empty variable
/// makes it `Absence::Variable`, never an empty-string substitution — design, "Environment
/// variables in import paths"); the result joined against `root` when it is not already
/// absolute; and, when a `.typdoc/config.json` really sits there, the project loaded with its
/// own imports left unread (one level only). A location that substitutes cleanly but has no
/// project of its own is `Absence::NoProject`, the same "absent on this machine" outcome as an
/// unset variable — both are the design's `imports.absent`, never a fatal error, since the whole
/// point of a machine-specific import is that it may not be there yet. A location that does have
/// a project, but one whose own config cannot be loaded, is treated as a real, fixable
/// misconfiguration and propagated as an ordinary error: unlike an import simply not being set
/// up yet, a broken config at a real location will not fix itself by installing more machines,
/// and folding it into `imports.absent` would hide a mistake the design gives no way to catch.
fn resolve_import(root: &Path, raw: &str, env: &dyn Env) -> Result<ImportState, Error> {
    let substituted = match crate::imports::substitute(raw, env) {
        Ok(text) => text,
        Err(variable) => {
            return Ok(ImportState::Absent(crate::imports::Absence::Variable(
                variable,
            )));
        }
    };
    let path = {
        let candidate = Path::new(&substituted);
        if candidate.is_absolute() {
            candidate.to_owned()
        } else {
            root.join(candidate)
        }
    };
    if !config_file(&path).is_file() {
        return Ok(ImportState::Absent(crate::imports::Absence::NoProject(
            path,
        )));
    }
    let imported = Project::load_inner(&path, env, false)?;
    Ok(ImportState::Loaded(Box::new(imported)))
}

/// The template of a collection as it is written, before it is bound to a schema. A template
/// that cannot be read is a config error.
fn read_template(collection: &Collection, report: &mut Report) -> Option<Template> {
    match Template::parse(&collection.pattern) {
        Ok(template) => Some(template),
        Err(message) => {
            report.add("config.match-template", &collection.path, message);
            None
        }
    }
}

/// `schema.valid`'s findings for a schema name or code shared by more than one collection's own
/// schema file: two different files, since `config.coded-schema-shared` already covers two
/// collections naming the exact same coded schema file. One finding per extra file, at the file
/// that comes later in collection order (collections are already sorted by name), naming the
/// first file that used it, the same convention `config.coded-schema-shared` uses.
fn duplicate_schema_findings(loaded: &[Loaded]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut first_by_name: BTreeMap<&str, &str> = BTreeMap::new();
    let mut first_by_code: BTreeMap<&str, &str> = BTreeMap::new();
    for item in loaded {
        match first_by_name.get(item.schema.name.as_str()) {
            Some(&earlier) if earlier != item.schema_path => {
                findings.push(validate::schema_finding(
                    &item.schema_path,
                    None,
                    format!(
                        "the schema name `{}` is also used by `{earlier}`",
                        item.schema.name
                    ),
                ));
            }
            _ => {
                first_by_name.insert(&item.schema.name, &item.schema_path);
            }
        }
        if let Some(code) = &item.schema.code {
            match first_by_code.get(code.as_str()) {
                Some(&earlier) if earlier != item.schema_path => {
                    findings.push(validate::schema_finding(
                        &item.schema_path,
                        None,
                        format!("the schema code `{code}` is also used by `{earlier}`"),
                    ));
                }
                _ => {
                    first_by_code.insert(code, &item.schema_path);
                }
            }
        }
    }
    findings
}
