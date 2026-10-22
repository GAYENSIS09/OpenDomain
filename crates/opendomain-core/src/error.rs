//! Error types for the OpenDomain core library.

use std::path::PathBuf;

/// Errors produced while loading, validating or querying an OpenDomain workspace.
#[derive(Debug, thiserror::Error)]
pub enum OdError {
    /// The `opendomain/` directory could not be found.
    #[error("no `opendomain/` directory found in {0} or any parent — run `opendomain init` first")]
    WorkspaceNotFound(PathBuf),

    /// A workspace was found but is missing required top-level entries.
    #[error("malformed workspace at {path}: {reason}")]
    Malformed { path: PathBuf, reason: String },

    /// `opendomain.toml` is absent.
    #[error("missing `opendomain.toml` in {0} — run `opendomain init` or `opendomain update`")]
    MissingConfig(PathBuf),

    /// `opendomain.toml` could not be parsed.
    #[error("invalid `opendomain.toml`: {0}")]
    Config(#[from] toml::de::Error),

    /// `opendomain.toml` failed validation.
    #[error("invalid `opendomain.toml`: {0}")]
    ConfigInvalid(String),

    /// An identifier could not be parsed.
    #[error("`{0}` is not a valid OpenDomain identifier (expected e.g. `REQ-001` or `REQ-INVENTORY-012`)")]
    BadId(String),

    /// An unknown identifier prefix was used.
    #[error("unknown id prefix `{prefix}` (known: {known})")]
    UnknownPrefix { prefix: String, known: String },

    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A file could not be written.
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A markdown artifact was structurally invalid.
    #[error("{path}: {reason}")]
    Artifact { path: PathBuf, reason: String },

    /// The referenced artifact does not exist in the workspace.
    #[error("unknown artifact `{0}`")]
    UnknownArtifact(String),

    /// The user asked for something the current stage does not allow.
    #[error("{0}")]
    StageViolation(String),

    /// A quality gate refused to pass.
    #[error("gate {gate} failed: {blocking} blocking, {advisory} advisory finding(s)")]
    GateFailed {
        gate: &'static str,
        blocking: usize,
        advisory: usize,
    },

    /// Generic bad input from the caller.
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, OdError>;

impl OdError {
    /// Convenience constructor for [`OdError::Read`].
    pub fn read(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        OdError::Read {
            path: path.into(),
            source,
        }
    }

    /// Convenience constructor for [`OdError::Write`].
    pub fn write(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        OdError::Write {
            path: path.into(),
            source,
        }
    }

    /// Convenience constructor for [`OdError::Artifact`].
    pub fn artifact(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        OdError::Artifact {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Convenience constructor for [`OdError::Invalid`].
    pub fn invalid(reason: impl Into<String>) -> Self {
        OdError::Invalid(reason.into())
    }
}
