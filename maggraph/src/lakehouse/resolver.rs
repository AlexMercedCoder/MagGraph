use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{MagGraphError, Result};
use crate::lakehouse::content::{AssetMetadata, ParquetMetadata, ResolvedContent};
use crate::lakehouse::uri::{infer_format, uri_scheme, validate_scheme};

/// Fetches content for a resolved external URI.
pub trait ContentResolver: Send + Sync {
    fn schemes(&self) -> &[&str];

    fn supports(&self, uri: &str) -> bool {
        uri_scheme(uri)
            .map(|scheme| self.schemes().contains(&scheme))
            .unwrap_or(false)
    }

    fn fetch(&self, uri: &str, format_hint: &str) -> Result<ResolvedContent>;
}

/// Registry of resolvers tried in registration order.
#[derive(Default)]
pub struct ResolverRegistry {
    resolvers: Vec<Box<dyn ContentResolver>>,
}

impl ResolverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(FileResolver::new(Vec::new())));
        registry.register(Box::new(HttpResolver));
        registry.register(Box::new(S3StubResolver));
        registry
    }

    /// Resolver set for production use: file allowlist plus HTTP/S3 stubs.
    pub fn with_file_allowlist(allowed_roots: Vec<PathBuf>) -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(FileResolver::new(allowed_roots)));
        registry.register(Box::new(HttpResolver));
        registry.register(Box::new(S3StubResolver));
        registry
    }

    pub fn register(&mut self, resolver: Box<dyn ContentResolver>) {
        self.resolvers.push(resolver);
    }

    pub fn fetch(&self, uri: &str, format_hint: &str) -> Result<ResolvedContent> {
        for resolver in &self.resolvers {
            if resolver.supports(uri) {
                return resolver.fetch(uri, format_hint);
            }
        }

        Err(MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: "no resolver registered for URI scheme".into(),
        })
    }
}

/// Resolves `file://` URIs and optional directory allowlist roots.
#[derive(Debug, Clone)]
pub struct FileResolver {
    allowed_roots: Vec<PathBuf>,
}

impl FileResolver {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self { allowed_roots }
    }

    fn resolve_path(&self, uri: &str) -> Result<PathBuf> {
        validate_scheme("file")?;
        let path = if let Some(rest) = uri.strip_prefix("file://") {
            PathBuf::from(rest)
        } else {
            PathBuf::from(uri)
        };

        let canonical =
            fs::canonicalize(&path).map_err(|source| MagGraphError::ContentResolve {
                uri: uri.to_string(),
                message: format!("failed to read file {}: {source}", path.display()),
            })?;

        if !self.allowed_roots.is_empty()
            && !self
                .allowed_roots
                .iter()
                .any(|root| canonical.starts_with(root))
        {
            return Err(MagGraphError::ContentResolve {
                uri: uri.to_string(),
                message: format!(
                    "path {} is outside configured file allowlist",
                    canonical.display()
                ),
            });
        }

        Ok(canonical)
    }
}

impl ContentResolver for FileResolver {
    fn schemes(&self) -> &[&str] {
        &["file"]
    }

    fn fetch(&self, uri: &str, format_hint: &str) -> Result<ResolvedContent> {
        let path = self.resolve_path(uri)?;
        let metadata = fs::metadata(&path).map_err(|source| MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: format!("metadata: {source}"),
        })?;

        let format = if format_hint != "unknown" {
            format_hint.to_string()
        } else {
            infer_format(uri, &[])
        };

        if format == "parquet" || path.extension().and_then(|ext| ext.to_str()) == Some("parquet") {
            return Ok(ResolvedContent::ExternalAsset {
                uri: uri.to_string(),
                format: "parquet".into(),
                metadata: parquet_metadata_from_file(&path, metadata.len()),
                snippet: read_snippet(&path, 256).ok(),
            });
        }

        let body = fs::read_to_string(&path).map_err(|source| MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: format!("read: {source}"),
        })?;

        Ok(ResolvedContent::Text {
            uri: uri.to_string(),
            body,
        })
    }
}

/// HTTP/HTTPS stub: returns metadata only (no network I/O in MVP).
pub struct HttpResolver;

impl ContentResolver for HttpResolver {
    fn schemes(&self) -> &[&str] {
        &["http", "https"]
    }

    fn fetch(&self, uri: &str, format_hint: &str) -> Result<ResolvedContent> {
        validate_scheme(uri_scheme(uri).unwrap_or("http"))?;
        Ok(ResolvedContent::ExternalAsset {
            uri: uri.to_string(),
            format: format_hint.to_string(),
            metadata: AssetMetadata {
                content_type: Some("application/octet-stream".into()),
                ..AssetMetadata::default()
            },
            snippet: Some(
                "HTTP fetch is not enabled in MVP; URI recorded for agent follow-up.".into(),
            ),
        })
    }
}

/// S3 stub resolver: returns metadata and a short snippet without AWS credentials.
#[derive(Debug, Clone, Default)]
pub struct S3StubResolver;

impl ContentResolver for S3StubResolver {
    fn schemes(&self) -> &[&str] {
        &["s3"]
    }

