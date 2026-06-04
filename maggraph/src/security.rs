//! Path and URI safety checks for MagGraph.
//!
//! See [`planning/SECURITY.md`](../../planning/SECURITY.md) for the threat model.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};

use crate::config::METADATA_DIR_NAME;
use crate::error::{MagGraphError, Result};

/// Validate that a node path stays inside the graph root (no `..`, no absolute paths).
pub fn validate_relative_node_path(relative_path: &Path) -> Result<PathBuf> {
    if relative_path.as_os_str().is_empty() {
        return Err(MagGraphError::Index(
            "node relative_path must not be empty".into(),
        ));
    }

    if relative_path.is_absolute() {
        return Err(MagGraphError::Index(
            "node relative_path must be relative to the graph root".into(),
        ));
    }

    for component in relative_path.components() {
        match component {
            Component::ParentDir => {
                return Err(MagGraphError::Index(
                    "node relative_path must not contain '..' components".into(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(MagGraphError::Index(
                    "node relative_path must not be absolute".into(),
                ));
            }
            _ => {}
        }
    }

    if relative_path
        .components()
        .any(|c| c.as_os_str() == METADATA_DIR_NAME)
    {
        return Err(MagGraphError::Index(format!(
            "node paths must not live under `{METADATA_DIR_NAME}/`"
        )));
    }

    Ok(relative_path.to_path_buf())
}

/// Ensure a resolved filesystem path stays under `root` after canonicalization.
pub fn assert_path_within_root(root: &Path, candidate: &Path) -> Result<PathBuf> {
    let canonical_root = root.canonicalize().map_err(|source| {
        MagGraphError::Index(format!(
            "failed to canonicalize graph root {}: {source}",
            root.display()
        ))
    })?;

    let canonical = candidate.canonicalize().map_err(|source| {
        MagGraphError::Index(format!(
            "failed to canonicalize path {}: {source}",
            candidate.display()
        ))
    })?;

    if !canonical.starts_with(&canonical_root) {
        return Err(MagGraphError::Index(format!(
            "path {} escapes graph root {}",
            canonical.display(),
            canonical_root.display()
        )));
    }

    Ok(canonical)
}

/// Reject HTTP(S) URIs whose host is loopback, link-local, or private (SSRF mitigation).
pub fn validate_http_uri_host(uri: &str) -> Result<()> {
    let host = http_uri_host(uri).ok_or_else(|| {
        MagGraphError::Lakehouse(format!("invalid HTTP URI (missing host): {uri}"))
    })?;

    if host.eq_ignore_ascii_case("localhost") {
        return Err(MagGraphError::Lakehouse(format!(
            "HTTP URI host `{host}` is not allowed (loopback)"
        )));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(MagGraphError::Lakehouse(format!(
                "HTTP URI host `{host}` is not allowed (private or loopback address)"
            )));
        }
        return Ok(());
    }

    // Block obvious local names even without DNS resolution.
    if host.ends_with(".local") || host.ends_with(".localhost") {
        return Err(MagGraphError::Lakehouse(format!(
            "HTTP URI host `{host}` is not allowed"
        )));
    }

    Ok(())
}

fn http_uri_host(uri: &str) -> Option<String> {
    let rest = uri
        .strip_prefix("http://")
        .or_else(|| uri.strip_prefix("https://"))?;
    let authority = rest.split('/').next()?.split('@').next()?;
    let host = authority.split(':').next().filter(|h| !h.is_empty())?;
    Some(host.trim_matches(['[', ']']).to_string())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.octets()[0] == 0
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local() || ip.is_unicast_link_local()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    #[test]
    fn rejects_parent_dir_in_relative_path() {
        let err = validate_relative_node_path(Path::new("../escape.md")).expect_err("err");
        assert!(matches!(err, MagGraphError::Index(_)));
    }

    #[test]
    fn rejects_absolute_relative_path() {
        let err = validate_relative_node_path(Path::new("/etc/passwd")).expect_err("err");
        assert!(matches!(err, MagGraphError::Index(_)));
    }

    #[test]
    fn rejects_maggraph_metadata_path() {
        let err = validate_relative_node_path(Path::new(".maggraph/secret.md")).expect_err("err");
        assert!(matches!(err, MagGraphError::Index(_)));
    }

    #[test]
    fn accepts_nested_relative_path() {
        let path = validate_relative_node_path(Path::new("notes/topic.md")).expect("ok");
        assert_eq!(path, PathBuf::from("notes/topic.md"));
    }

    #[test]
    fn assert_path_within_root_blocks_escape() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        fs::create_dir_all(&root).expect("root");
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, "nope").expect("write");

        let err = assert_path_within_root(&root, &outside).expect_err("escape");
        assert!(matches!(err, MagGraphError::Index(_)));
    }

    #[test]
    fn blocks_localhost_http_uri() {
        let err = validate_http_uri_host("http://127.0.0.1/data").expect_err("localhost");
        assert!(matches!(err, MagGraphError::Lakehouse(_)));
    }

    #[test]
    fn blocks_private_http_uri() {
        let err = validate_http_uri_host("https://192.168.1.10/x").expect_err("private");
        assert!(matches!(err, MagGraphError::Lakehouse(_)));
    }

    #[test]
    fn allows_public_http_uri() {
        validate_http_uri_host("https://example.com/data.parquet").expect("public ok");
    }
}
