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
    #[serde(default)]
    pub cache: LakehouseCacheConfig,
}

/// Cache policy for resolved external content (Phase 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LakehouseCacheConfig {
    /// Time-to-live for cached fetches, in seconds. `0` disables TTL expiry.
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    /// Maximum total bytes stored in the cache. `0` means no size cap.
    #[serde(default = "default_cache_max_bytes")]
    pub max_bytes: usize,
}

impl Default for LakehouseCacheConfig {
    fn default() -> Self {
        Self {
            ttl_secs: default_cache_ttl_secs(),
            max_bytes: default_cache_max_bytes(),
        }
    }
}

fn default_cache_ttl_secs() -> u64 {
    300
}

fn default_cache_max_bytes() -> usize {
    10 * 1024 * 1024
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
