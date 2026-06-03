use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    Local,
    Lakehouse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncRole {
    Leader,
    Follower,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    pub mode: StorageMode,
    pub root_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSource {
    pub uri: String,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LakehouseConfig {
    pub remote_sources: Vec<RemoteSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConfig {
    pub role: SyncRole,
    pub remote_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MagGraphConfig {
    pub storage: StorageConfig,
    #[serde(default)]
    pub lakehouse: Option<LakehouseConfig>,
    #[serde(default)]
    pub sync: Option<SyncConfig>,
}

/// Configuration with filesystem paths resolved relative to the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub config: MagGraphConfig,
    pub config_path: PathBuf,
    pub root_path: PathBuf,
}
