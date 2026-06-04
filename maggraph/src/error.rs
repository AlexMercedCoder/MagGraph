use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MagGraphError {
    #[error("failed to read config at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse config at {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("config validation failed: {0}")]
    ConfigValidation(String),

    #[error("failed to initialize graph root at {path}: {source}")]
    GraphRootInit {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read node at {path}: {source}")]
    NodeRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write node at {path}: {source}")]
    NodeWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to delete node at {path}: {source}")]
    NodeDelete {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("node parse error at {path}: {message}")]
    NodeParse { path: PathBuf, message: String },

    #[error("node {id} not found")]
    NodeNotFound { id: String },

    #[error("node {id} already exists at {path}")]
    NodeAlreadyExists { id: String, path: PathBuf },

    #[error("duplicate node id {id}: {first} and {second}")]
    DuplicateNodeId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },

    #[error("graph index error: {0}")]
    Index(String),

    #[error("lakehouse error: {0}")]
    Lakehouse(String),

    #[error("content resolve failed for {uri}: {message}")]
    ContentResolve { uri: String, message: String },

    #[error("content scheme {scheme} is not allowed")]
    DisallowedScheme { scheme: String },

    #[error("node {id} has no external source to resolve")]
    MissingSource { id: String },

    #[error("[sync] is not configured")]
    SyncNotConfigured,

    #[error("git error: {message}")]
    Git { message: String },

    #[error("write lock required for leader mutations; acquire lock before writing")]
    WriteLockRequired,

    #[error("write lock held by {holder} since {acquired_at}")]
    WriteLockHeld { holder: String, acquired_at: String },

    #[error("write lock error at {path}: {message}")]
    WriteLock { path: PathBuf, message: String },

    #[error("read-only role `{role}` cannot perform write operations")]
    ReadOnlyRole { role: String },
}

pub type Result<T> = std::result::Result<T, MagGraphError>;
