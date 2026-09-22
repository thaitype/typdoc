use std::path::PathBuf;

use crate::validate::Finding;

/// What a caller can do differently after a failure; the binary maps each to an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    BadArguments,
    Validation,
    NotFound,
    Io,
    LockTimeout,
    /// A `set --if` condition was false; nothing was written (design, Exit codes: "3 | An
    /// `--if` condition was false; nothing written").
    IfFalse,
    /// A write's destination is already there; nothing was written (design, Exit codes: "7 |
    /// The destination already exists").
    Exists,
}

/// One config error: the id from the design's table, the configuration file it is about
/// relative to the project folder, and a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub id: &'static str,
    pub path: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    BadArgument(String),

    #[error("no project found: there is no .typdoc/config.json in {from} or above it")]
    NoProject { from: PathBuf },

    #[error("no project found: {dir} has no .typdoc/config.json")]
    NoProjectAt { dir: PathBuf },

    #[error("no document at {path}{}", not_found_hint(*hint, path))]
    NotFound { path: String, hint: bool },

    /// A key that names a document in more than one namespace in scope.
    #[error("`{key}` is a key in more than one namespace: {}", candidates.join(", "))]
    AmbiguousKey {
        key: String,
        candidates: Vec<String>,
    },

    /// Every config error that could be determined, and whether that is all of them.
    #[error("{}", summary(errors))]
    ConfigErrors {
        errors: Vec<ConfigError>,
        complete: bool,
    },

    /// A fault in the configuration that has no id yet.
    #[error("{}: {message}", file.display())]
    Config { file: PathBuf, message: String },

    #[error("{}: {message}", file.display())]
    Frontmatter { file: PathBuf, message: String },

    #[error("{}: {source}", file.display())]
    Io {
        file: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A namespace or project lock not acquired within `--lock-timeout`. `message` names the
    /// path, pid, host and age, built where the lock was attempted, which is the one place
    /// that knows enough about the failed attempt and the competing lock to say it.
    #[error("{message}")]
    LockTimeout { path: PathBuf, message: String },

    /// A write command's own validation refused the write before anything was written: a value
    /// that does not fit its type, an enum value not in the schema, a transition `transitions`
    /// does not allow, a ref that does not resolve, or a field the schema marks `auto` given
    /// directly. `findings` is never empty; every problem found is reported, not only the first
    /// (the design's error object: "`details` holds findings").
    #[error("{}", finding_summary(findings))]
    Invalid { findings: Vec<Finding> },

    /// A `set --if` condition was false; nothing was written. `findings` names each condition
    /// that failed, never empty.
    #[error("{}", finding_summary(findings))]
    IfFalse { findings: Vec<Finding> },

    /// `new`'s scope holds more than one namespace, and it writes into exactly one (design,
    /// `typdoc new`: "if the scope holds more than one, it exits 1 with the choices"). Not
    /// `AmbiguousKey`: no key was given here, only namespaces to choose among.
    #[error("the scope holds more than one namespace: {}", candidates.join(", "))]
    AmbiguousScope { candidates: Vec<String> },

    /// A write's destination is already there, and nothing was written (decision 15): a path
    /// given on the command line, or a name a `match` template produced, refused alike, checked
    /// under the namespace's lock and enforced by the file system (`O_EXCL`) rather than
    /// typdoc remembering to look first.
    #[error("{message}")]
    Exists { path: PathBuf, message: String },
}

impl Error {
    /// For `map_err` on a failed read of `file`.
    pub fn io_at(file: &std::path::Path) -> impl FnOnce(std::io::Error) -> Error {
        let file = file.to_owned();
        move |source| Error::Io { file, source }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Error::BadArgument(_) | Error::AmbiguousKey { .. } | Error::AmbiguousScope { .. } => {
                ErrorKind::BadArguments
            }
            Error::NoProject { .. } | Error::NoProjectAt { .. } | Error::NotFound { .. } => {
                ErrorKind::NotFound
            }
            Error::Config { .. }
            | Error::ConfigErrors { .. }
            | Error::Frontmatter { .. }
            | Error::Invalid { .. } => ErrorKind::Validation,
            Error::Io { .. } => ErrorKind::Io,
            Error::LockTimeout { .. } => ErrorKind::LockTimeout,
            Error::IfFalse { .. } => ErrorKind::IfFalse,
            Error::Exists { .. } => ErrorKind::Exists,
        }
    }
}

/// The suggestion added to a "not found" message when a same-named file sits in the current
/// directory: never a substitution, only a pointer at what might have been meant.
fn not_found_hint(hint: bool, path: &str) -> String {
    if hint {
        format!(": `./{path}` exists")
    } else {
        String::new()
    }
}

fn summary(errors: &[ConfigError]) -> String {
    let Some(first) = errors.first() else {
        return "the config is not valid".to_owned();
    };
    let first = format!("{}: {}", first.path, first.message);
    match errors.len() {
        1 => first,
        n => format!("{n} config errors, the first is {first}"),
    }
}

/// `Error::Invalid` and `Error::IfFalse` share this: the top-level `error` string is the one
/// finding's own message when there is one (design's own error object example, `frontmatter.
/// transitions`: `"error"` is exactly the finding's `message`, not a wrapping phrase), and a
/// count with the first message otherwise, the same shape `summary` above already gives
/// `ConfigErrors`.
fn finding_summary(findings: &[Finding]) -> String {
    let Some(first) = findings.first() else {
        return "nothing to report".to_owned();
    };
    match findings.len() {
        1 => first.message.clone(),
        n => format!("{n} problems, the first is {}", first.message),
    }
}
