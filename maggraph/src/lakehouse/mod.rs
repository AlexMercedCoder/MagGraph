mod cache;
mod content;
mod resolver;
mod uri;

pub use cache::ContentCache;
pub use content::{AssetMetadata, NodeWithContent, ParquetMetadata, ResolvedContent};
pub use resolver::{
    file_allowlist_from_remotes, ContentResolver, FileResolver, HttpResolver, ResolverRegistry,
    S3StubResolver,
};
pub use uri::{infer_format, resolve_source_uri, ALLOWED_SCHEMES};

use crate::config::{ResolvedConfig, StorageMode};
use crate::error::Result;
use crate::index::GraphIndex;
use crate::node::Node;

/// Reads node content according to `[storage].mode` and lakehouse resolution rules.
pub struct LakehouseReader {
    mode: StorageMode,
    remote_sources: Vec<crate::config::RemoteSource>,
    cache: ContentCache,
    resolvers: ResolverRegistry,
}

impl LakehouseReader {
    pub fn from_config(config: &ResolvedConfig) -> Self {
        let lakehouse = config.config.lakehouse.as_ref();
        let cache_cfg = lakehouse.map(|lh| lh.cache.clone()).unwrap_or_default();

        let remote_uris: Vec<String> = lakehouse
            .map(|lh| lh.remote_sources.iter().map(|s| s.uri.clone()).collect())
            .unwrap_or_default();

        let allowlist = file_allowlist_from_remotes(&remote_uris);
        let resolvers = ResolverRegistry::with_file_allowlist(allowlist);

        Self {
            mode: config.config.storage.mode.clone(),
            remote_sources: lakehouse
                .map(|lh| lh.remote_sources.clone())
                .unwrap_or_default(),
            cache: ContentCache::new(cache_cfg.ttl_secs, cache_cfg.max_bytes),
            resolvers,
        }
    }

    pub fn with_resolvers(
        mode: StorageMode,
        remote_sources: Vec<crate::config::RemoteSource>,
        cache: ContentCache,
        resolvers: ResolverRegistry,
    ) -> Self {
        Self {
            mode,
            remote_sources,
            cache,
            resolvers,
        }
    }

    pub fn cache(&self) -> &ContentCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut ContentCache {
        &mut self.cache
    }

    /// Read a node and resolve external content when in lakehouse mode.
    pub fn read_node(&mut self, index: &GraphIndex, id: &str) -> Result<NodeWithContent> {
        let node = index.read_node(id)?;

        let content = match self.mode {
            StorageMode::Local => ResolvedContent::LocalMarkdown {
                body: node.body.clone(),
            },
            StorageMode::Lakehouse => self.resolve_lakehouse(&node)?,
        };

        Ok(NodeWithContent { node, content })
    }

    fn resolve_lakehouse(&mut self, node: &Node) -> Result<ResolvedContent> {
        let source = node
            .metadata
            .source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let Some(source) = source else {
            return Ok(ResolvedContent::LocalMarkdown {
                body: node.body.clone(),
            });
        };

        let uri = resolve_source_uri(source, &self.remote_sources)?;
        if let Some(cached) = self.cache.get(&uri) {
            return Ok(cached.clone());
        }

        let format = infer_format(&uri, &self.remote_sources);
        let resolved = self.resolvers.fetch(&uri, &format)?;
        self.cache.insert(uri, resolved.clone());
        Ok(resolved)
    }

    /// Fetch external content for a node id without loading from index (testing helper).
    pub fn resolve_source_for_node(&mut self, node: &Node) -> Result<ResolvedContent> {
        match self.mode {
            StorageMode::Local => Ok(ResolvedContent::LocalMarkdown {
                body: node.body.clone(),
            }),
            StorageMode::Lakehouse => self.resolve_lakehouse(node),
        }
    }
}

