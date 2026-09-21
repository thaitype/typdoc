//! Ref forms in frontmatter, resolved through the index of names as they are on disk, and the
//! rules that check them: `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved` (one
//! ref at a time, from `resolve_one`) and `refs.acyclic` (a whole-project graph, built from the
//! same `resolve_one`, and turned into findings by `cyclic_findings`). `names.shadowed` lives in
//! `project.rs`, since it is a fact about the config and never reads a document.
//!
//! A ref is not an argument (ticket 7's report): a bare form always means the document's own
//! namespace, whatever the working directory, and a prefix that names neither a sibling
//! namespace nor an import is `bad-prefix`, never a relative path. The import form (`name::`)
//! resolves into the alias's own project (`Ctx::imports`): an alias this project does not
//! configure is `bad-prefix`; one that is absent on this machine is `import-absent`; one that is
//! present is resolved inside it, by the same rules a document of that project would use.
//!
//! `classify` reads no file and touches no index: it decides which of the design's forms a
//! written ref is and, for a path form, the base it is joined against, all from the string and
//! the project's namespaces alone. `resolve_one` is the only part that reads the index and the
//! disk (`resolve_key`, `resolve_path`), so the form rules are tested here with no filesystem,
//! and the disk-touching part is tested through `validate` in the binary's own tests, the same
//! way the rest of this crate's index-reading behaviour already is.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::argument::looks_like_key;
use crate::config::{Namespace, RefBase};
use crate::imports::Absence;
use crate::index::Index;
use crate::project::ImportState;
use crate::schema::{Target, normalize};

/// Why a ref did not resolve, under the design's own `unresolved` ids. `ImportAbsent` carries why
/// the import itself is absent, so a caller can build `imports.absent`'s message naming the
/// variable, the same way the design's own example does ("TYPMEM_DIR is not set").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    NotFound,
    BadPrefix,
    ImportAbsent(Absence),
}

/// Whether a ref was written as a key or as a path: `refs.codedByPath` cares only about the
/// written form, never about which one a reader would have preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Via {
    Key,
    Path,
}

/// A ref that resolved: the path of the document or file it names, the collection it belongs to
/// (`None` for a file outside every collection, reachable only through `target: "*"`), the form
/// it was written in, and, when it landed in an imported project rather than this one, the alias
/// it was reached through (`collection` then indexes that project's own collections, never this
/// one's — a caller that reads `collection` to look up a schema must first check `project`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub path: String,
    pub collection: Option<usize>,
    pub via: Via,
    pub project: Option<String>,
}

/// What one written ref resolves to.
pub(crate) type Outcome = Result<Resolved, Reason>;

/// The name and code of a schema a collection uses, by the collection's position in the project
/// (`Index::Entry::collection`), so `refs.target` and `refs.codedByPath` can read them without
/// depending on `project.rs`'s own, private `Loaded` type. `Copy`: a caller reads one out of a
/// project's list (this project's own, or an imported project's) and holds it alone, never the
/// list it came from.
#[derive(Clone, Copy)]
pub(crate) struct SchemaInfo<'a> {
    pub name: &'a str,
    pub code: Option<&'a str>,
}

/// What a ref is resolved against: the namespace and collection of the document that holds it,
/// and the index and project root every ref reads through. Built fresh per document.
pub(crate) struct Ctx<'a> {
    pub doc_namespace: usize,
    pub doc_path: &'a str,
    pub ref_base: RefBase,
    pub namespaces: &'a [Namespace],
    pub codes: &'a BTreeSet<String>,
    pub index: &'a Index,
    pub root: &'a Path,
    /// This project's own `imports`, resolved once at load (`Project::imports`): every alias not
    /// a key here names neither a sibling namespace nor an import, and is `bad-prefix`.
    pub imports: &'a BTreeMap<String, ImportState>,
}

