use std::ffi::OsString;

use clap::error::ErrorKind as ClapKind;
use clap::{Parser, Subcommand};
use serde_json::{Map, Value as Json, json};
use typdoc_core::{Deps, Document, DocumentArg, Error, ErrorKind, Project, Value};

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
                Ok(document) => Outcome {
                    code: 0,
                    stdout: format!("{}\n", json!({ "document": document_json(&document) })),
                    stderr: String::new(),
                },
                Err(e) => failure(true, exit_code(e.kind()), &e),
            }
        }
    }
}

fn get(
    deps: &Deps,
    document: &std::ffi::OsStr,
    namespace: Option<&str>,
) -> Result<Document, Error> {
    let arg = DocumentArg::parse(document)?;
    let root = typdoc_core::discover(deps.env)?;
    let project = Project::load(&root)?;
    project.scope(None, namespace, deps.env)?;
    project.get(&arg)
}

fn exit_code(kind: ErrorKind) -> u8 {
    match kind {
        ErrorKind::BadArguments => 1,
        ErrorKind::Validation => 2,
        ErrorKind::NotFound => 5,
        ErrorKind::Io => 6,
    }
}

/// The error object on standard error with `--json`, and one line of text without.
fn failure_text(json: bool, code: u8, text: &str) -> Outcome {
    let stderr = if json {
        format!(
            "{}\n",
            json!({ "error": text, "code": code, "details": [] })
        )
    } else {
        format!("typdoc: {text}\n")
    };
    Outcome {
        code,
        stdout: String::new(),
        stderr,
    }
}

/// The error object of a failure that ended a command, with the config errors as `details`.
fn failure(json: bool, code: u8, error: &Error) -> Outcome {
    let Error::ConfigErrors { errors, complete } = error else {
        return failure_text(json, code, &error.to_string());
    };
    let details: Vec<Json> = errors
        .iter()
        .map(|e| json!({ "level": "error", "rule": e.id, "message": e.message, "path": e.path }))
        .collect();
    let object = json!({
        "error": error.to_string(),
        "code": code,
        "details": details,
        "complete": complete,
    });
    Outcome {
        code,
        stdout: String::new(),
        stderr: if json {
            format!("{object}\n")
        } else {
            format!("typdoc: {error}\n")
        },
    }
}

fn document_json(document: &Document) -> Json {
    let fields: Map<String, Json> = document
        .fields
        .iter()
        .map(|(name, value)| (name.clone(), value_json(value)))
        .collect();
    json!({
        "path": document.path,
        "namespace": document.namespace,
        "collection": document.collection,
        "schema": document.schema,
        "fields": fields,
    })
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::Text(text) => json!(text),
        Value::List(items) => json!(items),
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
