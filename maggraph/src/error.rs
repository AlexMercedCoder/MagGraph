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
}

pub type Result<T> = std::result::Result<T, MagGraphError>;