/// What a written ref's form decides, before anything is looked up: a key in one namespace, or a
/// path already joined against its base (the document's own folder or namespace under `refBase`
/// for the unprefixed form, always the named namespace's folder for a sibling prefix).
enum Form {
    Key {
        namespace: usize,
        key: String,
    },
    Path {
        base: String,
        rest: String,
    },
    /// `name::rest`: an import prefix, not yet looked up against `Ctx::imports` (`classify`
    /// itself reads no index, and telling `bad-prefix` from `import-absent` from a real
    /// resolution needs it).
    Import {
        alias: String,
        rest: String,
    },
    BadPrefix,
}

/// What a body link's destination is, before anything is looked up: a path to resolve (in this
/// project, or, for `Import`, in the alias's own project — its path is already joined against
/// that project's folder, never this one's, so the caller resolves it against the imported
/// project's own index and root), `bad-prefix`, `import-absent`, or skipped entirely — an
/// ordinary URL scheme (`https:`, `mailto:` and so on), which the design says is "always
/// skipped; no configuration is needed", never a finding of any kind.
pub(crate) enum BodyDestination {
    Path(String),
    Import { alias: String, path: String },
    BadPrefix,
    ImportAbsent(Absence),
    Skip,
}

/// Resolves one ref as written in frontmatter, through the forms the design's Refs table gives:
/// bare key, sibling prefix, relative path with `refBase`, and the import form.
pub(crate) fn resolve_one(written: &str, ctx: &Ctx) -> Outcome {
    match classify(
        written,
        ctx.doc_namespace,
        ctx.doc_path,
        ctx.ref_base,
        ctx.namespaces,
        ctx.codes,
    ) {
        Form::Key { namespace, key } => resolve_key(namespace, &key, ctx.index),
        Form::Path { base, rest } => resolve_path(&join(&base, &rest), ctx.index, ctx.root),
        Form::Import { alias, rest } => resolve_into_import(&alias, &rest, ctx.imports),
        Form::BadPrefix => Err(Reason::BadPrefix),
    }
}

/// `name::rest`: `alias` looked up against `imports` (`Project::imports`, resolved once at
/// load). An alias this project does not configure is `bad-prefix`, the same reading a namespace
/// prefix naming no sibling already gets; one absent on this machine is `import-absent`
/// (`imports.absent`'s reason); one present is resolved inside it by `resolve_into_project`, and
/// the result is tagged with the alias so the caller can name the document correctly (`project`
/// in its `path`, `namespace` and `key`).
fn resolve_into_import(
    alias: &str,
    rest: &str,
    imports: &BTreeMap<String, ImportState>,
) -> Outcome {
    match imports.get(alias) {
        None => Err(Reason::BadPrefix),
        Some(ImportState::Absent(absence)) => Err(Reason::ImportAbsent(absence.clone())),
        Some(ImportState::Loaded(imported)) => resolve_into_project(
            rest,
            imported.namespaces(),
            imported.index_ref(),
            imported.root_ref(),
            &imported.codes(),
        )
        .map(|resolved| Resolved {
            project: Some(alias.to_owned()),
            ..resolved
        }),
    }
}

