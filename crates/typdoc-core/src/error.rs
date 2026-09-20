use std::path::PathBuf;

/// What a caller can do differently after a failure; the binary maps each to an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    BadArguments,
    Validation,
    NotFound,
    Io,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    BadArgument(String),

    #[error("no project found: there is no .typdoc/config.json in {from} or above it")]
    NoProject { from: PathBuf },

    #[error("no project found: {dir} has no .typdoc/config.json")]
    NoProjectAt { dir: PathBuf },

    #[error("no document at {path}")]
    NotFound { path: String },

    #[error("{}: {message}", file.display())]
    Config { file: PathBuf, message: String },

    #[error("{}: {message}", file.display())]
    Frontmatter { file: PathBuf, message: String },

    #[error("{}: {message}", file.display())]
    Unreadable { file: PathBuf, message: String },

    #[error("{}: {source}", file.display())]
    Io {
        file: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    /// For `map_err` on a failed read of `file`.
    pub fn io_at(file: &std::path::Path) -> impl FnOnce(std::io::Error) -> Error {
        let file = file.to_owned();
        move |source| Error::Io { file, source }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Error::BadArgument(_) => ErrorKind::BadArguments,
            Error::NoProject { .. } | Error::NoProjectAt { .. } | Error::NotFound { .. } => {
                ErrorKind::NotFound
            }
            Error::Config { .. } | Error::Frontmatter { .. } => ErrorKind::Validation,
            Error::Unreadable { .. } | Error::Io { .. } => ErrorKind::Io,
        }
    }
}
