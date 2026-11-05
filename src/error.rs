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
    ///
    /// The path identifies the owning input file. When serde decodes a value
    /// captured separately from the document, the source position may be
    /// relative to that value rather than to the full file.
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
    /// A native build tool could not be started.
    #[error("failed to start {program} during {stage}: {source}")]
    NativeSpawn {
        /// Executable selected by the caller or PATH.
        program: String,
        /// Build stage being executed.
        stage: &'static str,
        /// Underlying process I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A native build tool reported an unsupported version.
    #[error("unsupported {program} version during {stage}: expected {expected}, got {actual:?}")]
    NativeVersion {
        /// Executable selected by the caller or PATH.
        program: String,
        /// Build stage being executed.
        stage: &'static str,
        /// Exact version accepted by this release.
        expected: &'static str,
        /// Captured version output.
        actual: String,
    },
    /// A native build tool exited unsuccessfully.
    #[error("{program} failed during {stage} with status {status}: {stderr}")]
    NativeStatus {
        /// Executable selected by the caller or PATH.
        program: String,
        /// Build stage being executed.
        stage: &'static str,
        /// Process exit status.
        status: String,
        /// Captured standard error.
        stderr: String,
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
