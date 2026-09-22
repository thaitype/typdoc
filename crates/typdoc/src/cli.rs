use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clap::error::ErrorKind as ClapKind;
use clap::{Parser, Subcommand};
use serde_json::value::RawValue;
use serde_json::{Map, Value as Json, json};
use typdoc_core::{
    Argument, AuditReport, Condition, Deps, Document, DocumentArg, Env, Error, ErrorKind, FieldRef,
    Finding, ListFilter, ListResult, NewTarget, Project, RefField, RefOutcome, RefsDirection,
    RefsReference, RefsReport, Scope, SetOp, Severity, SortKey, Source, Toc, ValidateReport,
    ValidateScope, Value, discover, discover_for, parse_field, parse_query, resolve_on_disk,
};

#[derive(Parser)]
#[command(name = "typdoc", version)]
pub(crate) struct Cli {
    /// The namespaces to read: names or globs, separated by `,`
    #[arg(long, global = true, value_name = "LIST")]
    namespace: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a document; allocate a key for a coded schema
    New {
        /// The code of a coded schema (`[A-Z][A-Z0-9]*`), or the path to create, which ends in
        /// `.md`
        target: OsString,
        /// The title to give the document; required for a coded schema, and not taken for a path
        title: Option<String>,
        /// `field=value` to set; may repeat
        #[arg(long = "set", value_name = "K=V")]
        set: Vec<String>,
        /// How long to wait for the namespace's lock before giving up
        #[arg(long, value_name = "SECONDS", default_value_t = 5)]
        lock_timeout: u64,
        #[arg(long)]
        json: bool,
    },
    /// Read one document's frontmatter
    Get {
        /// The key or path of the document, from the project folder
        document: OsString,
        #[arg(long)]
        json: bool,
    },
    /// Query documents
    List {
        /// Collection names, separated by `,`
        #[arg(long, value_name = "LIST")]
        collection: Option<String>,
        /// A shorthand that selects the collections whose schema has this code, separated by `,`
        #[arg(long, value_name = "LIST")]
        code: Option<String>,
        /// A condition every listed document must satisfy; may repeat, ANDed
        #[arg(long = "where", value_name = "EXPR")]
        where_: Vec<String>,
        /// The extra columns of the default table, separated by `,`
        #[arg(long, value_name = "LIST")]
        fields: Option<String>,
        /// A sort key, `field` or `field:asc`/`field:desc`; may repeat, the first breaking ties
        /// first
        #[arg(long, value_name = "KEY")]
        sort: Vec<String>,
        /// The most documents to print; `total` still counts every match
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
        /// Print one key or path per line instead of the table
        #[arg(long)]
        ids: bool,
        #[arg(long)]
        json: bool,
    },
    /// Outgoing or incoming refs of one document
    Refs {
        /// The key or path of the document, from the project folder
        document: OsString,
        /// Keep only the refs held in this field, `$body` for body links
        #[arg(long, value_name = "FIELD")]
        field: Option<String>,
        /// Refs that point at the document, scanning every namespace of the project
        #[arg(long)]
        reverse: bool,
        #[arg(long)]
        json: bool,
    },
    /// List the headings of a document's body with their line ranges
    Toc {
        /// The key or path of the document, from the project folder
        document: OsString,
        /// List only the headings down to this level, 1 to 6
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(1..))]
        depth: Option<u8>,
        #[arg(long)]
        json: bool,
    },
    /// Update fields, optionally compare-and-set
    Set {
        /// The key or path of the document, from the project folder
        document: OsString,
        /// `field=value` to set, or `field=` to remove it; at least one is required
        fields: Vec<String>,
        /// A condition checked under the same lock as the write; may repeat, ANDed. If any is
        /// false, nothing is written
        #[arg(long = "if", value_name = "EXPR")]
        if_: Vec<String>,
        /// How long to wait for the namespace's lock before giving up
        #[arg(long, value_name = "SECONDS", default_value_t = 5)]
        lock_timeout: u64,
        #[arg(long)]
        json: bool,
    },
    /// Check the project, or named documents, against its schemas and rules
    Validate {
        /// The keys or paths of the documents to check; the whole project when none is given
        documents: Vec<OsString>,
        /// Check schemas only, for the whole project
        #[arg(long)]
        schemas: bool,
        /// Raise every remaining `warn` to `error`
        #[arg(long)]
        strict: bool,
        /// What has to be fixed before typdoc can be adopted
        #[arg(long)]
        audit: bool,
        #[arg(long)]
        json: bool,
    },
}

