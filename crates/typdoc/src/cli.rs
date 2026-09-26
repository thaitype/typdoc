use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clap::error::ErrorKind as ClapKind;
use clap::{Parser, Subcommand};
use serde_json::value::RawValue;
use serde_json::{Map, Value as Json, json};
use typdoc_core::{
    Argument, AuditReport, Condition, Deps, Document, DocumentArg, Env, Error, ErrorKind, FieldRef,
    Finding, Heading, ListFilter, ListResult, MvReport, NewTarget, Position, Project, RefField,
    RefName, RefOutcome, RefsDirection, RefsReference, RefsReport, RewrittenRef, Scope, SetOp,
    Severity, SortKey, Source, Toc, UnrewrittenReason, UnrewrittenRef, ValidateReport,
    ValidateScope, Value, discover, discover_for, parse_field, parse_query, resolve_on_disk,
};

const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 5;

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
    /// Move a document within this project, rewriting every ref this project holds to it
    Mv {
        /// The key or path of the document to move, from the project folder
        from: OsString,
        /// The path it moves to; not given with `--renumber`, which takes one positional
        /// instead of two. A `mv` whose destination already exists writes nothing
        to: Option<OsString>,
        /// Moves a coded document to another namespace of this project under a new key, instead
        /// of the plain, two-positional form; requires a namespace, and cannot name another
        /// project
        #[arg(long, value_name = "NAMESPACE")]
        renumber: Option<OsString>,
        /// How long to wait for a namespace lock before giving up
        #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_LOCK_TIMEOUT_SECS)]
        lock_timeout: u64,
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
                Err(e) => return failure(json, exit_code(e.kind()), &e),
            };
            let sets = match set
                .iter()
                .map(|raw| parse_set_op(raw))
                .collect::<Result<Vec<SetOp>, Error>>()
            {
                Ok(sets) => sets,
                Err(e) => return failure(json, exit_code(e.kind()), &e),
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
                Ok(document) => Outcome {
                    code: 0,
                    stdout: document_text(&document),
                    stderr: String::new(),
                },
                Err(e) => failure(json, exit_code(e.kind()), &e),
            }
        }
        Command::Get { document, json } => match get(deps, &document, cli.namespace.as_deref()) {
            Ok(document) if json => {
                success_raw(&raw_object(&[("document", document_json(&document))]))
            }
            Ok(document) => Outcome {
                code: 0,
                stdout: document_text(&document),
                stderr: String::new(),
            },
            Err(e) => failure(json, exit_code(e.kind()), &e),
        },
        Command::Set {
            document,
            fields,
            if_,
            lock_timeout,
            json,
        } => {
            if fields.is_empty() {
                return failure_text(
                    json,
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
                Err(e) => return failure(json, exit_code(e.kind()), &e),
            };
            let ifs = match parse_ifs(&if_) {
                Ok(ifs) => ifs,
                Err(e) => return failure(json, exit_code(e.kind()), &e),
            };
            match set(
                deps,
                &document,
                &sets,
                &ifs,
                Duration::from_secs(lock_timeout),
                cli.namespace.as_deref(),
            ) {
                Ok(document) if json => {
                    success_raw(&raw_object(&[("document", document_json(&document))]))
                }
                Ok(document) => Outcome {
                    code: 0,
                    stdout: document_text(&document),
                    stderr: String::new(),
                },
                Err(e) => failure(json, exit_code(e.kind()), &e),
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
                Ok((result, multi_namespace)) => list_outcome(
                    &result,
                    limit,
                    fields.as_deref(),
                    &where_,
                    ids,
                    json,
                    multi_namespace,
                ),
                Err(e) => failure(json, exit_code(e.kind()), &e),
            }
        }
        Command::Refs {
            document,
            field,
            reverse,
            json,
        } => match refs(
            deps,
            &document,
            reverse,
            field.as_deref(),
            cli.namespace.as_deref(),
        ) {
            Ok((report, _)) if json => success(refs_json(&report)),
            Ok((report, multi_namespace)) => refs_outcome(&report, multi_namespace),
            Err(e) => failure(json, exit_code(e.kind()), &e),
        },
        Command::Toc {
            document,
            depth,
            json,
        } => match toc(deps, &document, cli.namespace.as_deref()) {
            Ok(toc) if json => success(toc_json(&toc, depth)),
            Ok(toc) => toc_outcome(&toc, depth),
            Err(e) => failure(json, exit_code(e.kind()), &e),
        },
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
            // Refused here (SPC-2): `Project::validate`'s `schemas_only` branch would otherwise
            // answer `--schemas` alone and drop `--audit`'s report without a word.
            if schemas && audit {
                return failure_text(
                    json,
                    1,
                    "--schemas and --audit cannot be combined: each describes the whole project in its own way",
                );
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
        Command::Mv {
            from,
            to,
            renumber,
            lock_timeout,
            json,
        } => match (to, renumber) {
            (Some(_), Some(_)) => failure_text(
                json,
                1,
                "mv takes a destination path, or --renumber <namespace>, not both: --renumber \
                 reads one positional argument, not two",
            ),
            (None, None) => failure_text(
                json,
                1,
                "mv needs a destination: a path to move to, or --renumber <namespace>",
            ),
            (Some(to), None) => match mv(
                deps,
                &from,
                &to,
                Duration::from_secs(lock_timeout),
                cli.namespace.as_deref(),
            ) {
                Ok((report, _)) if json => success_raw(&mv_json(&report)),
                Ok((report, multi_namespace)) => Outcome {
                    code: 0,
                    stdout: mv_text(&report, multi_namespace),
                    stderr: String::new(),
                },
                Err(e) => failure(json, exit_code(e.kind()), &e),
            },
            (None, Some(namespace)) => {
                match mv_renumber(
                    deps,
                    &from,
                    &namespace,
                    Duration::from_secs(lock_timeout),
                    cli.namespace.as_deref(),
                ) {
                    Ok((report, _)) if json => success_raw(&mv_json(&report)),
                    Ok((report, multi_namespace)) => Outcome {
                        code: 0,
                        stdout: mv_text(&report, multi_namespace),
                        stderr: String::new(),
                    },
                    Err(e) => failure(json, exit_code(e.kind()), &e),
                }
            }
        },
    }
}

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

