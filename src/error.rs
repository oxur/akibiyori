use std::path::PathBuf;

/// Failures reported by configuration loading and documentation generation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An input file could not be read.
    #[error("failed to read {path}: {source}")]
    Read {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// An input file contains invalid JSON.
    #[error("invalid JSON in {path}: {source}")]
    Json {
        /// File that could not be parsed.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// Input or output violates the documented generator contract.
    #[error("{0}")]
    Invalid(String),
    /// Writing generated output failed.
    #[error("failed to write {path}: {source}")]
    Write {
        /// File or directory operation that failed.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    pub(crate) fn read(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Read {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn write(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }
}
