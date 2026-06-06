pub mod agent;
pub mod config;
pub mod error;
pub mod graph;
pub mod index;
pub mod lakehouse;
pub mod memory;
pub mod node;
#[cfg(feature = "python")]
pub mod python;
pub mod query;
pub mod recall;
pub mod security;
pub mod sync;
#[cfg(feature = "ui")]
pub mod ui;
pub mod wikilink;

pub use agent::{
    render_skill_md, write_mcp_scaffold, write_skill_md, EdgePattern, EdgeSource, GraphSchema,
    McpScaffoldContext, SchemaEdge, SkillRenderContext,
};
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
pub use memory::{new_memory_node, validate_memory_type, MemoryNodeKind, MEMORY_TYPES};
pub use node::{NewNode, Node, NodeMetadata};
pub use query::{changed_since, search_index, GraphChange, QueryOptions, SearchResult};
pub use recall::{recall_bundle, RecallBundle};
pub use security::{assert_path_within_root, validate_http_uri_host, validate_relative_node_path};
pub use sync::{
    GitRepository, PullResult, PushResult, SyncEngine, SyncStatus, WriteLockGuard, WritePolicy,
    DEFAULT_BRANCH, DEFAULT_REMOTE, LOCK_FILE_NAME,
};
pub use wikilink::{extract_wikilink_targets, extract_wikilinks, normalize_wikilink_target};

#[cfg(feature = "ui")]
pub use ui::{parse_loopback_addr, router, run as run_ui_server, AppState, UiServerOptions};