/// A `project::` argument gets an unused placeholder: `Project::get`/`toc`/`refs` compute the
/// imported project's scope themselves, and its `namespace:` prefix names one of that project's
/// namespaces, so checking it against this project's would refuse a valid argument.
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

/// Each condition keeps the text the caller wrote, which a false condition's message quotes.
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

/// Returns every match, before `--limit`: `list_outcome` applies it, so `total` and the table's
/// column widths come from the whole match (SPC-5, SPC-12).
fn list(
    deps: &Deps,
    collection: Option<&str>,
    code: Option<&str>,
    wheres: &[String],
    sort: &[String],
    namespace: Option<&str>,
) -> Result<(ListResult, bool), Error> {
    let root = discover(deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let multi_namespace = project.config().namespaces.len() > 1;
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
    let result = project.list_all(&scope, &filter)?;
    Ok((result, multi_namespace))
}

fn split_list(value: Option<&str>) -> Vec<String> {
    value
        .map(|v| v.split(',').map(str::to_owned).collect())
        .unwrap_or_default()
}

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

/// Dangling refs warn on stderr in every shape, `--json` included (SPC-13).
fn list_outcome(
    result: &ListResult,
    limit: Option<usize>,
    fields: Option<&str>,
    wheres: &[String],
    ids: bool,
    json: bool,
    multi_namespace: bool,
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
            stdout.push_str(&identity_text(doc, multi_namespace));
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
        stdout: list_table(matched, listed.len(), &columns, multi_namespace),
        stderr,
    }
}

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

/// A `--where` that fails to parse never reaches here: `list` has already returned its error.
///
/// A `ref.*`/`refby.*` condition adds its ref field (`blocked_by` in
/// `ref.all(blocked_by).status=resolved`), not the field of the document it reaches (`status`).
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

