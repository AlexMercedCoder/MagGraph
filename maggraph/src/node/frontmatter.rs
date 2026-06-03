use std::path::Path;

use serde_yaml::Value;

use crate::node::NodeMetadata;

/// Split a markdown file into optional YAML frontmatter and body.
pub fn split_frontmatter(raw: &str) -> Result<(Option<&str>, &str), String> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return Ok((None, raw));
    }

    let after_open = &trimmed[3..];
    let end = after_open.find("\n---").ok_or_else(|| {
        "markdown file is missing closing frontmatter delimiter (---)".to_string()
    })?;

    let yaml = &after_open[..end];
    let body_start = end + 4; // skip \n---
    let body = if after_open.len() > body_start {
        &after_open[body_start..]
    } else {
        ""
    };

    // Trim a single leading newline from body if present.
    let body = body.strip_prefix('\n').unwrap_or(body);

    Ok((Some(yaml.trim()), body))
}

/// Parse YAML frontmatter and body into typed metadata.
pub fn parse_markdown_node(raw: &str, path: &Path) -> Result<(NodeMetadata, String), String> {
    let (yaml, body) = split_frontmatter(raw)?;
    let yaml = yaml.ok_or_else(|| {
        format!(
            "node at {} must start with YAML frontmatter delimited by ---",
            path.display()
        )
    })?;

    let value: Value = serde_yaml::from_str(yaml)
        .map_err(|source| format!("invalid YAML frontmatter in {}: {source}", path.display()))?;

    let metadata: NodeMetadata = serde_yaml::from_value(value).map_err(|source| {
        format!(
            "invalid node frontmatter schema in {}: {source}",
            path.display()
        )
    })?;

    if metadata.id.trim().is_empty() {
        return Err(format!(
            "node at {} is missing required frontmatter field `id`",
            path.display()
        ));
    }

    if metadata.node_type.trim().is_empty() {
        return Err(format!(
            "node at {} is missing required frontmatter field `type`",
            path.display()
        ));
    }

    Ok((metadata, body.to_string()))
}

/// Serialize metadata and body back to markdown.
pub fn serialize_markdown_node(metadata: &NodeMetadata, body: &str) -> Result<String, String> {
    let yaml = serde_yaml::to_string(metadata)
        .map_err(|source| format!("failed to serialize frontmatter: {source}"))?;

    let yaml = yaml.trim_end();
    let mut output = String::from("---\n");
    output.push_str(yaml);
    output.push_str("\n---\n");
    output.push_str(body);

    if !body.is_empty() && !body.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_handles_bom() {
        let raw = "\u{feff}---\nid: x\n---\nbody";
        let (yaml, body) = split_frontmatter(raw).expect("split");
        assert_eq!(yaml, Some("id: x"));
        assert_eq!(body, "body");
    }
}
