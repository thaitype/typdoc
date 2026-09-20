use std::ffi::OsString;
use std::path::PathBuf;

use clap::error::ErrorKind as ClapKind;
use clap::{Parser, Subcommand};
use serde_json::{Map, Value as Json, json};
use typdoc_core::{
    Argument, Deps, Document, DocumentArg, Env, Error, ErrorKind, Finding, Project, RefOutcome,
    RefsDirection, RefsReference, RefsReport, Severity, Toc, ValidateReport, ValidateScope, Value,
    discover, discover_for, resolve_on_disk,
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
    /// Read one document's frontmatter
    Get {
        /// The path of the document, from the project folder
        document: OsString,
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
        /// The path of the document, from the project folder
        document: OsString,
        /// List only the headings down to this level, 1 to 6
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(1..))]
        depth: Option<u8>,
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
        Command::Get { document, json } => {
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            match get(deps, &document, cli.namespace.as_deref()) {
                Ok(document) => success(json!({ "document": document_json(&document) })),
                Err(e) => failure(true, exit_code(e.kind()), &e),
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
            if !json {
                return failure_text(false, 1, "the output without --json is not built yet");
            }
            if (schemas || audit) && !documents.is_empty() {
                let flag = if schemas { "--schemas" } else { "--audit" };
                return failure_text(
                    true,
                    1,
                    &format!(
                        "{flag} describes the whole project and cannot be combined with arguments"
                    ),
                );
            }
            // `--audit` answers a different question from plain `validate` (design, "Audit
            // mode": `summary.audit`, `summary.unreported`, the `audit` object, rules at `off`
            // shown as `info`, exit 0 unless the config itself is invalid), and none of that is
            // built yet (ticket 19). Refusing it here is the same choice already made for
            // `--json`'s absence and for a `project::` prefix: a flag or form the binary does
            // not answer yet is refused plainly, not run as something else with no word said.
            if audit {
                return failure_text(true, 1, "--audit is not built yet");
            }
            match validate(deps, &documents, schemas, strict, cli.namespace.as_deref()) {
                Ok(report) => validate_outcome(&report),
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
    }
}

fn success(result: Json) -> Outcome {
    Outcome {
        code: 0,
        stdout: format!("{result}\n"),
        stderr: String::new(),
    }
}

fn get(
    deps: &Deps,
    document: &std::ffi::OsStr,
    namespace: Option<&str>,
) -> Result<Document, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root)?;
    let scope = project.scope(arg.namespace_prefix(), namespace, deps.env)?;
    project.get(&arg, &scope, deps.env)
}

fn toc(deps: &Deps, document: &std::ffi::OsStr, namespace: Option<&str>) -> Result<Toc, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root)?;
    let scope = project.scope(arg.namespace_prefix(), namespace, deps.env)?;
    project.toc(&arg, &scope, deps.env)
}

fn refs(
    deps: &Deps,
    document: &std::ffi::OsStr,
    reverse: bool,
    field: Option<&str>,
    namespace: Option<&str>,
) -> Result<RefsReport, Error> {
    let (root, arg) = discover_for(Argument::parse(document)?, deps.env)?;
    let project = Project::load(&root)?;
    let scope = project.scope(arg.namespace_prefix(), namespace, deps.env)?;
    project.refs(&arg, &scope, reverse, field, deps.env)
}

fn validate(
    deps: &Deps,
    documents: &[OsString],
    schemas: bool,
    strict: bool,
    namespace: Option<&str>,
) -> Result<ValidateReport, Error> {
    let (root, args) = validate_args(deps.env, documents)?;
    // A config error stops here today (`Error::ConfigErrors`, see `config::Report`'s own
    // comment on why), before `project.validate` below ever runs: nothing reaches it as a
    // finding yet. This `?` is where a config error that answered the design's question on its
    // own, "does it make checking impossible?", with no, would instead let the project load and
    // reach `validate`'s report.
    let project = Project::load(&root)?;
    project.validate(&args, schemas, strict, namespace, deps.env)
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
/// (design, JSON output), with exit 2 when a finding is an error and 0 otherwise.
fn validate_outcome(report: &ValidateReport) -> Outcome {
    let code = if report.findings.iter().any(|f| f.level == Severity::Error) {
        2
    } else {
        0
    };
    Outcome {
        code,
        stdout: format!("{}\n", validate_json(report)),
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
    let (error, warn) = report
        .findings
        .iter()
        .fold((0u32, 0u32), |(error, warn), finding| match finding.level {
            Severity::Error => (error + 1, warn),
            Severity::Warn => (error, warn + 1),
        });
    let mut summary = Map::new();
    summary.insert("scope".to_owned(), json!(scope_name(report.scope)));
    summary.insert("strict".to_owned(), json!(report.strict));
    summary.insert("checked".to_owned(), Json::Object(checked));
    summary.insert(
        "findings".to_owned(),
        json!({ "error": error, "warn": warn, "info": 0 }),
    );
    let findings: Vec<Json> = report.findings.iter().map(finding_json).collect();
    json!({ "summary": Json::Object(summary), "findings": findings })
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
    }
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
        ErrorKind::NotFound => 5,
        ErrorKind::Io => 6,
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
        Error::AmbiguousKey { candidates, .. } => {
            object.insert("candidates".to_owned(), json!(candidates));
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
        "document": Json::Object(document_name(&toc.path, &toc.namespace, toc.key.as_deref())),
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
            &report.document.namespace,
            report.document.key.as_deref(),
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
            object.insert("namespace".to_owned(), json!(name.namespace));
            if let Some(key) = &name.key {
                object.insert("key".to_owned(), json!(key));
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

/// The name of a document: `path` and `namespace` always, `key` only for a coded document.
fn document_name(path: &str, namespace: &str, key: Option<&str>) -> Map<String, Json> {
    let mut object = Map::new();
    object.insert("path".to_owned(), json!(path));
    object.insert("namespace".to_owned(), json!(namespace));
    if let Some(key) = key {
        object.insert("key".to_owned(), json!(key));
    }
    object
}

fn document_json(document: &Document) -> Json {
    let mut object = document_name(&document.path, &document.namespace, document.key.as_deref());
    object.insert("code".to_owned(), json!(document.code));
    object.insert("collection".to_owned(), json!(document.collection));
    object.insert("schema".to_owned(), json!(document.schema));
    let fields: Map<String, Json> = document
        .fields
        .iter()
        .map(|(name, value)| (name.clone(), value_json(value)))
        .collect();
    object.insert("fields".to_owned(), Json::Object(fields));
    Json::Object(object)
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::Text(text) => json!(text),
        Value::List(items) => json!(items),
        Value::Number(number) => Json::Number(number.clone()),
        Value::Bool(flag) => json!(flag),
        Value::Date(text) | Value::Datetime(text) => json!(text),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io;
    use std::path::PathBuf;

    use serde_json::json;
    use typdoc_core::{Deps, Env};

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
    }

    fn fixtures() -> PathBuf {
        typdoc_testkit::fixtures::path("")
    }

    fn get_note(env: &FakeEnv) -> super::Outcome {
        let args: Vec<OsString> = ["typdoc", "get", "note.md", "--json"]
            .iter()
            .map(OsString::from)
            .collect();
        run(&args, &Deps { env })
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
