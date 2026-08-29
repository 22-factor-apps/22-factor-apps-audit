use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("refusing to overwrite existing output {0}; pass --force to replace it")]
    OutputExists(PathBuf),

    #[error("invalid JSON in {context}: {source}")]
    Json {
        context: String,
        source: serde_json::Error,
    },

    #[error("GitHub request failed for {url}: {message}")]
    Github { url: String, message: String },

    #[error("catalog request failed for {url}: {message}")]
    Catalog { url: String, message: String },

    #[error("{0}")]
    InvalidAssessment(String),

    #[error("git command failed: {0}")]
    Git(String),
}

pub type Result<T> = std::result::Result<T, AuditError>;