impl GraphIndex {
    /// Read a node with content resolution driven by storage mode and lakehouse config.
    pub fn read_node_with_content(
        &self,
        reader: &mut LakehouseReader,
        id: &str,
    ) -> Result<NodeWithContent> {
        reader.read_node(self, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::config::MagGraphConfig;

    #[test]
    fn local_mode_returns_markdown_body() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("dir");
        fs::write(
            root.join("note.md"),
            r#"---
id: "note"
type: "note"
---
# Hello
"#,
        )
        .expect("write");

        let config_path = temp.path().join("maggraph.toml");
        fs::write(
            &config_path,
            r#"
[storage]
mode = "local"
root_path = "./graph"
"#,
        )
        .expect("write config");

        let resolved = MagGraphConfig::load(&config_path).expect("load");
        let index = GraphIndex::open(&resolved.root_path).expect("index");
        let mut reader = LakehouseReader::from_config(&resolved);

        let result = reader.read_node(&index, "note").expect("read");
        assert!(matches!(
            result.content,
            ResolvedContent::LocalMarkdown { .. }
        ));
        assert!(result.content.to_markdown().contains("# Hello"));
    }

    #[test]
    fn lakehouse_mode_resolves_mocked_s3() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("dir");
        fs::write(
            root.join("asset.md"),
            r#"---
id: "customer_churn_q2"
type: "external_asset"
source: "s3://lake/churn_data.parquet"
---
# Analysis
"#,
        )
        .expect("write");

        let config_path = temp.path().join("maggraph.toml");
        fs::write(
            &config_path,
            r#"
[storage]
mode = "lakehouse"
root_path = "./graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]
"#,
        )
        .expect("write config");

        let resolved = MagGraphConfig::load(&config_path).expect("load");
        let index = GraphIndex::open(&resolved.root_path).expect("index");
        let mut reader = LakehouseReader::from_config(&resolved);

        let result = reader.read_node(&index, "customer_churn_q2").expect("read");
        assert!(matches!(
            result.content,
            ResolvedContent::ExternalAsset { .. }
        ));
        let md = result.content.to_markdown();
        assert!(md.contains("s3://lake/churn_data.parquet"));
        assert!(md.contains("parquet"));

        // cache hit
        let cached = reader
            .read_node(&index, "customer_churn_q2")
            .expect("cached");
        assert_eq!(cached.content, result.content);
        assert_eq!(reader.cache().len(), 1);
    }

    #[test]
    fn lakehouse_resolves_file_uri() {
        let temp = TempDir::new().expect("temp");
        let data_dir = temp.path().join("data");
        fs::create_dir_all(&data_dir).expect("dir");
        let data_path = data_dir.join("metrics.parquet");
        fs::write(&data_path, b"PAR1demo").expect("write");

        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("dir");

        let file_uri = format!("file://{}", data_path.display());
        let node_md = format!(
            r#"---
id: "metrics"
type: "external_asset"
source: "{file_uri}"
---
Pointer
"#
        );
        fs::write(root.join("metrics.md"), node_md).expect("write node");

        let config_path = temp.path().join("maggraph.toml");
        fs::write(
            &config_path,
            r#"
[storage]
mode = "lakehouse"
root_path = "./graph"

[lakehouse]
remote_sources = [{ uri = "file://PLACEHOLDER", format = "parquet" }]
"#
            .replace(
                "PLACEHOLDER",
                &data_dir
                    .canonicalize()
                    .expect("canon")
                    .display()
                    .to_string(),
            ),
        )
        .expect("write config");

        let resolved = MagGraphConfig::load(&config_path).expect("load");
        let index = GraphIndex::open(&resolved.root_path).expect("index");
        let mut reader = LakehouseReader::from_config(&resolved);

        let result = reader.read_node(&index, "metrics").expect("read");
        if let ResolvedContent::ExternalAsset { metadata, .. } = result.content {
            assert!(metadata.parquet.as_ref().unwrap().magic_valid);
        } else {
            panic!("expected parquet external asset");
        }
    }

    #[test]
    fn relative_source_joins_remote_prefix() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("dir");
        fs::write(
            root.join("rel.md"),
            r#"---
id: "rel"
type: "external_asset"
source: "churn_data.parquet"
---
"#,
        )
        .expect("write");

        let config_path = temp.path().join("maggraph.toml");
        fs::write(
            &config_path,
            r#"
[storage]
mode = "lakehouse"
root_path = "./graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]
"#,
        )
        .expect("write");

        let resolved = MagGraphConfig::load(&config_path).expect("load");
        let index = GraphIndex::open(&resolved.root_path).expect("index");
        let mut reader = LakehouseReader::from_config(&resolved);
        let result = reader.read_node(&index, "rel").expect("read");

        if let ResolvedContent::ExternalAsset { uri, .. } = result.content {
            assert_eq!(uri, "s3://corp-data/lake/churn_data.parquet");
        } else {
            panic!("expected external asset");
        }
    }

    #[test]
    fn source_uri_alias_parses() {
        let raw = r#"---
id: "x"
type: "external_asset"
source_uri: "s3://lake/data.parquet"
---
"#;
        let (metadata, _) =
            crate::node::parse_markdown_node(raw, PathBuf::from("x.md").as_path()).expect("parse");
        assert_eq!(metadata.source.as_deref(), Some("s3://lake/data.parquet"));
    }
}
