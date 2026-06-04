use crate::config::RemoteSource;
use crate::error::{MagGraphError, Result};

/// Allowed URI schemes for lakehouse content resolution.
pub const ALLOWED_SCHEMES: &[&str] = &["file", "s3", "http", "https"];

/// Resolve a node's `source` against configured `[lakehouse].remote_sources`.
///
/// - Absolute URIs (with an allowed scheme) are returned unchanged.
/// - Relative paths are joined to the first matching `remote_sources` entry, or the
///   first entry when no prefix match is found.
pub fn resolve_source_uri(source: &str, remote_sources: &[RemoteSource]) -> Result<String> {
    let source = source.trim();
    if source.is_empty() {
        return Err(MagGraphError::Lakehouse("source must not be empty".into()));
    }

    if let Some(scheme) = uri_scheme(source) {
        validate_scheme(scheme)?;
        return Ok(source.to_string());
    }

    if remote_sources.is_empty() {
        return Err(MagGraphError::Lakehouse(
            "relative source requires at least one [lakehouse].remote_sources entry".into(),
        ));
    }

    let base = pick_remote_base(source, remote_sources);
    Ok(join_uri_path(&base.uri, source))
}

pub fn uri_scheme(uri: &str) -> Option<&str> {
    let (scheme, rest) = uri.split_once(':')?;
    if rest.starts_with("//")
        && !scheme.is_empty()
        && scheme.chars().all(|c| c.is_ascii_alphanumeric())
    {
        Some(scheme)
    } else {
        None
    }
}

pub fn validate_scheme(scheme: &str) -> Result<()> {
    if ALLOWED_SCHEMES.contains(&scheme) {
        Ok(())
    } else {
        Err(MagGraphError::DisallowedScheme {
            scheme: scheme.to_string(),
        })
    }
}

/// Infer asset format from URI path and optional remote source format hint.
pub fn infer_format(uri: &str, remote_sources: &[RemoteSource]) -> String {
    let path = uri_path(uri);
    if path.ends_with(".parquet") {
        return "parquet".into();
    }
    if path.ends_with(".json") {
        return "json".into();
    }
    if path.ends_with(".csv") {
        return "csv".into();
    }

    for remote in remote_sources {
        if uri.starts_with(&remote.uri) {
            return remote.format.clone();
        }
    }

    "unknown".into()
}

fn pick_remote_base<'a>(source: &str, remote_sources: &'a [RemoteSource]) -> &'a RemoteSource {
    remote_sources
        .iter()
        .filter(|remote| source.starts_with(remote.uri.trim_end_matches('/')))
        .max_by_key(|remote| remote.uri.len())
        .unwrap_or(&remote_sources[0])
}

fn join_uri_path(base: &str, relative: &str) -> String {
    let base = base.trim_end_matches('/');
    let relative = relative.trim_start_matches('/');
    format!("{base}/{relative}")
}

fn uri_path(uri: &str) -> &str {
    uri_scheme(uri)
        .and_then(|_| uri.split_once("://"))
        .map(|(_, rest)| rest)
        .unwrap_or(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parquet_remote() -> RemoteSource {
        RemoteSource {
            uri: "s3://corp-data/lake".into(),
            format: "parquet".into(),
        }
    }

    #[test]
    fn absolute_s3_uri_passes_through() {
        let resolved = resolve_source_uri("s3://lake/churn_data.parquet", &[parquet_remote()])
            .expect("resolve");
        assert_eq!(resolved, "s3://lake/churn_data.parquet");
    }

    #[test]
    fn relative_source_joins_remote_base() {
        let resolved =
            resolve_source_uri("churn_data.parquet", &[parquet_remote()]).expect("resolve");
        assert_eq!(resolved, "s3://corp-data/lake/churn_data.parquet");
    }

    #[test]
    fn rejects_disallowed_scheme() {
        let err =
            resolve_source_uri("ftp://example/data", &[parquet_remote()]).expect_err("scheme");
        assert!(matches!(err, MagGraphError::DisallowedScheme { .. }));
    }

    #[test]
    fn infers_parquet_from_extension() {
        assert_eq!(
            infer_format("s3://bucket/a.parquet", &[]),
            "parquet".to_string()
        );
    }
}
