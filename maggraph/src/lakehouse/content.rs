use crate::node::Node;

/// Content returned when reading a node (local body or resolved external asset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedContent {
    /// Markdown body stored in-repo (local mode or lakehouse node without fetch).
    LocalMarkdown {
        body: String,
    },
    /// Plain text fetched from an external URI.
    Text {
        uri: String,
        body: String,
    },
    /// External asset metadata (and optional snippet); full analytics deferred.
    ExternalAsset {
        uri: String,
        format: String,
        metadata: AssetMetadata,
        snippet: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetMetadata {
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub parquet: Option<ParquetMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParquetMetadata {
    /// Whether the file begins with the PAR1 magic header.
    pub magic_valid: bool,
    /// Row group count is not parsed in MVP; reserved for future use.
    pub row_groups: Option<u32>,
}

/// A graph node paired with resolved content appropriate for the storage mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeWithContent {
    pub node: Node,
    pub content: ResolvedContent,
}

impl ResolvedContent {
    pub fn approximate_size_bytes(&self) -> usize {
        match self {
            Self::LocalMarkdown { body } => body.len(),
            Self::Text { body, .. } => body.len(),
            Self::ExternalAsset { snippet, metadata, .. } => {
                snippet.as_ref().map(String::len).unwrap_or(0)
                    + metadata
                        .size_bytes
                        .map(|size| size.min(usize::MAX as u64) as usize)
                        .unwrap_or(128)
            }
        }
    }

    /// Markdown-friendly summary for agents.
    pub fn to_markdown(&self) -> String {
        match self {
            Self::LocalMarkdown { body } => body.clone(),
            Self::Text { uri, body } => format!("## Resolved content\n\n**URI:** `{uri}`\n\n{body}"),
            Self::ExternalAsset {
                uri,
                format,
                metadata,
                snippet,
            } => {
                let mut sections = vec![
                    "## External asset".to_string(),
                    format!("**URI:** `{uri}`"),
                    format!("**Format:** `{format}`"),
                ];
                if let Some(size) = metadata.size_bytes {
                    sections.push(format!("**Size:** {size} bytes"));
                }
                if let Some(parquet) = &metadata.parquet {
                    sections.push(format!(
                        "**Parquet magic valid:** {}",
                        parquet.magic_valid
                    ));
                }
                if let Some(snippet) = snippet {
                    sections.push(format!("### Snippet\n\n{snippet}"));
                }
                sections.join("\n\n")
            }
        }
    }
}
