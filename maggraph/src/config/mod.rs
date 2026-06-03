mod schema;

pub use schema::{
    LakehouseCacheConfig, LakehouseConfig, MagGraphConfig, RemoteSource, ResolvedConfig,
    StorageConfig, StorageMode, SyncConfig, SyncRole,
};

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{MagGraphError, Result};

/// Name of the optional metadata directory created under the graph root.
pub const METADATA_DIR_NAME: &str = ".maggraph";

impl MagGraphConfig {
    /// Load and validate configuration from a TOML file.
    ///
    /// `root_path` is resolved relative to the config file's parent directory.
    pub fn load(path: impl AsRef<Path>) -> Result<ResolvedConfig> {
        let path = path.as_ref().to_path_buf();
        let raw = fs::read_to_string(&path).map_err(|source| MagGraphError::ConfigRead {
            path: path.clone(),
            source,
        })?;

        let config: MagGraphConfig =
            toml::from_str(&raw).map_err(|source| MagGraphError::ConfigParse {
                path: path.clone(),
                source,
            })?;

        config.into_resolved(&path)
    }

    /// Validate fields and resolve `root_path` against the config file location.
    pub fn into_resolved(self, config_path: &Path) -> Result<ResolvedConfig> {
        self.validate()?;

        let config_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let root_path = resolve_path(&config_dir, &self.storage.root_path);

        Ok(ResolvedConfig {
            config: self,
            config_path: config_path.to_path_buf(),
            root_path,
        })
    }

    fn validate(&self) -> Result<()> {
        if self.storage.root_path.as_os_str().is_empty() {
            return Err(MagGraphError::ConfigValidation(
                "[storage].root_path must not be empty".into(),
            ));
        }

        if self.storage.mode == StorageMode::Lakehouse && self.lakehouse.is_none() {
            return Err(MagGraphError::ConfigValidation(
                "[lakehouse] section is required when [storage].mode = \"lakehouse\"".into(),
            ));
        }

        if let Some(lakehouse) = &self.lakehouse {
            if lakehouse.remote_sources.is_empty() {
                return Err(MagGraphError::ConfigValidation(
                    "[lakehouse].remote_sources must contain at least one entry".into(),
                ));
            }

            for (index, source) in lakehouse.remote_sources.iter().enumerate() {
                validate_remote_source(source, index)?;
            }
        }

        if let Some(sync) = &self.sync {
            if sync.remote_url.trim().is_empty() {
                return Err(MagGraphError::ConfigValidation(
                    "[sync].remote_url must not be empty when [sync] is present".into(),
                ));
            }
        }

        Ok(())
    }
}

impl ResolvedConfig {
    /// Ensure the graph root exists and optionally create the metadata directory.
    pub fn initialize_graph_root(&self, create_metadata_dir: bool) -> Result<()> {
        ensure_directory(&self.root_path)?;

        if create_metadata_dir {
            ensure_directory(&self.root_path.join(METADATA_DIR_NAME))?;
        }

        Ok(())
    }

    /// Return whether the graph root currently exists on disk.
    pub fn root_exists(&self) -> bool {
        self.root_path.is_dir()
    }
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn validate_remote_source(source: &RemoteSource, index: usize) -> Result<()> {
    if source.uri.trim().is_empty() {
        return Err(MagGraphError::ConfigValidation(format!(
            "[lakehouse].remote_sources[{index}].uri must not be empty"
        )));
    }

    if source.format.trim().is_empty() {
        return Err(MagGraphError::ConfigValidation(format!(
            "[lakehouse].remote_sources[{index}].format must not be empty"
        )));
    }

    Ok(())
}

fn ensure_directory(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            return Err(MagGraphError::GraphRootInit {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "path exists but is not a directory",
                ),
            });
        }
        return Ok(());
    }

    fs::create_dir_all(path).map_err(|source| MagGraphError::GraphRootInit {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tempfile::TempDir;

    fn write_config(dir: &Path, contents: &str) -> PathBuf {
        let path = dir.join("maggraph.toml");
        fs::write(&path, contents).expect("write config");
        path
    }

    #[test]
    fn loads_valid_local_config_and_resolves_root_path() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "local"
root_path = "./knowledge_graph"
"#,
        );

        let resolved = MagGraphConfig::load(&config_path).expect("load config");

        assert_eq!(resolved.config.storage.mode, StorageMode::Local);
        assert_eq!(resolved.root_path, temp.path().join("knowledge_graph"));
    }

    #[test]
    fn loads_prd_example_config() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]