/// Widths are measured over the whole of `matched`, so they do not move with `--limit`. A
/// `--limit 0` prints nothing, header included, as an empty result does. The identity column is
/// `document` when `matched` mixes keyed and unkeyed documents (SPC-5).
fn list_table(
    matched: &[Document],
    listed_len: usize,
    columns: &[String],
    multi_namespace: bool,
) -> String {
    if listed_len == 0 {
        return String::new();
    }
    let column_count = 2 + columns.len();
    let rows: Vec<Vec<String>> = matched
        .iter()
        .map(|doc| table_row(doc, columns, multi_namespace))
        .collect();
    let identity_label = if matched.iter().all(|doc| doc.key.is_some()) {
        "key"
    } else if matched.iter().any(|doc| doc.key.is_some()) {
        "document"
    } else {
        "path"
    };
    let mut header = Vec::with_capacity(column_count);
    header.push(identity_label.to_owned());
    header.push("title".to_owned());
    header.extend(columns.iter().cloned());

    render_table(&header, &rows, listed_len)
}

/// Widths are measured over all of `rows`, not only the first `listed_len`, so they do not move
/// with `--limit` (SPC-5).
fn render_table(header: &[String], rows: &[Vec<String>], listed_len: usize) -> String {
    if listed_len == 0 {
        return String::new();
    }
    let mut widths = vec![0usize; header.len()];
    for row in rows
        .iter()
        .map(Vec::as_slice)
        .chain(std::iter::once(header))
    {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    render_row(&mut out, header, &widths);
    for row in rows.iter().take(listed_len) {
        render_row(&mut out, row, &widths);
    }
    out
}

fn render_row(out: &mut String, row: &[String], widths: &[usize]) {
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
    // An empty last cell would leave the padding before it as trailing whitespace.
    out.push_str(line.trim_end());
    out.push('\n');
}

fn table_row(doc: &Document, columns: &[String], multi_namespace: bool) -> Vec<String> {
    let mut row = vec![identity_text(doc, multi_namespace)];
    row.push(cell_value(doc, "title"));
    for column in columns {
        row.push(cell_value(doc, column));
    }
    row
}

/// Qualified as `namespace:key` only when the project has several namespaces, so a printed name
/// is one every command accepts (SPC-2).
fn identity_text(doc: &Document, multi_namespace: bool) -> String {
    match (&doc.key, &doc.namespace) {
        (Some(key), Some(namespace)) if multi_namespace => format!("{namespace}:{key}"),
        (Some(key), _) => key.clone(),
        _ => doc.path.clone(),
    }
}

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

/// A number prints the value it converts to; `field_text` and `--json` print its digits.
fn render_cell(value: &Value) -> String {
    match value {
        Value::Text(text) | Value::Date(text) | Value::Datetime(text) => text.clone(),
        Value::Empty => String::new(),
        Value::List(items) => items.join(","),
        Value::Number(n) => n.converted(),
        Value::Bool(b) => b.to_string(),
    }
}

/// The labeled block of `get`, `set`, `new` and `mv` (SPC-5). An empty `collection` means `mv`
/// moved the document out of every collection, and then there is no schema either (SPC-2).
fn document_text(document: &Document) -> String {
    let mut out = String::new();
    push_line(&mut out, "path", &document.path);
    if !document.collection.is_empty() {
        push_line(&mut out, "collection", &document.collection);
        push_line(&mut out, "schema", &document.schema);
    }
    if let Some(namespace) = &document.namespace {
        push_line(&mut out, "namespace", namespace);
    }
    if let Some(key) = &document.key {
        push_line(&mut out, "key", key);
    }
    for (name, value) in &document.fields {
        push_line(&mut out, name, &field_text(value));
    }
    out
}

/// `name` is not escaped: the block is labeled for people, and a program reads `--json` (SPC-5).
fn push_line(out: &mut String, name: &str, value: &str) {
    out.push_str(name);
    out.push_str(": ");
    out.push_str(value);
    out.push('\n');
}

/// As `render_cell`, except that a number prints the digits the document holds, as `--json` does
/// (SPC-5).
fn field_text(value: &Value) -> String {
    match value {
        Value::Text(text) | Value::Date(text) | Value::Datetime(text) => text.clone(),
        Value::Empty => String::new(),
        Value::List(items) => items.join(","),
        Value::Number(n) => n.written().to_owned(),
        Value::Bool(b) => b.to_string(),
    }
}

fn refs(
    deps: &Deps,
    document: &std::ffi::OsStr,
    reverse: bool,
    field: Option<&str>,
    namespace: Option<&str>,
) -> Result<(RefsReport, bool), Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let multi_namespace = project.config().namespaces.len() > 1;
    let scope = scope_for(&project, &arg, namespace, deps.env)?;
    let report = project.refs(&arg, &scope, reverse, field, deps.env)?;
    Ok((report, multi_namespace))
}