/// `rest` after an import prefix, resolved inside the project it names: `namespace:rest` is a
/// key or a path in that project's namespace `namespace`, or `bad-prefix` when it has none by
/// that name; a bare, key-shaped `rest` whose code the imported project has is a key in its one
/// namespace, or `bad-prefix` when it has more than one (design: "a ref into a project with
/// several namespaces must name one" — an unconditional syntax requirement, not "ambiguous only
/// when the key happens to collide"); anything else is a path relative to the imported project's
/// own folder, never to the referring document's (imports name a document by `project::path`,
/// design's Arguments that name a document table, and a ref's path form follows the same rule
/// Body links already gives a sibling-prefixed path: from the named project's folder, not
/// `refBase`, which is a property of the *referring* document's own collection and has no
/// meaning once the ref has crossed into another project entirely). A further `::` inside `rest`
/// is `bad-prefix`: imports of imports are ignored.
fn resolve_into_project(
    rest: &str,
    namespaces: &[Namespace],
    index: &Index,
    root: &Path,
    codes: &BTreeSet<String>,
) -> Outcome {
    if rest.contains("::") {
        return Err(Reason::BadPrefix);
    }
    if let Some((prefix, sub)) = rest.split_once(':') {
        let namespace = namespace_named(namespaces, prefix).ok_or(Reason::BadPrefix)?;
        return if looks_like_key(sub) {
            resolve_key(namespace, sub, index)
        } else {
            resolve_path(&join(&namespaces[namespace].folder, sub), index, root)
        };
    }
    if looks_like_key(rest) && codes.contains(code_of(rest)) {
        if namespaces.len() != 1 {
            return Err(Reason::BadPrefix);
        }
        return resolve_key(0, rest, index);
    }
    resolve_path(rest, index, root)
}

/// What a body link's destination (`target`, already percent-decoded, with any `#anchor` already
/// split off by `links::scan`) is, before it is looked up: the same prefix rules a frontmatter
/// ref uses (a leading `./` or `../` escapes a colon that is part of the path; `::` is always
/// `bad-prefix`, the import form this story does not read; a single `name:` is the sibling
/// namespace `name` when one exists), except that a body link is always a path and never a key
/// (design.md's Body links paragraph: "after the prefix comes a path, never a key" — read here as
/// holding for the unprefixed form too, since nothing in Body links gives a body link a bare-key
/// reading the way a frontmatter `ref` field's Refs table does), and a single colon whose prefix
/// names no sibling is an ordinary URL scheme rather than `bad-prefix`: body text carries real
/// URLs a typed frontmatter field never does, and the design's own words for this case are "a URL
/// scheme... that is not a namespace name or an import alias are always skipped".
pub(crate) fn classify_body(target: &str, ctx: &Ctx) -> BodyDestination {
    if target.starts_with("./") || target.starts_with("../") {
        let base = base_of(
            ctx.ref_base,
            ctx.doc_path,
            ctx.doc_namespace,
            ctx.namespaces,
        );
        return BodyDestination::Path(join(&base, target));
    }
    if let Some((alias, rest)) = target.split_once("::") {
        return match ctx.imports.get(alias) {
            None => BodyDestination::BadPrefix,
            Some(ImportState::Absent(absence)) => BodyDestination::ImportAbsent(absence.clone()),
            Some(ImportState::Loaded(imported)) => {
                // Mirrors the sibling-prefix branch below: an explicit namespace before the path
                // is accepted, though a path is self-qualifying either way, since the design
                // gives a sibling-prefixed path this same form and gives no reason an import
                // should differ.
                let (base, rest) = match rest.split_once(':') {
                    Some((namespace, sub)) => {
                        match namespace_named(imported.namespaces(), namespace) {
                            Some(index) => (imported.namespaces()[index].folder.clone(), sub),
                            None => return BodyDestination::BadPrefix,
                        }
                    }
                    None => (String::new(), rest),
                };
                BodyDestination::Import {
                    alias: alias.to_owned(),
                    path: join(&base, rest),
                }
            }
        };
    }
    if let Some((prefix, rest)) = target.split_once(':') {
        return match namespace_named(ctx.namespaces, prefix) {
            Some(namespace) => BodyDestination::Path(join(&ctx.namespaces[namespace].folder, rest)),
            None => BodyDestination::Skip,
        };
    }
    let base = base_of(
        ctx.ref_base,
        ctx.doc_path,
        ctx.doc_namespace,
        ctx.namespaces,
    );
    BodyDestination::Path(join(&base, target))
}