/// What a run prints and the code it ends with.
pub struct Outcome {
    pub code: u8,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(args: &[OsString], deps: &Deps) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) if matches!(e.kind(), ClapKind::DisplayHelp | ClapKind::DisplayVersion) => {
            return Outcome {
                code: 0,
                stdout: e.to_string(),
                stderr: String::new(),
            };
        }
        Err(e) => return failure_text(json, 1, e.to_string().trim_end()),
    };
    match cli.command {
        Command::New {
            target,
            title,
            set,
            lock_timeout,
            json,
        } => {
            let target_text = match target.to_str() {
                Some(text) => text,
                None => {
                    return failure_text(
                        json,
                        1,
                        &format!("{target:?} is not valid UTF-8, so it names no schema or path"),
                    );
                }
            };
            let parsed = match parse_new_target(target_text, title.as_deref()) {
                Ok(parsed) => parsed,
                Err(e) => return failure(true, exit_code(e.kind()), &e),
            };
            // Unlike every other command, `--json` is not checked first: which text form (if
            // any) is built depends on the parsed target (the bare-key form the design shows for
            // a coded schema, design `typdoc new`: "stdout: WF-3"; nothing yet for a path), and
            // parsing is pure, so deciding this after it costs nothing and does not risk a write
            // whose report is then refused.
            if !json && matches!(parsed, NewTarget::Path { .. }) {
                return failure_text(
                    false,
                    1,
                    "the output without --json is not built yet for a path-identified document",
                );
            }
            let sets = match set
                .iter()
                .map(|raw| parse_set_op(raw))
                .collect::<Result<Vec<SetOp>, Error>>()
            {
                Ok(sets) => sets,
                Err(e) => return failure(true, exit_code(e.kind()), &e),
            };
            match new_document(
                deps,
                &parsed,
                &sets,
                Duration::from_secs(lock_timeout),
                cli.namespace.as_deref(),
            ) {
                Ok(document) if json => {
                    success_raw(&raw_object(&[("document", document_json(&document))]))
                }
                Ok(document) => {
                    // `parsed` was refused above unless it is `NewTarget::Coded`, which
                    // `Project::new_document` always answers with a `key` (design, `typdoc new`:
                    // "It prints the new key, and nothing else, on standard output").
                    let key = document.key.as_deref().unwrap_or_default();
                    Outcome {
                        code: 0,
                        stdout: format!("{key}\n"),
                        stderr: String::new(),
                    }
                }
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
        Command::Get { document, json } => {
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            match get(deps, &document, cli.namespace.as_deref()) {
                Ok(document) => success_raw(&raw_object(&[("document", document_json(&document))])),
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
        Command::Set {
            document,
            fields,
            if_,
            lock_timeout,
            json,
        } => {
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            if fields.is_empty() {
                return failure_text(
                    true,
                    1,
                    "set needs at least one `field=value` or `field=` argument",
                );
            }
            let sets = match fields
                .iter()
                .map(|raw| parse_set_op(raw))
                .collect::<Result<Vec<SetOp>, Error>>()
            {
                Ok(sets) => sets,
                Err(e) => return failure(true, exit_code(e.kind()), &e),
            };
            let ifs = match parse_ifs(&if_) {
                Ok(ifs) => ifs,
                Err(e) => return failure(true, exit_code(e.kind()), &e),
            };
            match set(
                deps,
                &document,
                &sets,
                &ifs,
                Duration::from_secs(lock_timeout),
                cli.namespace.as_deref(),
            ) {
                Ok(document) => success_raw(&raw_object(&[("document", document_json(&document))])),
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
        Command::List {
            collection,
            code,
            where_,
            fields,
            sort,
            limit,
            ids,
            json,
        } => {
            if json && ids {
                return failure_text(
                    json,
                    1,
                    "--ids and --json cannot be combined: --ids has no place in the --json shape",
                );
            }
            match list(
                deps,
                collection.as_deref(),
                code.as_deref(),
                &where_,
                &sort,
                cli.namespace.as_deref(),
            ) {
                Ok(result) => list_outcome(&result, limit, fields.as_deref(), &where_, ids, json),
                Err(e) => failure(json, exit_code(e.kind()), &e),
            }
        }
        Command::Refs {
            document,
            field,
            reverse,
            json,
        } => {
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            match refs(
                deps,
                &document,
                reverse,
                field.as_deref(),
                cli.namespace.as_deref(),
            ) {
                Ok(report) => success(refs_json(&report)),
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
        Command::Toc {
            document,
            depth,
            json,
        } => {
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            match toc(deps, &document, cli.namespace.as_deref()) {
                Ok(toc) => success(toc_json(&toc, depth)),
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
        Command::Validate {
            documents,
            schemas,
            strict,
            audit,
            json,
        } => {
            if (schemas || audit) && !documents.is_empty() {
                let flag = if schemas { "--schemas" } else { "--audit" };
                return failure_text(
                    json,
                    1,
                    &format!(
                        "{flag} describes the whole project and cannot be combined with arguments"
                    ),
                );
            }
            // `--schemas` and `--audit` each describe the whole project in their own,
            // incompatible way (design: `--schemas` "checks schemas only"; `--audit` "runs the
            // same checks" with a different report). Nothing in the design says what the two
            // together would mean, and without this check `Project::validate`'s `schemas_only`
            // branch runs first and silently answers `--schemas` alone, dropping `--audit`'s
            // report with no word said — the same silent-drop this file already refuses for
            // `--json`'s absence and for `--audit` combined with arguments.
            if schemas && audit {
                return failure_text(
                    json,
                    1,
                    "--schemas and --audit cannot be combined: each describes the whole project in its own way",
                );
            }
            // Every other command, and plain `validate`, still refuse the output without
            // `--json` as not built yet. `--audit` is the one exception: the design gives it its
            // own text form (Audit mode, the worked "typdoc audit: ..." example), so it is built
            // here rather than refused alongside the rest.
            if !json && !audit {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            match validate(
                deps,
                &documents,
                schemas,
                strict,
                audit,
                cli.namespace.as_deref(),
            ) {
                Ok(report) => validate_outcome(&report, json),
                Err(e) => failure(json, exit_code(e.kind()), &e),
            }
        }
    }
}

/// A run that ends with 0 and prints `result` on standard output. A command whose result
/// holds a document has already built its JSON text and calls `success_raw` instead.
fn success(result: Json) -> Outcome {
    success_raw(&raw(&result))
}

fn success_raw(result: &RawValue) -> Outcome {
    Outcome {
        code: 0,
        stdout: format!("{}\n", result.get()),
        stderr: String::new(),
    }
}

/// The scope to pass into `Project::get`/`toc`/`refs`: chosen the ordinary way (`Project::
/// scope`) for a document of this project, or an unused placeholder for a `project::` argument.
/// A project prefix picks a different project entirely, and `Project::get`/`toc`/`refs` compute
/// their own scope inside it (`imported_scope`) before ever reading the `scope` this function
/// returns; choosing this project's scope from `arg`'s inner `namespace:` prefix would validate
/// it against the *wrong* project's namespaces (that prefix names one of the import's, not this
/// project's) and refuse arguments such as `several::story-2:WF-5` for a reason that has nothing
/// to do with them.
fn scope_for(
    project: &Project,
    arg: &DocumentArg,
    namespace: Option<&str>,
    env: &dyn Env,
) -> Result<Scope, Error> {
    if arg.project_prefix().is_some() {
        return Ok(Scope {
            source: Source::Everything,
            namespaces: Vec::new(),
            imports: Vec::new(),
        });
    }
    project.scope(arg.namespace_prefix(), namespace, env)
}

fn get(
    deps: &Deps,
    document: &std::ffi::OsStr,
    namespace: Option<&str>,
) -> Result<Document, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let scope = scope_for(&project, &arg, namespace, deps.env)?;
    project.get(&arg, &scope, deps.env)
}

/// `new`'s one argument, once its shape is known: `[A-Z][A-Z0-9]*` is a coded schema's code and
/// needs `title`; anything ending in `.md` is a path and takes none (design, `typdoc new`: two
/// forms, told apart by shape, the same way every argument that could be either always is in
/// this design). Anything else, or a form given the title shape it does not take, is bad
/// arguments.
fn parse_new_target(target: &str, title: Option<&str>) -> Result<NewTarget, Error> {
    if target.ends_with(".md") {
        if title.is_some() {
            return Err(Error::BadArgument(format!(
                "`{target}` is a path, and `new` takes no title after one: `typdoc new <path> \
                 [--set k=v ...]`"
            )));
        }
        return Ok(NewTarget::Path {
            path: target.to_owned(),
        });
    }
    if looks_like_code(target) {
        let Some(title) = title else {
            return Err(Error::BadArgument(format!(
                "`{target}` is a schema's code, and `new` needs a title: `typdoc new {target} \
                 \"<title>\"`"
            )));
        };
        return Ok(NewTarget::Coded {
            code: target.to_owned(),
            title: title.to_owned(),
        });
    }
    Err(Error::BadArgument(format!(
        "`{target}` is neither a schema's code (`[A-Z][A-Z0-9]*`) nor a path, which ends in `.md`"
    )))
}

/// `[A-Z][A-Z0-9]*`, the shape of a schema's own `code` (design, Schema format).
fn looks_like_code(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

fn new_document(
    deps: &Deps,
    target: &NewTarget,
    sets: &[SetOp],
    lock_timeout: Duration,
    namespace: Option<&str>,
) -> Result<Document, Error> {
    let root = discover(deps.env)?;
    let project = Project::load(&root, deps.env)?;
    project.new_document(target, deps, sets, lock_timeout, namespace)
}

#[allow(
    clippy::too_many_arguments,
    reason = "each is a distinct argument set's own value"
)]
fn set(
    deps: &Deps,
    document: &std::ffi::OsStr,
    sets: &[SetOp],
    ifs: &[(String, Condition)],
    lock_timeout: Duration,
    namespace: Option<&str>,
) -> Result<Document, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let scope = scope_for(&project, &arg, namespace, deps.env)?;
    project.set(&arg, &scope, deps, sets, ifs, lock_timeout)
}

/// One `set` argument as the design's grammar reads it: `field=value` sets it, `field=` (nothing
/// after the `=`) removes it (design, `typdoc set`: "`k=` removes a field"). An argument with no
/// `=` at all, or with nothing before it, is bad arguments: `=v` and `v` alike name no field.
fn parse_set_op(raw: &str) -> Result<SetOp, Error> {
    let Some((field, value)) = raw.split_once('=') else {
        return Err(Error::BadArgument(format!(
            "`{raw}` is not `field=value` or `field=`: a `set` argument needs an `=`"
        )));
    };
    if field.is_empty() {
        return Err(Error::BadArgument(format!(
            "`{raw}` names no field before `=`"
        )));
    }
    if value.is_empty() {
        Ok(SetOp::Remove {
            field: field.to_owned(),
        })
    } else {
        Ok(SetOp::Set {
            field: field.to_owned(),
            raw: value.to_owned(),
        })
    }
}

/// Every `--if` expression, parsed by the same grammar `--where` uses (design, `typdoc set`:
/// "`--if` uses `--where` expressions"), kept beside its own original text so a false condition's
/// message can quote exactly what the caller wrote.
fn parse_ifs(raw: &[String]) -> Result<Vec<(String, Condition)>, Error> {
    raw.iter()
        .map(|expr| {
            parse_query(expr)
                .map(|condition| (expr.clone(), condition))
                .map_err(|e| Error::BadArgument(e.to_string()))
        })
        .collect()
}

fn toc(deps: &Deps, document: &std::ffi::OsStr, namespace: Option<&str>) -> Result<Toc, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let scope = scope_for(&project, &arg, namespace, deps.env)?;
    project.toc(&arg, &scope, deps.env)
}

/// `list`: no document argument, so the scope is chosen with no namespace prefix (`--namespace`,
/// `TYPDOC_NAMESPACE` and the current directory still apply). Returns every matching document,
/// sorted, `--limit` not yet applied: the caller (`list_outcome`) cuts the result for it, so a
/// value the table prints (a column's width) can be computed from the whole match and stay the
/// same whatever `--limit` is (design: a flag that limits what is listed never changes the
/// values of an item).
fn list(
    deps: &Deps,
    collection: Option<&str>,
    code: Option<&str>,
    wheres: &[String],
    sort: &[String],
    namespace: Option<&str>,
) -> Result<ListResult, Error> {
    let root = discover(deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let scope = project.scope(None, namespace, deps.env)?;
    let collections = split_list(collection);
    let codes = split_list(code);
    let conditions: Vec<_> = wheres
        .iter()
        .map(|expr| parse_query(expr).map_err(|e| Error::BadArgument(e.to_string())))
        .collect::<Result<_, _>>()?;
    let sort_keys: Vec<SortKey> = sort
        .iter()
        .map(|spec| parse_sort_key(spec))
        .collect::<Result<_, _>>()?;
    let filter = ListFilter {
        collections: &collections,
        codes: &codes,
        wheres: &conditions,
        sort: &sort_keys,
    };
    project.list_all(&scope, &filter)
}

/// `--collection`/`--code`'s value, split on `,`; absent is the same as empty (every collection).
fn split_list(value: Option<&str>) -> Vec<String> {
    value
        .map(|v| v.split(',').map(str::to_owned).collect())
        .unwrap_or_default()
}

/// One `--sort` entry: `field` alone (ascending) or `field:asc`/`field:desc`. Any other suffix
/// after the `:` is an error (design, Sorting: "anything else is an error").
fn parse_sort_key(spec: &str) -> Result<SortKey, Error> {
    let (name, dir) = match spec.split_once(':') {
        Some((name, dir)) => (name, Some(dir)),
        None => (spec, None),
    };
    let field = parse_field(name).map_err(|e| Error::BadArgument(e.to_string()))?;
    let desc = match dir {
        None | Some("asc") => false,
        Some("desc") => true,
        Some(other) => {
            return Err(Error::BadArgument(format!(
                "`{other}` is not a sort direction: it is `asc` or `desc`"
            )));
        }
    };
    Ok(SortKey { field, desc })
}

/// `list`'s three output shapes, chosen by `--json`/`--ids`/neither: `total` and `truncated`
/// (design: `truncated` is true exactly when `total`, the count before `--limit`, is larger than
/// the number listed) are computed here, once, from `matched` (every document `list` matched, in
/// order) and `limit`, so every shape agrees with the other two. `result.dangling_refs` goes to
/// stderr the same way whichever of the three shapes is chosen (design, Query: "dangling refs
/// also warn on stderr" — the warning is not part of any of the three shapes `--json` names, so
/// it is not conditioned on `json` the way `stdout` is).
fn list_outcome(
    result: &ListResult,
    limit: Option<usize>,
    fields: Option<&str>,
    wheres: &[String],
    ids: bool,
    json: bool,
) -> Outcome {
    let matched = &result.documents;
    let stderr = dangling_refs_stderr(&result.dangling_refs);
    let total = matched.len();
    let listed = match limit {
        Some(n) => &matched[..n.min(matched.len())],
        None => matched,
    };
    let truncated = total > listed.len();
    if json {
        return Outcome {
            stderr,
            ..success_raw(&list_json(listed, total, truncated))
        };
    }
    if ids {
        let mut stdout = String::new();
        for doc in listed {
            stdout.push_str(doc.key.as_deref().unwrap_or(doc.path.as_str()));
            stdout.push('\n');
        }
        return Outcome {
            code: 0,
            stdout,
            stderr,
        };
    }
    let columns = table_columns(fields, wheres);
    Outcome {
        code: 0,
        stdout: list_table(matched, listed.len(), &columns),
        stderr,
    }
}

/// Every dangling ref a `ref.*` condition reached, one per line, for stderr (design, Query,
/// "Reached documents": "dangling refs also warn on stderr"). Empty when none were reached, the
/// ordinary case, which prints nothing.
fn dangling_refs_stderr(dangling_refs: &[String]) -> String {
    if dangling_refs.is_empty() {
        return String::new();
    }
    let mut text = dangling_refs.join("\n");
    text.push('\n');
    text
}

fn list_json(documents: &[Document], total: usize, truncated: bool) -> Box<RawValue> {
    let documents: Vec<Box<RawValue>> = documents.iter().map(document_json).collect();
    raw_object(&[
        ("documents", raw_array(&documents)),
        ("total", raw(&json!(total))),
        ("truncated", raw(&json!(truncated))),
    ])
}

/// The default table's extra columns (beyond the key-or-path identity and `title`): `--fields`
/// when given, or else every field named by a `--where` condition, each once, in the order it
/// first appears (design: "Default output is a table of key or path, title, and every field
/// used in `--where`"). A `--where` expression that failed to parse never reaches here (`list`
/// already returned its error), so re-parsing it for its field name alone is safe.
///
/// A `ref.*`/`refby.*` condition names one of my own fields too — the ref field `f` itself, e.g.
/// `blocked_by` in `ref.all(blocked_by).status=resolved` — so that field is added the same way a
/// plain condition's field is; `status` in that example belongs to the document reached, not to
/// "me", and is left out, the same as `$body` (not a printable frontmatter field of "me") is.
fn table_columns(fields: Option<&str>, wheres: &[String]) -> Vec<String> {
    if let Some(fields) = fields {
        return fields.split(',').map(str::to_owned).collect();
    }
    let mut columns = Vec::new();
    for expr in wheres {
        if let Ok(condition) = parse_query(expr) {
            let name = match &condition {
                Condition::Plain(plain) => Some(field_name(&plain.field)),
                Condition::Ref(ref_condition) => match &ref_condition.field {
                    RefField::Named(name) => Some(name.clone()),
                    RefField::Body => None,
                },
            };
            if let Some(name) = name
                && !columns.contains(&name)
            {
                columns.push(name);
            }
        }
    }
    columns
}

fn field_name(field: &FieldRef) -> String {
    match field {
        FieldRef::Path => "path".to_owned(),
        FieldRef::Key => "key".to_owned(),
        FieldRef::Code => "code".to_owned(),
        FieldRef::Collection => "collection".to_owned(),
        FieldRef::Schema => "schema".to_owned(),
        FieldRef::Namespace => "namespace".to_owned(),
        FieldRef::Named(name) => name.clone(),
    }
}

/// The default text table: one row per document of `matched[..listed_len]`, columns padded to
/// the width their longest value takes across the whole of `matched` (not only the rows
/// printed), so that a value the table prints does not change with `--limit` (the ticket's own
/// criterion: a column's width computed from the listed rows only would move when `--limit`
/// changes which rows are listed, and this construction cannot do that, since every row's cells
/// are measured before any row is left out). Columns are separated by two spaces; there is no
/// header row, since the design gives none.
fn list_table(matched: &[Document], listed_len: usize, columns: &[String]) -> String {
    let column_count = 2 + columns.len();
    let rows: Vec<Vec<String>> = matched.iter().map(|doc| table_row(doc, columns)).collect();
    let mut widths = vec![0usize; column_count];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in rows.iter().take(listed_len) {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                line.push_str("  ");
            }
            line.push_str(cell);
            if i + 1 < row.len() {
                line.push_str(&" ".repeat(widths[i] - cell.chars().count()));
            }
        }
        // The padding above never trails past a row's last non-empty cell except when that very
        // last cell is itself empty (an unset field in the rightmost column): trimmed here so an
        // empty trailing column leaves no trailing whitespace, without touching a column's own
        // width (computed above, from `matched`, and untouched by this).
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn table_row(doc: &Document, columns: &[String]) -> Vec<String> {
    let mut row = vec![doc.key.clone().unwrap_or_else(|| doc.path.clone())];
    row.push(cell_value(doc, "title"));
    for column in columns {
        row.push(cell_value(doc, column));
    }
    row
}

/// One cell of the default table: a pseudo-field read straight off `doc`, a named field read
/// from `doc.fields` (as `get`'s own `fields` already holds it), and a field this document has
/// no value for as an empty cell (a document whose collection lacks it, or a document simply
/// never given it).
fn cell_value(doc: &Document, field: &str) -> String {
    match field {
        "path" => doc.path.clone(),
        "key" => doc.key.clone().unwrap_or_default(),
        "code" => doc.code.clone().unwrap_or_default(),
        "collection" => doc.collection.clone(),
        "schema" => doc.schema.clone(),
        "namespace" => doc.namespace.clone().unwrap_or_default(),
        other => doc
            .fields
            .iter()
            .find(|(found, _)| found == other)
            .map(|(_, value)| render_cell(value))
            .unwrap_or_default(),
    }
}

/// A field's value as the table prints it: as written for text-shaped values, the value a
/// number or a bool converts to, and comma-joined for a list (the table has one cell per
/// document, not one per element). `--json` is where a number's own digits are printed.
fn render_cell(value: &Value) -> String {
    match value {
        Value::Text(text) | Value::Date(text) | Value::Datetime(text) => text.clone(),
        Value::Empty => String::new(),
        Value::List(items) => items.join(","),
        Value::Number(n) => n.converted(),
        Value::Bool(b) => b.to_string(),
    }
}

fn refs(
    deps: &Deps,
    document: &std::ffi::OsStr,
    reverse: bool,
    field: Option<&str>,
    namespace: Option<&str>,
) -> Result<RefsReport, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let scope = scope_for(&project, &arg, namespace, deps.env)?;
    project.refs(&arg, &scope, reverse, field, deps.env)
}

fn validate(
    deps: &Deps,
    documents: &[OsString],
    schemas: bool,
    strict: bool,
    audit: bool,
    namespace: Option<&str>,
) -> Result<ValidateReport, Error> {
    let (root, args) = validate_args(deps.env, documents)?;
    // A config error stops here today (`Error::ConfigErrors`, see `config::Report`'s own
    // comment on why), before `project.validate` below ever runs: nothing reaches it as a
    // finding yet. This `?` is where a config error that answered the design's question on its
    // own, "does it make checking impossible?", with no, would instead let the project load and
    // reach `validate`'s report. This is also `--audit`'s own "unless the config itself is
    // invalid" exception (design, Audit mode): a config error still ends here with its own exit
    // code, before `project.validate` ever produces a report for `audit` to make exit 0.
    let project = Project::load(&root, deps.env)?;
    project.validate(&args, schemas, strict, audit, namespace, deps.env)
}

/// The project every argument is read against, found from the first argument as `get` finds
/// it, and the arguments themselves. An on-disk argument after the first is read against that
/// same, already-known root rather than walking up from its own folder again.
fn validate_args(env: &dyn Env, raw: &[OsString]) -> Result<(PathBuf, Vec<DocumentArg>), Error> {
    let Some((first, rest)) = raw.split_first() else {
        return Ok((discover(env)?, Vec::new()));
    };
    let (root, first) = discover_for(Argument::parse(first)?, env)?;
    let mut args = vec![first];
    for raw_arg in rest {
        args.push(match Argument::parse(raw_arg)? {
            Argument::Named(document) => document,
            Argument::OnDisk(path) => resolve_on_disk(&root, &path, env)?,
        });
    }
    Ok((root, args))
}

/// `validate`'s own outcome: the report on standard output whether or not it is favourable
/// (design, JSON output), with exit 2 when a finding is an error and 0 otherwise — except under
/// `--audit`, whose exit code is 0 unless the config itself is invalid (design, Audit mode), and
/// a config error never reaches this far: it ends the run before a report exists (`validate`'s
/// own comment on its `?`). `json` chooses between the two output shapes the CLI's own match arm
/// already decided are the only two reachable here: `--json`, or `--audit` alone, which is the
/// one command whose text form this ticket builds.
fn validate_outcome(report: &ValidateReport, json: bool) -> Outcome {
    let code = if report.audit.is_some() {
        0
    } else if report.findings.iter().any(|f| f.level == Severity::Error) {
        2
    } else {
        0
    };
    let stdout = if json {
        format!("{}\n", validate_json(report))
    } else {
        audit_text(report)
    };
    Outcome {
        code,
        stdout,
        stderr: String::new(),
    }
}

fn validate_json(report: &ValidateReport) -> Json {
    let mut checked = Map::new();
    checked.insert("namespaces".to_owned(), json!(report.namespaces));
    checked.insert("documents".to_owned(), json!(report.documents));
    if let Some(paths) = &report.paths {
        checked.insert("paths".to_owned(), json!(paths));
    }
    let (error, warn, info) = report.findings.iter().fold(
        (0u32, 0u32, 0u32),
        |(error, warn, info), finding| match finding.level {
            Severity::Error => (error + 1, warn, info),
            Severity::Warn => (error, warn + 1, info),
            Severity::Info => (error, warn, info + 1),
        },
    );
    let mut summary = Map::new();
    summary.insert("scope".to_owned(), json!(scope_name(report.scope)));
    summary.insert("strict".to_owned(), json!(report.strict));
    summary.insert("checked".to_owned(), Json::Object(checked));
    summary.insert(
        "findings".to_owned(),
        json!({ "error": error, "warn": warn, "info": info }),
    );
    if let Some(audit) = &report.audit {
        summary.insert("audit".to_owned(), json!(true));
        summary.insert(
            "unreported".to_owned(),
            json!({
                "uncollected": audit.uncollected.len(),
                "no_frontmatter": audit.no_frontmatter.len(),
            }),
        );
        // Beside `unreported`, not inside it (design, the paragraph beginning "The summary of
        // `validate`"): an overlapping file is reported under `collections.overlap`, so it is
        // not outside `findings`, which is what `unreported` means. Contract item 8's own
        // accounting equation reads `summary.overlapping` directly, so this is the number a
        // caller who only reads the summary needs to reconcile the run.
        summary.insert("overlapping".to_owned(), json!(audit.overlapping.len()));
        // Beside `unreported` and `overlapping` for the same reason as `overlapping` itself:
        // every one of these is already reported in `findings` (`files.unreadable` or
        // `files.leftover`), so it is not `unreported`. Without it the account a reader takes
        // from the summary would be short by every entry the run skipped (design, the
        // paragraph on `not_read`).
        summary.insert("not_read".to_owned(), json!(audit.not_read.len()));
    }
    let findings: Vec<Json> = report.findings.iter().map(finding_json).collect();
    let mut object = Map::new();
    object.insert("summary".to_owned(), Json::Object(summary));
    object.insert("findings".to_owned(), json!(findings));
    if let Some(audit) = &report.audit {
        object.insert("audit".to_owned(), audit_json(audit));
    }
    Json::Object(object)
}

/// The `audit` object of `--json`'s report (design, JSON output, "Audit"): `collections`, one
/// `{ "name", "documents" }` per collection of the project, sorted by name (already sorted by
/// `AuditReport::collections`); `uncollected` and `no_frontmatter`, the sorted `path` of every file
/// the design names; `overlapping`, one `{ "path", "collections" }` per file, sorted by path; and
/// `not_read`, one `{ "path", "reason" }` per directory entry the run met and did not read,
/// sorted by path.
fn audit_json(audit: &AuditReport) -> Json {
    let collections: Vec<Json> = audit
        .collections
        .iter()
        .map(|collection| json!({ "name": collection.name, "documents": collection.documents }))
        .collect();
    let overlapping: Vec<Json> = audit
        .overlapping
        .iter()
        .map(|overlap| json!({ "path": overlap.path, "collections": overlap.collections }))
        .collect();
    let not_read: Vec<Json> = audit
        .not_read
        .iter()
        .map(|entry| json!({ "path": entry.path, "reason": entry.reason }))
        .collect();
    json!({
        "collections": collections,
        "uncollected": audit.uncollected,
        "no_frontmatter": audit.no_frontmatter,
        "overlapping": overlapping,
        "not_read": not_read,
    })
}

fn scope_name(scope: ValidateScope) -> &'static str {
    match scope {
        ValidateScope::All => "all",
        ValidateScope::Paths => "paths",
        ValidateScope::Schemas => "schemas",
    }
}

fn severity_name(level: Severity) -> &'static str {
    match level {
        Severity::Error => "error",
        Severity::Warn => "warn",
        Severity::Info => "info",
    }
}

/// The text form of `--audit` (design, Audit mode, the worked `typdoc audit: ...` example): a
/// header, one line per collection (its document count and, grouped by rule, the count and level
/// of its findings, or `clean` when it has none), and a line naming every file in no collection,
/// every file with no frontmatter and every file matched by more than one collection, when the
/// list is not empty. `overlapping` is not part of the design's worked example (contract item 8
/// is what gives it its own list), so this function lists it the same way it already lists
/// `no_frontmatter` beyond that example: one line, only when the list holds something, so the
/// text's own total agrees with `summary.overlapping` in the JSON; each path is followed by the
/// names of the collections that match it. The design's own worked example is not spaced by an
/// algorithm this reads out consistently (`precedents`/`learnings`, the same length short of
/// their suffix, are padded two different amounts there), so the padding here is this function's
/// own, simple and deterministic: the name column is as wide as the
/// longest collection name, plus two spaces, and every group is separated by " · " as the example
/// shows. Design line 577 also promises "then details" after the summary for audit's text form,
/// the per-finding lines plain `validate` prints one of per line; no command's plain-text output
/// is built yet (every one of them still refuses the output without `--json`, this ticket's own
/// `--audit` exception aside), so there is no such renderer yet to append here, and this stays a
/// summary only, carried forward for whichever ticket gives `validate` its own text form.
fn audit_text(report: &ValidateReport) -> String {
    let Some(audit) = &report.audit else {
        return String::new();
    };
    // Contract item 8's equation, in text form: `checked.documents` plus every count of what was
    // not checked, `uncollected`, `no_frontmatter`, `overlapping` and `not_read`, is the number
    // of directory entries the run met. Leaving any one of them out of `total` would make a
    // reader of the text see fewer entries than a reader of the JSON's `summary` does.
    let total = report.documents
        + audit.uncollected.len()
        + audit.no_frontmatter.len()
        + audit.overlapping.len()
        + audit.not_read.len();
    let mut out = format!(
        "typdoc audit: {} collections, {total} files ({} in no collection)\n\n",
        audit.collections.len(),
        audit.uncollected.len()
    );
    let name_width = audit
        .collections
        .iter()
        .map(|collection| collection.name.chars().count())
        .max()
        .unwrap_or(0);
    for collection in &audit.collections {
        let body = collection_summary(&collection.name, &report.findings);
        out.push_str(&format!(
            "{:<name_width$}  {} files   {body}\n",
            collection.name, collection.documents
        ));
    }
    if !audit.uncollected.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "in no collection: {} ({})\n",
            audit.uncollected.join(", "),
            audit.uncollected.len()
        ));
    }
    if !audit.no_frontmatter.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "no frontmatter: {} ({})\n",
            audit.no_frontmatter.join(", "),
            audit.no_frontmatter.len()
        ));
    }
    if !audit.overlapping.is_empty() {
        out.push('\n');
        let overlapping: Vec<String> = audit
            .overlapping
            .iter()
            .map(|overlap| format!("{} ({})", overlap.path, overlap.collections.join(", ")))
            .collect();
        out.push_str(&format!(
            "matched by more than one collection: {} ({})\n",
            overlapping.join(", "),
            audit.overlapping.len()
        ));
    }
    if !audit.not_read.is_empty() {
        out.push('\n');
        let not_read: Vec<String> = audit
            .not_read
            .iter()
            .map(|entry| format!("{} ({})", entry.path, entry.reason))
            .collect();
        out.push_str(&format!(
            "not read: {} ({})\n",
            not_read.join(", "),
            audit.not_read.len()
        ));
    }
    out
}