/// `to` is read as `from` is (SPC-2), and an on-disk `to` against the project `from` found: a
/// destination in another project is one `mv` cannot write.
fn mv(
    deps: &Deps,
    from: &std::ffi::OsStr,
    to: &std::ffi::OsStr,
    lock_timeout: Duration,
    namespace: Option<&str>,
) -> Result<(MvReport, bool), Error> {
    let (root, from_arg) = discover_for(Argument::parse(from)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let multi_namespace = project.config().namespaces.len() > 1;
    let to_arg = match Argument::parse(to)? {
        Argument::Named(document) => document,
        Argument::OnDisk(path) => resolve_on_disk(&root, &path, deps.env)?,
    };
    let scope = scope_for(&project, &from_arg, namespace, deps.env)?;
    let report = project.mv(&from_arg, &to_arg, &scope, lock_timeout, deps)?;
    Ok((report, multi_namespace))
}

/// `namespace` is a bare namespace name, not a document argument, so `Argument::parse` does not
/// read it (SPC-2).
fn mv_renumber(
    deps: &Deps,
    from: &std::ffi::OsStr,
    namespace: &std::ffi::OsStr,
    lock_timeout: Duration,
    namespace_flag: Option<&str>,
) -> Result<(MvReport, bool), Error> {
    let (root, from_arg) = discover_for(Argument::parse(from)?, deps.env)?;
    let project = Project::load(&root, deps.env)?;
    let multi_namespace = project.config().namespaces.len() > 1;
    let namespace_text = match namespace.to_str() {
        Some(text) => text,
        None => {
            return Err(Error::BadArgument(format!(
                "{namespace:?} is not valid UTF-8, so it names no namespace"
            )));
        }
    };
    let scope = scope_for(&project, &from_arg, namespace_flag, deps.env)?;
    let report = project.mv_renumber(&from_arg, namespace_text, &scope, lock_timeout, deps)?;
    Ok((report, multi_namespace))
}

/// The three lines after the block are always printed, so a clean move reads as loud as a busy
/// one (SPC-5).
fn mv_text(report: &MvReport, multi_namespace: bool) -> String {
    let mut out = document_text(&report.document);
    push_line(&mut out, "rewritten", &rewritten_summary(&report.rewritten));
    push_unrewritten_lines(&mut out, &report.unrewritten, multi_namespace);
    push_findings_lines(&mut out, &report.findings);
    out
}

fn rewritten_summary(rewritten: &[RewrittenRef]) -> String {
    let documents: BTreeSet<&str> = rewritten.iter().map(|r| r.document.as_str()).collect();
    format!(
        "{} in {}",
        count_noun(rewritten.len(), "ref"),
        count_noun(documents.len(), "document"),
    )
}

/// The plural adds `s`, which holds for every noun passed in.
fn count_noun(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn push_unrewritten_lines(out: &mut String, unrewritten: &[UnrewrittenRef], multi_namespace: bool) {
    if unrewritten.is_empty() {
        push_line(out, "unrewritten", "none");
        return;
    }
    push_line(out, "unrewritten", &unrewritten.len().to_string());
    for item in unrewritten {
        out.push_str(&unrewritten_text(item, multi_namespace));
        out.push('\n');
    }
}

/// `item.reference.other` is always `Resolved` (`mv_reverse_scan` builds no other), but nothing
/// across the crate boundary enforces it, so the unresolved form is rendered too.
fn unrewritten_text(item: &UnrewrittenRef, multi_namespace: bool) -> String {
    let name = ref_outcome_text(&item.reference.other, multi_namespace);
    format!(
        "{name}  {}  {}",
        item.reference.field, item.reference.written
    )
}

fn ref_outcome_text(outcome: &RefOutcome, multi_namespace: bool) -> String {
    match outcome {
        RefOutcome::Resolved(name) => ref_name_text(name, multi_namespace),
        RefOutcome::Unresolved(reason) => format!("(unresolved: {reason})"),
    }
}

fn ref_name_text(name: &RefName, multi_namespace: bool) -> String {
    let mut out = String::new();
    if let Some(project) = &name.project {
        out.push_str(project);
        out.push_str("::");
    }
    match (&name.namespace, &name.key) {
        (Some(namespace), Some(key)) if multi_namespace => {
            out.push_str(namespace);
            out.push(':');
            out.push_str(key);
        }
        (_, Some(key)) => out.push_str(key),
        _ => out.push_str(&name.path),
    }
    out
}

fn push_findings_lines(out: &mut String, findings: &[Finding]) {
    if findings.is_empty() {
        push_line(out, "findings", "none");
        return;
    }
    out.push_str("findings:\n");
    for finding in findings {
        out.push_str(&finding_text(finding));
        out.push('\n');
    }
}

/// `mv`'s own finding line; `validate` prints its findings as a table instead (`validate_text`).
fn finding_text(finding: &Finding) -> String {
    let mut out = finding.path.clone();
    if let Some(field) = &finding.field {
        out.push('#');
        out.push_str(field);
    }
    out.push_str(": ");
    out.push_str(finding.rule);
    out.push(' ');
    out.push_str(severity_name(finding.level));
    out.push_str(": ");
    out.push_str(&finding.message);
    out
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
    // A config error ends the run here, before a report exists, so `--audit` does not turn it
    // into exit 0 (SPC-2).
    let project = Project::load(&root, deps.env)?;
    project.validate(&args, schemas, strict, audit, namespace, deps.env)
}

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

/// The report is a verdict, printed on standard output whatever it says (SPC-12); `--audit`
/// exits 0 (SPC-2).
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
    } else if report.audit.is_some() {
        audit_text(report)
    } else {
        validate_text(report)
    };
    Outcome {
        code,
        stdout,
        stderr: String::new(),
    }
}

