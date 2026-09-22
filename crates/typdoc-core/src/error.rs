use std::path::PathBuf;

/// What a caller can do differently after a failure; the binary maps each to an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    BadArguments,
    Validation,
    NotFound,
    Io,
    LockTimeout,
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
}

impl Error {
    /// For `map_err` on a failed read of `file`.
    pub fn io_at(file: &std::path::Path) -> impl FnOnce(std::io::Error) -> Error {
        let file = file.to_owned();
        move |source| Error::Io { file, source }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Error::BadArgument(_) | Error::AmbiguousKey { .. } => ErrorKind::BadArguments,
            Error::NoProject { .. } | Error::NoProjectAt { .. } | Error::NotFound { .. } => {
                ErrorKind::NotFound
            }
            Error::Config { .. } | Error::ConfigErrors { .. } | Error::Frontmatter { .. } => {
                ErrorKind::Validation
            }
            Error::Io { .. } => ErrorKind::Io,
            Error::LockTimeout { .. } => ErrorKind::LockTimeout,
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