/// A path that really contains a colon is written with a leading `./` or `../`; that escape is
/// read first and skips prefix detection entirely, so a literal file name is never misread as a
/// namespace. Otherwise: `name::rest` is an import prefix (looked up against `Ctx::imports` by
/// the caller, not here); `name:rest` is a key or a path in the sibling namespace `name`, or
/// `bad-prefix` when no sibling has that name; anything else is a bare key in the document's own
/// namespace when it has the shape of one and the code exists somewhere in the project, and a
/// relative path otherwise.
fn classify(
    written: &str,
    doc_namespace: usize,
    doc_path: &str,
    ref_base: RefBase,
    namespaces: &[Namespace],
    codes: &BTreeSet<String>,
) -> Form {
    if written.starts_with("./") || written.starts_with("../") {
        return Form::Path {
            base: base_of(ref_base, doc_path, doc_namespace, namespaces),
            rest: written.to_owned(),
        };
    }
    if let Some((alias, rest)) = written.split_once("::") {
        return Form::Import {
            alias: alias.to_owned(),
            rest: rest.to_owned(),
        };
    }
    if let Some((prefix, rest)) = written.split_once(':') {
        return match namespace_named(namespaces, prefix) {
            Some(namespace) if looks_like_key(rest) => Form::Key {
                namespace,
                key: rest.to_owned(),
            },
            Some(namespace) => Form::Path {
                base: namespaces[namespace].folder.clone(),
                rest: rest.to_owned(),
            },
            None => Form::BadPrefix,
        };
    }
    if looks_like_key(written) && codes.contains(code_of(written)) {
        return Form::Key {
            namespace: doc_namespace,
            key: written.to_owned(),
        };
    }
    Form::Path {
        base: base_of(ref_base, doc_path, doc_namespace, namespaces),
        rest: written.to_owned(),
    }
}

/// The base the unprefixed, "anything else" form is joined against: the document's own folder
/// (`refBase: file`) or its namespace's folder (`refBase: namespace`).
fn base_of(
    ref_base: RefBase,
    doc_path: &str,
    doc_namespace: usize,
    namespaces: &[Namespace],
) -> String {
    match ref_base {
        RefBase::File => folder_of(doc_path),
        RefBase::Namespace => namespaces[doc_namespace].folder.clone(),
    }
}

fn namespace_named(namespaces: &[Namespace], name: &str) -> Option<usize> {
    namespaces
        .iter()
        .position(|namespace| namespace.name == name)
}

/// The part of a key before its dash: `WF` in `WF-3`. Only called once `looks_like_key` has
/// already shown a dash is there.
#[expect(
    clippy::expect_used,
    reason = "each of the three calls first checks that the text has the key shape, which needs a \
              dash: `resolve_into_project` and `classify` in this file test `looks_like_key(..)` on the \
              same string in the `if` that holds the call, and `Project::mention_missing` passes the \
              part after the last `:` of a `Mention.written`, which `links::mention_shape` accepts \
              only when `looks_like_key_shape` holds for that same part"
)]
pub(crate) fn code_of(key: &str) -> &str {
    key.split_once('-')
        .map(|(code, _)| code)
        .expect("looks_like_key already found a dash")
}

/// The document's own namespace's key index, for a bare key, or the named namespace's for a
/// sibling-prefixed one.
fn resolve_key(namespace: usize, key: &str, index: &Index) -> Outcome {
    let path = index.key(namespace, key).ok_or(Reason::NotFound)?;
    #[expect(
        clippy::expect_used,
        reason = "`path` comes from `index.key(..)` on the same `index`; `Index::build` binds a key \
                  to a path in the same step that inserts the path's entry, removes both together \
                  for a path more than one collection matches, and `Index` has no other mutator, so \
                  a path in a key group has an entry"
    )]
    let entry = index
        .get(path)
        .expect("a key in the key index always has a matching entry");
    Ok(Resolved {
        path: path.to_owned(),
        collection: Some(entry.collection),
        via: Via::Key,
        project: None,
    })
}