/// The columns are named and ordered as `finding_json`'s fields, with the position folded into
/// `path`. No findings prints nothing: the exit code carries a clean result (SPC-5).
fn validate_text(report: &ValidateReport) -> String {
    let header = vec![
        "path".to_owned(),
        "level".to_owned(),
        "rule".to_owned(),
        "message".to_owned(),
    ];
    let rows: Vec<Vec<String>> = report
        .findings
        .iter()
        .map(|finding| {
            let path = match finding.position {
                Some(Position { line, col }) => format!("{}:{line}:{col}", finding.path),
                None => finding.path.clone(),
            };
            vec![
                path,
                severity_name(finding.level).to_owned(),
                finding.rule.to_owned(),
                finding.message.clone(),
            ]
        })
        .collect();
    let listed_len = rows.len();
    render_table(&header, &rows, listed_len)
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
        // `overlapping` and `not_read` sit beside `unreported`, not in it: both are reported in
        // `findings` (SPC-12).
        summary.insert("overlapping".to_owned(), json!(audit.overlapping.len()));
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

/// Every list keeps the order `AuditReport` gives it (SPC-12, Audit).
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

fn audit_text(report: &ValidateReport) -> String {
    let Some(audit) = &report.audit else {
        return String::new();
    };
    // The summary's account (SPC-12): leaving a count out would show fewer entries than
    // `--json` does.
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

/// A rule's group appears where its first finding does in `findings`, which is already ordered
/// (SPC-12, Order). Its level is the first finding's: `--audit` gives a rule one level per
/// collection.
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
        ErrorKind::AlreadyExists => 7,
    }
}

