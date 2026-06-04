pub mod config;
pub mod error;
pub mod graph;
pub mod index;
pub mod lakehouse;
pub mod node;
#[cfg(feature = "python")]
pub mod python;
pub mod sync;
pub mod wikilink;

pub use config::{
    LakehouseCacheConfig, LakehouseConfig, MagGraphConfig, RemoteSource, ResolvedConfig,
    StorageConfig, StorageMode, SyncConfig, SyncRole, METADATA_DIR_NAME,
};
pub use error::{MagGraphError, Result};
pub use graph::{
    resolve_target, traverse, GraphAdjacency, TraversalNode, TraversalOrder, TraversalResult,
};
pub use index::{GraphIndex, NodeIndexEntry};
pub use lakehouse::{
    ContentCache, ContentResolver, FileResolver, HttpResolver, LakehouseReader, NodeWithContent,
    ResolvedContent, ResolverRegistry, S3StubResolver,
};
pub use node::{NewNode, Node, NodeMetadata};
pub use sync::{
    GitRepository, PullResult, PushResult, SyncEngine, SyncStatus, WriteLockGuard, WritePolicy,
    DEFAULT_BRANCH, DEFAULT_REMOTE, LOCK_FILE_NAME,
};
pub use wikilink::{extract_wikilink_targets, extract_wikilinks, normalize_wikilink_target};