/// The folder a project-relative path sits in, or the project folder itself for a path with no
/// folder of its own.
fn folder_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => folder.to_owned(),
        None => String::new(),
    }
}

/// `base` and `rest` joined and normalized, the same way a schema reference is joined against
/// the folder that names it (`schema::normalize`).
fn join(base: &str, rest: &str) -> String {
    if base.is_empty() {
        normalize(rest)
    } else {
        normalize(&format!("{base}/{rest}"))
    }
}

/// A path already indexed as a document resolves with its collection known; a path that exists
/// on disk but matches no collection resolves too, with no collection (only `target: "*"`
/// accepts it: design.md's Target names, "`*` also accepts files outside any collection, such as
/// a README"); anything else is `not-found`.
pub(crate) fn resolve_path(path: &str, index: &Index, root: &Path) -> Outcome {
    if let Some(entry) = index.get(path) {
        return Ok(Resolved {
            path: path.to_owned(),
            collection: Some(entry.collection),
            via: Via::Path,
            project: None,
        });
    }
    if root.join(path).is_file() {
        return Ok(Resolved {
            path: path.to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
        });
    }
    Err(Reason::NotFound)
}

/// Whether a resolved ref's target is one `target` allows. `None` (the option was not written)
/// reads as `Target::Any`, the design's stated default. `Target::Other` is a value `schema.valid`
/// already reports as invalid on its own; treated here as no restriction, so the same fault is
/// not reported twice under two different rules.
///
/// `info` is the schema the ref actually resolved to, already read by the caller for whichever
/// project `resolved.collection` indexes (this one, or, once a ref has crossed an import, the
/// alias's own — `Project::schema_info_of`); `None` when the target has no schema at all (a
/// file outside every collection, reachable only through `target: "*"`, which never reaches the
/// `Target::Schemas` arm below since a written target list never allows it either way).
///
/// A bare name in `target` (design, Target names: "a bare name... means a schema in this
/// project") never allows a ref that crossed into an import, and a qualified name
/// (`"alias::name"`) never allows one that stayed inside this project: `resolved.project` picks
/// which reading of `target`'s own list applies, so the two forms are never compared against
/// the wrong kind of ref.
pub(crate) fn target_allowed(
    target: Option<&Target>,
    resolved: &Resolved,
    info: Option<&SchemaInfo>,
) -> bool {
    match target.unwrap_or(&Target::Any) {
        Target::Any | Target::Other(_) => true,
        Target::Schemas(names) => match info {
            None => false,
            Some(info) => match &resolved.project {
                None => names.iter().any(|name| name == info.name),
                Some(alias) => names
                    .iter()
                    .any(|name| *name == format!("{alias}::{}", info.name)),
            },
        },
    }
}

/// The nodes that lie on a cycle, from a project-wide list of directed edges (source path,
/// target path) collected for one `acyclic` field: depth-first search with the classic three
/// colours, where a back edge to a node still on the stack closes a cycle and every node from
/// there to the top of the stack is part of it. `refs.acyclic`'s findings are one per node this
/// returns (its own outgoing edge is what is wrong with it), never one per cycle, since a node
/// can sit on more than one.
pub(crate) fn cyclic_nodes(edges: &[(String, String)]) -> BTreeSet<String> {
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for (source, target) in edges {
        outgoing
            .entry(source.as_str())
            .or_default()
            .push(target.as_str());
        nodes.insert(source.as_str());
        nodes.insert(target.as_str());
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    fn visit<'a>(
        node: &'a str,
        outgoing: &BTreeMap<&'a str, Vec<&'a str>>,
        color: &mut BTreeMap<&'a str, Color>,
        stack: &mut Vec<&'a str>,
        cyclic: &mut BTreeSet<String>,
    ) {
        color.insert(node, Color::Gray);
        stack.push(node);
        if let Some(children) = outgoing.get(node) {
            for &child in children {
                match color.get(child).copied().unwrap_or(Color::White) {
                    Color::White => visit(child, outgoing, color, stack, cyclic),
                    Color::Gray => {
                        if let Some(at) = stack.iter().position(|n| *n == child) {
                            for n in &stack[at..] {
                                cyclic.insert((*n).to_owned());
                            }
                        }
                    }
                    Color::Black => {}
                }
            }
        }
        stack.pop();
        color.insert(node, Color::Black);
    }

    let mut color: BTreeMap<&str, Color> = BTreeMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut cyclic: BTreeSet<String> = BTreeSet::new();
    for &node in &nodes {
        if !matches!(color.get(node), Some(Color::Black)) {
            visit(node, &outgoing, &mut color, &mut stack, &mut cyclic);
        }
    }
    cyclic
}