fn error_object(text: &str, code: u8) -> Map<String, Json> {
    let mut object = Map::new();
    object.insert("error".to_owned(), json!(text));
    object.insert("code".to_owned(), json!(code));
    object.insert("details".to_owned(), json!([]));
    object
}

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

fn within_depth(level: u8, depth: Option<u8>) -> bool {
    depth.is_none_or(|depth| level <= depth)
}

/// A heading's `end` is its end in the whole document, whatever `depth` leaves out (SPC-12).
fn toc_json(toc: &Toc, depth: Option<u8>) -> Json {
    let headings: Vec<Json> = toc
        .headings
        .iter()
        .filter(|heading| within_depth(heading.level, depth))
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

/// A document whose headings are all deeper than `depth` says so on stderr, so it is not taken
/// for one with no headings (SPC-5).
fn toc_outcome(toc: &Toc, depth: Option<u8>) -> Outcome {
    let headings: Vec<&Heading> = toc
        .headings
        .iter()
        .filter(|heading| within_depth(heading.level, depth))
        .collect();
    if let Some(depth) = depth
        && headings.is_empty()
        && !toc.headings.is_empty()
    {
        let deeper = toc.headings.len();
        return Outcome {
            code: 0,
            stdout: String::new(),
            stderr: format!(
                "no headings at depth \u{2264} {depth} ({deeper} {} {} deeper)\n",
                if deeper == 1 { "heading" } else { "headings" },
                if deeper == 1 { "is" } else { "are" },
            ),
        };
    }
    Outcome {
        code: 0,
        stdout: toc_table(&headings),
        stderr: String::new(),
    }
}

fn toc_table(headings: &[&Heading]) -> String {
    if headings.is_empty() {
        return String::new();
    }
    const HEADER: [&str; 4] = ["line", "end", "level", "heading"];
    let rows: Vec<[String; 4]> = headings
        .iter()
        .map(|heading| {
            [
                heading.line.to_string(),
                heading.end.to_string(),
                heading.level.to_string(),
                heading.text.clone(),
            ]
        })
        .collect();
    let mut widths = HEADER.map(str::len);
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    push_toc_row(&mut out, &HEADER.map(str::to_owned), &widths);
    for row in &rows {
        push_toc_row(&mut out, row, &widths);
    }
    out
}

fn push_toc_row(out: &mut String, cells: &[String; 4], widths: &[usize; 4]) {
    let mut line = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            line.push_str("  ");
        }
        line.push_str(cell);
        if i + 1 < cells.len() {
            line.push_str(&" ".repeat(widths[i] - cell.chars().count()));
        }
    }
    out.push_str(line.trim_end());
    out.push('\n');
}

fn refs_outcome(report: &RefsReport, multi_namespace: bool) -> Outcome {
    Outcome {
        code: 0,
        stdout: refs_text(report, multi_namespace),
        stderr: String::new(),
    }
}

/// `written` is a column only for `out`: with `--reverse` it only repeats the document named on
/// the command line (SPC-5).
fn refs_text(report: &RefsReport, multi_namespace: bool) -> String {
    let forward = report.direction == RefsDirection::Out;
    let mut header = vec!["document".to_owned(), "field".to_owned()];
    if forward {
        header.push("written".to_owned());
    }
    let rows: Vec<Vec<String>> = report
        .refs
        .iter()
        .map(|reference| {
            let mut row = vec![
                ref_outcome_text(&reference.other, multi_namespace),
                reference.field.clone(),
            ];
            if forward {
                row.push(reference.written.clone());
            }
            row
        })
        .collect();
    let listed_len = rows.len();
    render_table(&header, &rows, listed_len)
}

