use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::lakehouse::content::ResolvedContent;

/// In-memory cache for externally resolved content.
#[derive(Debug, Clone)]
pub struct ContentCache {
    ttl: Duration,
    max_bytes: usize,
    current_bytes: usize,
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    content: ResolvedContent,
    fetched_at: Instant,
    size_bytes: usize,
}

impl ContentCache {
    pub fn new(ttl_secs: u64, max_bytes: usize) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            max_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, uri: &str) -> Option<&ResolvedContent> {
        let entry = self.entries.get(uri)?;
        if self.is_expired(entry) {
            return None;
        }
        Some(&entry.content)
    }

    pub fn insert(&mut self, uri: impl Into<String>, content: ResolvedContent) {
        let uri = uri.into();
        let size_bytes = content.approximate_size_bytes();

        if let Some(existing) = self.entries.remove(&uri) {
            self.current_bytes = self.current_bytes.saturating_sub(existing.size_bytes);
        }

        if self.max_bytes > 0 {
            while self.current_bytes + size_bytes > self.max_bytes {
                if !self.evict_oldest() {
                    break;
                }
            }
            if size_bytes > self.max_bytes {
                return;
            }
        }

        self.entries.insert(
            uri,
            CacheEntry {
                content,
                fetched_at: Instant::now(),
                size_bytes,
            },
        );
        self.current_bytes += size_bytes;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    fn is_expired(&self, entry: &CacheEntry) -> bool {
        self.ttl.as_secs() > 0 && entry.fetched_at.elapsed() > self.ttl
    }

    fn evict_oldest(&mut self) -> bool {
        let oldest_key = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.fetched_at)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_bytes = self.current_bytes.saturating_sub(entry.size_bytes);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lakehouse::content::{AssetMetadata, ResolvedContent};

    #[test]
    fn caches_and_returns_content() {
        let mut cache = ContentCache::new(60, 1024);
        let content = ResolvedContent::ExternalAsset {
            uri: "s3://bucket/key".into(),
            format: "parquet".into(),
            metadata: AssetMetadata::default(),
            snippet: None,
        };
        cache.insert("s3://bucket/key", content.clone());
        assert!(cache.get("s3://bucket/key").is_some());
    }

    #[test]
    fn respects_max_bytes() {
        let mut cache = ContentCache::new(0, 50);
        for i in 0..5 {
            cache.insert(
                format!("s3://bucket/key{i}"),
                ResolvedContent::ExternalAsset {
                    uri: format!("s3://bucket/key{i}"),
                    format: "parquet".into(),
                    metadata: AssetMetadata::default(),
                    snippet: Some("x".repeat(20)),
                },
            );
        }
        assert!(cache.current_bytes() <= 50);
    }
}