#[cfg(test)]
mod tests {
    use super::*;

    fn namespaces() -> Vec<Namespace> {
        vec![
            Namespace {
                name: "default".to_owned(),
                folder: String::new(),
            },
            Namespace {
                name: "story-2".to_owned(),
                folder: "story-2".to_owned(),
            },
        ]
    }

    fn classify_default(written: &str, ref_base: RefBase, codes: &BTreeSet<String>) -> Form {
        classify(
            written,
            0,
            "tickets/WF-1.md",
            ref_base,
            &namespaces(),
            codes,
        )
    }

    /// `classify_body` on a document `tickets/WF-1.md` of the two-namespace project `namespaces`
    /// gives: `classify_body` itself never reads `codes` or `index` (a body link is never a bare
    /// key), so an empty index stands in.
    fn classify_body_default(target: &str, ref_base: RefBase) -> BodyDestination {
        let namespaces = namespaces();
        let codes = BTreeSet::new();
        let index = Index::default();
        let imports = BTreeMap::new();
        let ctx = Ctx {
            doc_namespace: 0,
            doc_path: "tickets/WF-1.md",
            ref_base,
            namespaces: &namespaces,
            codes: &codes,
            index: &index,
            root: Path::new("."),
            imports: &imports,
        };
        classify_body(target, &ctx)
    }

    fn code_set(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn a_bare_key_shaped_value_whose_code_exists_is_a_key_in_the_documents_own_namespace() {
        let form = classify_default("WF-1", RefBase::File, &code_set(&["WF"]));

        assert!(matches!(form, Form::Key { namespace: 0, key } if key == "WF-1"));
    }

    #[test]
    fn a_key_shaped_value_whose_code_is_not_in_the_project_is_read_as_a_relative_path() {
        let form = classify_default("WF-1", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::Path { rest, .. } if rest == "WF-1"));
    }

    #[test]
    fn a_double_colon_is_an_import_form_never_a_relative_path() {
        // `classify` itself reads no index and cannot yet know whether `chief` is configured;
        // that is `resolve_one`'s job once it has `Ctx::imports` in hand (see the module doc and
        // `resolve_into_import`'s own tests through `Project`, in `crates/typdoc/tests`).
        let form = classify_default("chief::WF-5", RefBase::File, &code_set(&[]));

        assert!(
            matches!(&form, Form::Import { alias, rest } if alias == "chief" && rest == "WF-5")
        );
    }

    #[test]
    fn a_double_colon_is_an_import_form_whatever_the_project_has() {
        // The ticket's own example: `chief::WF-5` in a project with several namespaces. The
        // alias and the rest after it are read the same way whether or not this project happens
        // to have a namespace of that name, since `::` and `:` are never confused with each
        // other (design: the two syntaxes never fall back to each other).
        let form = classify(
            "chief::WF-5",
            0,
            "a.md",
            RefBase::File,
            &namespaces(),
            &code_set(&[]),
        );

        assert!(
            matches!(&form, Form::Import { alias, rest } if alias == "chief" && rest == "WF-5")
        );
    }