fn refs_json(report: &RefsReport) -> Json {
    let refs: Vec<Json> = report
        .refs
        .iter()
        .map(|reference| Json::Object(reference_json(reference)))
        .collect();
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

/// Returns the fields rather than an object, so `unrewritten_json` can add `reason` to the same
/// object (SPC-12).
fn reference_json(reference: &RefsReference) -> Map<String, Json> {
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
    object
}

/// Built as JSON text for the same reason as `document_json` (SPC-12, The write commands).
fn mv_json(report: &MvReport) -> Box<RawValue> {
    let rewritten: Vec<Box<RawValue>> = report.rewritten.iter().map(rewritten_json).collect();
    let unrewritten: Vec<Box<RawValue>> = report.unrewritten.iter().map(unrewritten_json).collect();
    let findings: Vec<Box<RawValue>> = report
        .findings
        .iter()
        .map(|finding| raw(&finding_json(finding)))
        .collect();
    raw_object(&[
        ("document", document_json(&report.document)),
        ("rewritten", raw_array(&rewritten)),
        ("unrewritten", raw_array(&unrewritten)),
        ("findings", raw_array(&findings)),
    ])
}

fn rewritten_json(item: &RewrittenRef) -> Box<RawValue> {
    raw(&json!({
        "document": item.document,
        "field": item.field,
        "before": item.before,
        "after": item.after,
    }))
}

fn unrewritten_json(item: &UnrewrittenRef) -> Box<RawValue> {
    let mut object = reference_json(&item.reference);
    object.insert("reason".to_owned(), json!(reason_name(item.reason)));
    raw(&Json::Object(object))
}

fn reason_name(reason: UnrewrittenReason) -> &'static str {
    match reason {
        UnrewrittenReason::ImportedProject => "imported-project",
        UnrewrittenReason::Mention => "mention",
        UnrewrittenReason::LinksRuleOff => "links-rule-off",
    }
}

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

/// Built as JSON text so that a `number` keeps the digits written in the document (SPC-12): no
/// `serde_json::Number` holds `1e3` as `1e3`. It is either converted to `1000.0` or, with the
/// digits kept, written back with an exponent sign the file never had.
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
    // A collection name is never empty (`config.rs`'s `plain_name` refuses one), so an empty one
    // means `mv` moved the document out of every collection (SPC-2): `""` would name a collection
    // that does not exist.
    if !document.collection.is_empty() {
        object.push(("code", raw(&json!(document.code))));
        object.push(("collection", raw(&json!(document.collection))));
        object.push(("schema", raw(&json!(document.schema))));
    }
    object.push(("fields", raw_object(&fields)));
    raw_object(&object)
}

fn value_json(value: &Value) -> Box<RawValue> {
    match value {
        Value::Text(text) => raw(&json!(text)),
        // Kept apart from a field written as `""` (SPC-12).
        Value::Empty => raw(&json!(null)),
        Value::List(items) => raw(&json!(items)),
        // The one value that does not go through `serde_json::Value`: the digits as the
        // document wrote them, which `Number::read` has already checked are a JSON number.
        Value::Number(number) => raw_text(number.written().to_owned()),
        Value::Bool(flag) => raw(&json!(flag)),
        Value::Date(text) | Value::Datetime(text) => raw(&json!(text)),
    }
}

fn raw(value: &Json) -> Box<RawValue> {
    raw_text(value.to_string())
}

#[expect(
    clippy::expect_used,
    reason = "every caller passes either the printed form of a `serde_json::Value`, or an \
              object or array `joined` has assembled from values that are themselves JSON \
              text, or the digits `Number::read` has parsed as a JSON number"
)]
fn raw_text(text: String) -> Box<RawValue> {
    RawValue::from_string(text).expect("assembled from JSON")
}

fn raw_object(entries: &[(&str, Box<RawValue>)]) -> Box<RawValue> {
    let parts = entries
        .iter()
        .map(|(key, value)| format!("{}:{}", json!(key), value.get()));
    joined('{', parts, '}')
}

fn raw_array(items: &[Box<RawValue>]) -> Box<RawValue> {
    joined('[', items.iter().map(|item| item.get().to_owned()), ']')
}

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

        fn hostname(&self) -> String {
            "fake-host".to_owned()
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
