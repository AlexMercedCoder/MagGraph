pub mod config;
pub mod error;

pub use config::{
    LakehouseConfig, MagGraphConfig, RemoteSource, ResolvedConfig, StorageConfig, StorageMode,
    SyncConfig, SyncRole, METADATA_DIR_NAME,
};
pub use error::{MagGraphError, Result};