    #[test]
    fn a_single_colon_naming_no_sibling_namespace_is_bad_prefix() {
        let form = classify_default("nosuch:WF-5", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::BadPrefix));
    }

    #[test]
    fn a_single_colon_naming_a_sibling_with_a_key_shaped_rest_is_a_key_in_that_namespace() {
        let form = classify_default("story-2:WF-5", RefBase::File, &code_set(&[]));

        // No "code exists in the project" gate here, unlike the bare form (design.md's table
        // gives the sibling key row only "story-2 is a namespace" as its condition): a key shape
        // is enough once the namespace is real, even when the code is not used anywhere.
        assert!(matches!(form, Form::Key { namespace: 1, key } if key == "WF-5"));
    }

    #[test]
    fn a_single_colon_naming_a_sibling_with_a_path_rest_is_a_path_from_that_namespaces_folder() {
        let form = classify_default("story-2:notes/x.md", RefBase::Namespace, &code_set(&[]));

        // `RefBase::Namespace` is passed on purpose: the sibling path form always reads from the
        // named namespace's own folder, never from `refBase`, which applies only to the
        // unprefixed form.
        assert!(
            matches!(form, Form::Path { base, rest } if base == "story-2" && rest == "notes/x.md")
        );
    }

    #[test]
    fn a_leading_dot_slash_escapes_a_colon_that_is_really_part_of_the_path() {
        let form = classify_default("./weird:name.md", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::Path { rest, .. } if rest == "./weird:name.md"));
    }

    #[test]
    fn a_body_link_with_no_prefix_is_a_path_never_a_key_even_when_key_shaped() {
        // Unlike a frontmatter ref, a body link never reads the "bare key" row: design.md's Body
        // links paragraph gives it no bare-key form at all, only the forms a Markdown link
        // destination can take, all of them paths.
        let form = classify_body_default("WF-1", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "tickets/WF-1"));
    }

    #[test]
    fn a_body_link_sibling_prefix_is_a_path_from_that_namespaces_folder_never_a_key() {
        let form = classify_body_default("story-2:WF-5", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "story-2/WF-5"));
    }

    #[test]
    fn a_body_link_double_colon_is_bad_prefix_the_import_form_this_story_does_not_read() {
        let form = classify_body_default("memory::precedents/x.md", RefBase::File);

        assert!(matches!(form, BodyDestination::BadPrefix));
    }

    #[test]
    fn a_body_link_single_colon_naming_no_sibling_is_skipped_as_a_url_scheme() {
        for target in ["https://example.com/a.md", "mailto:a@b.com", "file:///a.md"] {
            let form = classify_body_default(target, RefBase::File);

            assert!(matches!(form, BodyDestination::Skip), "{target}");
        }
    }

    #[test]
    fn a_body_link_unprefixed_path_joins_against_the_documents_folder_or_its_namespace() {
        let file = classify_body_default("x.md", RefBase::File);
        let namespace = classify_body_default("x.md", RefBase::Namespace);

        assert!(matches!(file, BodyDestination::Path(path) if path == "tickets/x.md"));
        assert!(matches!(namespace, BodyDestination::Path(path) if path == "x.md"));
    }

    #[test]
    fn a_body_link_leading_dot_slash_escapes_a_colon_that_is_really_part_of_the_path() {
        let form = classify_body_default("./weird:name.md", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "tickets/weird:name.md"));
    }

    #[test]
    fn the_unprefixed_form_is_joined_against_the_documents_folder_or_its_namespace_by_ref_base() {
        let file = classify_default("x.md", RefBase::File, &code_set(&[]));
        let namespace = classify_default("x.md", RefBase::Namespace, &code_set(&[]));

        assert!(matches!(file, Form::Path { base, .. } if base == "tickets"));
        assert!(matches!(namespace, Form::Path { base, .. } if base.is_empty()));
    }

    #[test]
    fn folder_of_a_top_level_path_is_the_project_folder() {
        assert_eq!(folder_of("a.md"), "");
        assert_eq!(folder_of("tickets/a.md"), "tickets");
    }

    #[test]
    fn join_normalizes_dots_the_same_way_a_schema_reference_does() {
        assert_eq!(join("tickets", "./a.md"), "tickets/a.md");
        assert_eq!(join("tickets", "../a.md"), "a.md");
        assert_eq!(join("", "a.md"), "a.md");
    }

    #[test]
    fn target_any_or_no_target_at_all_allows_any_resolved_ref() {
        let file = Resolved {
            path: "README.md".to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
        };

        assert!(target_allowed(None, &file, None));
        assert!(target_allowed(Some(&Target::Any), &file, None));
    }

    #[test]
    fn a_target_list_of_schema_names_refuses_a_file_with_no_schema() {
        let file = Resolved {
            path: "README.md".to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
        };

        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["wayfinder".to_owned()])),
            &file,
            None
        ));
    }

    fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(s, t)| (s.to_string(), t.to_string()))
            .collect()
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_two_node_cycle_reports_both_nodes() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "a")]));

        assert_eq!(found, set(&["a", "b"]));
    }

    #[test]
    fn a_self_loop_reports_its_one_node() {
        let found = cyclic_nodes(&edges(&[("a", "a")]));

        assert_eq!(found, set(&["a"]));
    }

    #[test]
    fn a_chain_with_no_cycle_reports_nothing() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "c")]));

        assert_eq!(found, BTreeSet::new());
    }

    #[test]
    fn a_cycle_with_a_tail_does_not_report_the_tail() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "a"), ("b", "c")]));

        assert_eq!(
            found,
            set(&["a", "b"]),
            "c only receives an edge, it starts none"
        );
    }

    #[test]
    fn a_target_list_of_schema_names_allows_exactly_the_named_schemas() {
        let info = SchemaInfo {
            name: "wayfinder",
            code: Some("WF"),
        };
        let matching = Resolved {
            path: "tickets/WF-1.md".to_owned(),
            collection: Some(0),
            via: Via::Key,
            project: None,
        };

        assert!(target_allowed(
            Some(&Target::Schemas(vec!["wayfinder".to_owned()])),
            &matching,
            Some(&info)
        ));
        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["other".to_owned()])),
            &matching,
            Some(&info)
        ));
    }

    #[test]
    fn a_bare_name_in_target_never_allows_a_ref_that_crossed_an_import() {
        let info = SchemaInfo {
            name: "learning",
            code: None,
        };
        let crossed = Resolved {
            path: "precedents/x.md".to_owned(),
            collection: Some(0),
            via: Via::Path,
            project: Some("memory".to_owned()),
        };

        assert!(
            !target_allowed(
                Some(&Target::Schemas(vec!["learning".to_owned()])),
                &crossed,
                Some(&info)
            ),
            "a bare name means a schema of this project (design, Target names), never one \
             reached through an import"
        );
    }

    #[test]
    fn a_qualified_name_in_target_allows_exactly_the_schema_it_names_in_that_import() {
        let info = SchemaInfo {
            name: "learning",
            code: None,
        };
        let crossed = Resolved {
            path: "precedents/x.md".to_owned(),
            collection: Some(0),
            via: Via::Path,
            project: Some("memory".to_owned()),
        };

        assert!(target_allowed(
            Some(&Target::Schemas(vec!["memory::learning".to_owned()])),
            &crossed,
            Some(&info)
        ));
        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["memory::precedent".to_owned()])),
            &crossed,
            Some(&info)
        ));
        assert!(
            !target_allowed(
                Some(&Target::Schemas(vec!["other::learning".to_owned()])),
                &crossed,
                Some(&info)
            ),
            "the qualified name has to name the alias the ref actually crossed, not just any \
             import"
        );
    }
}