/// One collection's part of the audit summary line: every rule that found something in it,
/// grouped (a rule's findings in one collection always share one level, since `--audit` computes
/// one effective level per rule per collection), as `{rule} {count} {level}`, joined by " · ".
/// The design does not say how the groups are ordered, and its own worked example is not
/// alphabetical (`frontmatter.types, body.links, body.mentions`); what it is consistent with is
/// the order `findings` is already guaranteed to carry (by `path`, then position, then `rule`):
/// a rule's group appears where its first finding does in that order, which is what `findings`
/// is passed in here already sorted by (`report.findings`, ordered by `validate::order`). `clean`
/// when the collection has no finding.
fn collection_summary(collection: &str, findings: &[Finding]) -> String {
    let mut order: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, (Severity, u32)> = BTreeMap::new();
    for finding in findings {
        if finding.collection.as_deref() != Some(collection) {
            continue;
        }
        if !groups.contains_key(finding.rule) {
            order.push(finding.rule);
        }
        groups
            .entry(finding.rule)
            .and_modify(|(_, count)| *count += 1)
            .or_insert((finding.level, 1));
    }
    if order.is_empty() {
        return "clean".to_owned();
    }
    order
        .into_iter()
        .map(|rule| {
            let (level, count) = groups[rule];
            format!("{rule} {count} {}", severity_name(level))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn finding_json(finding: &Finding) -> Json {
    let mut object = Map::new();
    object.insert("path".to_owned(), json!(finding.path));
    if let Some(namespace) = &finding.namespace {
        object.insert("namespace".to_owned(), json!(namespace));
    }
    if let Some(collection) = &finding.collection {
        object.insert("collection".to_owned(), json!(collection));
    }
    if let Some(key) = &finding.key {
        object.insert("key".to_owned(), json!(key));
    }
    if let Some(field) = &finding.field {
        object.insert("field".to_owned(), json!(field));
    }
    if let Some(position) = finding.position {
        object.insert("line".to_owned(), json!(position.line));
        object.insert("col".to_owned(), json!(position.col));
    }
    object.insert("rule".to_owned(), json!(finding.rule));
    object.insert("level".to_owned(), json!(severity_name(finding.level)));
    object.insert("message".to_owned(), json!(finding.message));
    Json::Object(object)
}

fn exit_code(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::BadArguments => 1,
        ErrorKind::Validation => 2,
        ErrorKind::IfFalse => 3,
        ErrorKind::LockTimeout => 4,
        ErrorKind::NotFound => 5,
        ErrorKind::Io => 6,
        ErrorKind::Exists => 7,
    }
}

/// `{ "error": text, "code": code, "details": [] }`, the error object every failure starts
/// from; a failure that has more to say adds to it.
fn error_object(text: &str, code: u8) -> Map<String, Json> {
    let mut object = Map::new();
    object.insert("error".to_owned(), json!(text));
    object.insert("code".to_owned(), json!(code));
    object.insert("details".to_owned(), json!([]));
    object
}

/// The error object on standard error with `--json`, and one line of text without.
fn failure_text(json: bool, code: u8, text: &str) -> Outcome {
    let stderr = if json {
        format!("{}\n", Json::Object(error_object(text, code)))
    } else {
        format!("typdoc: {text}\n")
    };
    Outcome {
        code,
        stdout: String::new(),
        stderr,
    }
}

/// The error object of a failure that ended a command, with the config errors as `details`, or
/// with `candidates` when a key was ambiguous across namespaces.
fn failure(json: bool, code: u8, error: &Error) -> Outcome {
    let mut object = error_object(&error.to_string(), code);
    match error {
        Error::ConfigErrors { errors, complete } => {
            let details: Vec<Json> = errors
                .iter()
                .map(|e| {
                    json!({ "level": "error", "rule": e.id, "message": e.message, "path": e.path })
                })
                .collect();
            object.insert("details".to_owned(), json!(details));
            object.insert("complete".to_owned(), json!(complete));
        }
        Error::AmbiguousKey { candidates, .. } | Error::AmbiguousScope { candidates } => {
            object.insert("candidates".to_owned(), json!(candidates));
        }
        Error::Invalid { findings } | Error::IfFalse { findings } => {
            let details: Vec<Json> = findings.iter().map(finding_json).collect();
            object.insert("details".to_owned(), json!(details));
        }
        _ => return failure_text(json, code, &error.to_string()),
    }
    Outcome {
        code,
        stdout: String::new(),
        stderr: if json {
            format!("{}\n", Json::Object(object))
        } else {
            format!("typdoc: {error}\n")
        },
    }
}

/// The headings down to `depth`, each with the `end` it has in the whole document.
fn toc_json(toc: &Toc, depth: Option<u8>) -> Json {
    let headings: Vec<Json> = toc
        .headings
        .iter()
        .filter(|heading| depth.is_none_or(|depth| heading.level <= depth))
        .map(|heading| {
            json!({
                "level": heading.level,
                "text": heading.text,
                "slug": heading.slug,
                "line": heading.line,
                "end": heading.end,
            })
        })
        .collect();
    json!({
        "document": Json::Object(document_name(
            &toc.path,
            Some(&toc.namespace),
            toc.key.as_deref(),
            toc.project.as_deref(),
        )),
        "headings": headings,
    })
}

/// `refs`' report: the document asked about, `direction`, and its references in the order
/// `Project::refs` already gives them.
fn refs_json(report: &RefsReport) -> Json {
    let refs: Vec<Json> = report.refs.iter().map(reference_json).collect();
    json!({
        "document": Json::Object(document_name(
            &report.document.path,
            report.document.namespace.as_deref(),
            report.document.key.as_deref(),
            report.document.project.as_deref(),
        )),
        "direction": direction_name(report.direction),
        "refs": refs,
    })
}

fn direction_name(direction: RefsDirection) -> &'static str {
    match direction {
        RefsDirection::Out => "out",
        RefsDirection::In => "in",
    }
}

/// One reference: the name of the document at the other end when it resolved, `unresolved`
/// otherwise (never both — the ticket's own guarantee), plus `field`, `written` and, for a body
/// link, `line` and `col`.
fn reference_json(reference: &RefsReference) -> Json {
    let mut object = Map::new();
    match &reference.other {
        RefOutcome::Resolved(name) => {
            object.insert("path".to_owned(), json!(name.path));
            if let Some(namespace) = &name.namespace {
                object.insert("namespace".to_owned(), json!(namespace));
            }
            if let Some(key) = &name.key {
                object.insert("key".to_owned(), json!(key));
            }
            if let Some(project) = &name.project {
                object.insert("project".to_owned(), json!(project));
            }
        }
        RefOutcome::Unresolved(reason) => {
            object.insert("unresolved".to_owned(), json!(reason));
        }
    }
    object.insert("field".to_owned(), json!(reference.field));
    object.insert("written".to_owned(), json!(reference.written));
    if let Some(position) = reference.position {
        object.insert("line".to_owned(), json!(position.line));
        object.insert("col".to_owned(), json!(position.col));
    }
    Json::Object(object)
}

/// The name of a document: `path` always, `namespace` unless the file is outside every
/// namespace folder, `key` only for a coded document, `project` only for a document of an
/// imported project (design: "`project`... is absent for a document of this project").
fn document_name(
    path: &str,
    namespace: Option<&str>,
    key: Option<&str>,
    project: Option<&str>,
) -> Map<String, Json> {
    let mut object = Map::new();
    object.insert("path".to_owned(), json!(path));
    if let Some(namespace) = namespace {
        object.insert("namespace".to_owned(), json!(namespace));
    }
    if let Some(key) = key {
        object.insert("key".to_owned(), json!(key));
    }
    if let Some(project) = project {
        object.insert("project".to_owned(), json!(project));
    }
    object
}

/// A document as the design's JSON output has it. It is built as JSON text rather than as a
/// `serde_json::Value` for one reason: a `number` is printed with the digits written in the
/// document, and no `serde_json::Number` holds `1e3` as `1e3` — it is either converted to
/// `1000.0` or, with the digits kept, written back with an exponent sign the file never had.
/// Everything else here is an ordinary value turned into its JSON text first.
fn document_json(document: &Document) -> Box<RawValue> {
    let name = document_name(
        &document.path,
        document.namespace.as_deref(),
        document.key.as_deref(),
        document.project.as_deref(),
    );
    let mut object: Vec<(&str, Box<RawValue>)> = name
        .iter()
        .map(|(key, value)| (&**key, raw(value)))
        .collect();
    let fields: Vec<(&str, Box<RawValue>)> = document
        .fields
        .iter()
        .map(|(name, value)| (&**name, value_json(value)))
        .collect();
    object.push(("code", raw(&json!(document.code))));
    object.push(("collection", raw(&json!(document.collection))));
    object.push(("schema", raw(&json!(document.schema))));
    object.push(("fields", raw_object(&fields)));
    raw_object(&object)
}

fn value_json(value: &Value) -> Box<RawValue> {
    match value {
        Value::Text(text) => raw(&json!(text)),
        // A field written with no value at all, kept apart from one written as the empty
        // string (design, "Document files" and "JSON output": "the first is `null`").
        Value::Empty => raw(&json!(null)),
        Value::List(items) => raw(&json!(items)),
        // The one value that does not go through `serde_json::Value`: the digits as the
        // document wrote them, which `Number::read` has already checked are a JSON number.
        Value::Number(number) => raw_text(number.written().to_owned()),
        Value::Bool(flag) => raw(&json!(flag)),
        Value::Date(text) | Value::Datetime(text) => raw(&json!(text)),
    }
}

/// `value` as the JSON text that stands for it.
fn raw(value: &Json) -> Box<RawValue> {
    raw_text(value.to_string())
}

/// `text`, which is already JSON, as a value that can be put inside another.
#[expect(
    clippy::expect_used,
    reason = "every caller passes either the printed form of a `serde_json::Value`, or an \
              object or array `joined` has assembled from values that are themselves JSON \
              text, or the digits `Number::read` has parsed as a JSON number"
)]
fn raw_text(text: String) -> Box<RawValue> {
    RawValue::from_string(text).expect("assembled from JSON")
}