[sync]
role = "follower"
remote_url = "git@github.com:org/maggraph-sync.git"
"#,
        );

        let resolved = MagGraphConfig::load(&config_path).expect("load config");

        assert_eq!(resolved.config.storage.mode, StorageMode::Lakehouse);
        assert_eq!(
            resolved.config.sync.as_ref().unwrap().role,
            SyncRole::Follower
        );
        assert_eq!(resolved.root_path, temp.path().join("knowledge_graph"));
    }

    #[test]
    fn rejects_lakehouse_mode_without_lakehouse_section() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"
"#,
        );

        let err = MagGraphConfig::load(&config_path).expect_err("expected validation error");
        assert!(matches!(err, MagGraphError::ConfigValidation(_)));
    }

    #[test]
    fn rejects_invalid_storage_mode() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "cloud"
root_path = "./knowledge_graph"
"#,
        );

        let err = MagGraphConfig::load(&config_path).expect_err("expected parse error");
        assert!(matches!(err, MagGraphError::ConfigParse { .. }));
    }

    #[test]
    fn rejects_empty_sync_remote_url() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "local"
root_path = "./knowledge_graph"

[sync]
role = "leader"
remote_url = "   "
"#,
        );

        let err = MagGraphConfig::load(&config_path).expect_err("expected validation error");
        assert!(matches!(err, MagGraphError::ConfigValidation(_)));
    }

    #[test]
    fn initializes_graph_root_and_metadata_dir() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "local"
root_path = "./knowledge_graph"
"#,
        );

        let resolved = MagGraphConfig::load(&config_path).expect("load config");
        assert!(!resolved.root_exists());

        resolved
            .initialize_graph_root(true)
            .expect("initialize graph root");

        assert!(resolved.root_exists());
        assert!(resolved.root_path.join(METADATA_DIR_NAME).is_dir());
    }

    #[test]
    fn rejects_graph_root_when_path_is_a_file() {
        let temp = TempDir::new().expect("temp dir");
        let root_file = temp.path().join("knowledge_graph");
        let mut file = fs::File::create(&root_file).expect("create file");
        writeln!(file, "not a directory").expect("write file");

        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "local"
root_path = "./knowledge_graph"
"#,
        );

        let resolved = MagGraphConfig::load(&config_path).expect("load config");
        let err = resolved
            .initialize_graph_root(false)
            .expect_err("expected init error");

        assert!(matches!(err, MagGraphError::GraphRootInit { .. }));
    }

    #[test]
    fn loads_lakehouse_cache_defaults() {
        let temp = TempDir::new().expect("temp dir");
        let config_path = write_config(
            temp.path(),
            r#"
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]

[lakehouse.cache]
ttl_secs = 60
max_bytes = 4096
"#,
        );

        let resolved = MagGraphConfig::load(&config_path).expect("load");
        let cache = &resolved.config.lakehouse.as_ref().unwrap().cache;
        assert_eq!(cache.ttl_secs, 60);
        assert_eq!(cache.max_bytes, 4096);
    }

    #[test]
    fn example_config_resolves_root_path() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let config_path = manifest_dir.join("../examples/basic/maggraph.toml");

        let resolved = MagGraphConfig::load(&config_path).expect("load example config");
        assert_eq!(
            resolved.root_path,
            manifest_dir.join("../examples/basic/knowledge_graph")
        );
    }
}
