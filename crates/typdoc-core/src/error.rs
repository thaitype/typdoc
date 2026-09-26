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
    /// A `set --if` condition was false; nothing was written.
    IfFalse,
    AlreadyExists,
}

/// One config error: its id (SPC-6), the configuration file it is about relative to the
/// project folder, and a message.
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

    /// A fault in the configuration that has no config error id of its own.
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
    /// path, pid, host and age; it is built where the lock was attempted, the one place that
    /// knows both the attempt and the competing lock.
    #[error("{message}")]
    LockTimeout { path: PathBuf, message: String },

    /// A write command's own validation refused the write before anything was written.
    /// `findings` is never empty and holds every problem found, not only the first (SPC-3).
    #[error("{}", finding_summary(findings))]
    Invalid { findings: Vec<Finding> },

    /// A `set --if` condition was false; nothing was written. `findings` names each condition
    /// that failed, never empty.
    #[error("{}", finding_summary(findings))]
    IfFalse { findings: Vec<Finding> },

    /// Raised by `new`, which writes into exactly one namespace. Not `AmbiguousKey`: no key was
    /// given, only namespaces to choose among.
    #[error("the scope holds more than one namespace: {}", candidates.join(", "))]
    AmbiguousScope { candidates: Vec<String> },

    /// The destination of a write already exists, and nothing was written: a destination that is
    /// the source file itself, a path given on the command line, or a name a `match` template
    /// produced, refused alike (SPC-2, SPC-10). `message` is built by the caller, which knows
    /// which of these it is.
    #[error("{message}")]
    AlreadyExists { path: String, message: String },
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
            Error::AlreadyExists { .. } => ErrorKind::AlreadyExists,
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

/// The top-level `error` of `Error::Invalid` and `Error::IfFalse`: the one finding's own
/// message, not a wrapping phrase (SPC-3), or a count with the first message.
fn finding_summary(findings: &[Finding]) -> String {
    let Some(first) = findings.first() else {
        return "nothing to report".to_owned();
    };
    match findings.len() {
        1 => first.message.clone(),
        n => format!("{n} problems, the first is {}", first.message),
    }
}