    fn fetch(&self, uri: &str, format_hint: &str) -> Result<ResolvedContent> {
        validate_scheme("s3")?;
        let format = if format_hint == "unknown" {
            infer_format(uri, &[])
        } else {
            format_hint.to_string()
        };

        let (bucket, key) = parse_s3_uri(uri)?;
        let snippet = format!("s3://{bucket}/{key} (stub resolver — metadata only)");
        let is_parquet = format == "parquet";

        Ok(ResolvedContent::ExternalAsset {
            uri: uri.to_string(),
            format,
            metadata: AssetMetadata {
                content_type: Some(if is_parquet {
                    "application/vnd.apache.parquet".into()
                } else {
                    "application/octet-stream".into()
                }),
                parquet: if is_parquet {
                    Some(ParquetMetadata {
                        magic_valid: false,
                        row_groups: None,
                    })
                } else {
                    None
                },
                ..AssetMetadata::default()
            },
            snippet: Some(snippet),
        })
    }
}

fn parse_s3_uri(uri: &str) -> Result<(String, String)> {
    let rest = uri
        .strip_prefix("s3://")
        .ok_or_else(|| MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: "invalid s3 URI".into(),
        })?;

    let (bucket, key) = rest
        .split_once('/')
        .ok_or_else(|| MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: "s3 URI must be s3://bucket/key".into(),
        })?;

    if bucket.is_empty() || key.is_empty() {
        return Err(MagGraphError::ContentResolve {
            uri: uri.to_string(),
            message: "s3 bucket and key must not be empty".into(),
        });
    }

    Ok((bucket.to_string(), key.to_string()))
}

fn parquet_metadata_from_file(path: &Path, size_bytes: u64) -> AssetMetadata {
    let magic_valid = fs::File::open(path)
        .ok()
        .and_then(|mut file| {
            let mut buf = [0u8; 4];
            file.read_exact(&mut buf).ok()?;
            Some(buf == *b"PAR1")
        })
        .unwrap_or(false);

    AssetMetadata {
        size_bytes: Some(size_bytes),
        content_type: Some("application/vnd.apache.parquet".into()),
        parquet: Some(ParquetMetadata {
            magic_valid,
            row_groups: None,
        }),
    }
}

fn read_snippet(path: &Path, max_bytes: usize) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|source| MagGraphError::ContentResolve {
        uri: path.display().to_string(),
        message: format!("open: {source}"),
    })?;
    let mut buf = vec![0u8; max_bytes];
    let read = file
        .read(&mut buf)
        .map_err(|source| MagGraphError::ContentResolve {
            uri: path.display().to_string(),
            message: format!("read: {source}"),
        })?;
    buf.truncate(read);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Collect `file://` roots from remote source configuration for path allowlisting.
pub fn file_allowlist_from_remotes(remote_uris: &[String]) -> Vec<PathBuf> {
    remote_uris
        .iter()
        .filter_map(|uri| uri.strip_prefix("file://"))
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tempfile::TempDir;

    #[test]
    fn s3_stub_returns_metadata() {
        let resolver = S3StubResolver;
        let content = resolver
            .fetch("s3://lake/churn_data.parquet", "parquet")
            .expect("fetch");

        assert!(matches!(content, ResolvedContent::ExternalAsset { .. }));
        if let ResolvedContent::ExternalAsset {
            snippet, metadata, ..
        } = content
        {
            assert!(snippet.is_some());
            assert!(metadata.parquet.is_some());
        }
    }

    #[test]
    fn file_resolver_reads_text() {
        let temp = TempDir::new().expect("temp");
        let path = temp.path().join("note.txt");
        fs::write(&path, "hello lakehouse").expect("write");

        let uri = format!("file://{}", path.display());
        let resolver = FileResolver::new(vec![temp.path().canonicalize().unwrap()]);
        let content = resolver.fetch(&uri, "unknown").expect("fetch");

        match content {
            ResolvedContent::Text { body, .. } => assert_eq!(body, "hello lakehouse"),
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn file_resolver_parquet_metadata() {
        let temp = TempDir::new().expect("temp");
        let path = temp.path().join("data.parquet");
        let mut file = fs::File::create(&path).expect("create");
        file.write_all(b"PAR1xxxx").expect("write");

        let uri = format!("file://{}", path.display());
        let resolver = FileResolver::new(vec![temp.path().canonicalize().unwrap()]);
        let content = resolver.fetch(&uri, "parquet").expect("fetch");

        if let ResolvedContent::ExternalAsset { metadata, .. } = content {
            assert!(metadata.parquet.as_ref().unwrap().magic_valid);
            assert_eq!(metadata.size_bytes, Some(8));
        } else {
            panic!("expected external asset");
        }
    }

    #[test]
    fn registry_selects_matching_resolver() {
        let registry = ResolverRegistry::with_defaults();
        let content = registry
            .fetch("s3://bucket/object.parquet", "parquet")
            .expect("fetch");
        assert!(matches!(content, ResolvedContent::ExternalAsset { .. }));
    }
}
