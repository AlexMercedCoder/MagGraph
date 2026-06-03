pub mod config;
pub mod error;
pub mod graph;
pub mod index;
pub mod node;
pub mod wikilink;

pub use config::{
    LakehouseConfig, MagGraphConfig, RemoteSource, ResolvedConfig, StorageConfig, StorageMode,
    SyncConfig, SyncRole, METADATA_DIR_NAME,
};
pub use error::{MagGraphError, Result};
pub use graph::{
    resolve_target, traverse, GraphAdjacency, TraversalNode, TraversalOrder, TraversalResult,
};
pub use index::{GraphIndex, NodeIndexEntry};
pub use node::{NewNode, Node, NodeMetadata};
pub use wikilink::{extract_wikilink_targets, extract_wikilinks, normalize_wikilink_target};