/// A JSON object of values that are already JSON text, in the order given.
fn raw_object(entries: &[(&str, Box<RawValue>)]) -> Box<RawValue> {
    let parts = entries
        .iter()
        .map(|(key, value)| format!("{}:{}", json!(key), value.get()));
    joined('{', parts, '}')
}

/// A JSON array of values that are already JSON text, in the order given.
fn raw_array(items: &[Box<RawValue>]) -> Box<RawValue> {
    joined('[', items.iter().map(|item| item.get().to_owned()), ']')
}

/// `parts`, each of them JSON text, separated by commas and wrapped in `open` and `close`.
fn joined(open: char, parts: impl Iterator<Item = String>, close: char) -> Box<RawValue> {
    let mut text = String::from(open);
    for (at, part) in parts.enumerate() {
        if at > 0 {
            text.push(',');
        }
        text.push_str(&part);
    }
    text.push(close);
    raw_text(text)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io;
    use std::path::PathBuf;

    use serde_json::json;
    use typdoc_core::{Deps, Env};
    use typdoc_testkit::fake::{FakeFs, FixedClock};

    use super::run;

    struct FakeEnv {
        cwd: PathBuf,
        vars: HashMap<&'static str, &'static str>,
    }

    impl Env for FakeEnv {
        fn var(&self, name: &str) -> Option<OsString> {
            self.vars.get(name).map(OsString::from)
        }

        fn current_dir(&self) -> io::Result<PathBuf> {
            Ok(self.cwd.clone())
        }

        fn hostname(&self) -> io::Result<String> {
            Ok("test-host".to_owned())
        }
    }

    fn fixtures() -> PathBuf {
        typdoc_testkit::fixtures::path("")
    }

    fn get_note(env: &FakeEnv) -> super::Outcome {
        let args: Vec<OsString> = ["typdoc", "get", "note.md", "--json"]
            .iter()
            .map(OsString::from)
            .collect();
        run(
            &args,
            &Deps {
                env,
                fs: &FakeFs::new(),
                clock: &FixedClock::new(),
            },
        )
    }

    #[test]
    fn the_current_directory_comes_from_env_and_not_from_the_process() {
        let env = FakeEnv {
            cwd: fixtures().join("valid/minimal"),
            vars: HashMap::new(),
        };

        let outcome = get_note(&env);

        assert_eq!(outcome.code, 0, "{}", outcome.stderr);
        let printed: serde_json::Value = serde_json::from_str(&outcome.stdout).expect("JSON");
        assert_eq!(printed["document"]["path"], json!("note.md"));
    }

    #[test]
    fn typdoc_dir_comes_from_env() {
        let env = FakeEnv {
            cwd: fixtures(),
            vars: HashMap::from([("TYPDOC_DIR", "valid/minimal")]),
        };

        assert_eq!(get_note(&env).code, 0);
    }

    #[test]
    fn an_empty_typdoc_dir_is_not_set_so_the_walk_finds_the_project() {
        let env = FakeEnv {
            cwd: fixtures().join("valid/minimal/schemas"),
            vars: HashMap::from([("TYPDOC_DIR", "")]),
        };

        assert_eq!(get_note(&env).code, 0);
    }
}
