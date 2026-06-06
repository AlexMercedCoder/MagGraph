use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::Value;

use crate::error::Result;
use crate::index::GraphIndex;
use crate::node::Node;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryOptions {
    pub text: Option<String>,
    pub node_type: Option<String>,
    pub tags: Vec<String>,
    pub include_suppressed: bool,
    pub limit: usize,
    pub modified_since_unix: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub id: String,
    pub node_type: String,
    pub relative_path: String,
    pub score: i32,
    pub matched: Vec<String>,
    pub summary: String,
    pub modified_unix: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphChange {
    pub id: String,
    pub relative_path: String,
    pub modified_unix: i64,
}

pub fn search_index(index: &GraphIndex, options: &QueryOptions) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    let needle = options.text.as_deref().unwrap_or("").to_ascii_lowercase();
    let limit = if options.limit == 0 {
        50
    } else {
        options.limit
    };

    for (id, entry) in index.iter() {
        if let Some(node_type) = &options.node_type {
            if &entry.metadata.node_type != node_type {
                continue;
            }
        }
        if !options.include_suppressed && is_suppressed_extra(&entry.metadata.extra) {
            continue;
        }
        if !tags_match(&entry.metadata.extra, &options.tags) {
            continue;
        }

        let modified_unix = modified_unix(index, &entry.relative_path);
        if let Some(since) = options.modified_since_unix {
            if modified_unix.map(|m| m <= since).unwrap_or(true) {
                continue;
            }
        }

        let node = index.read_node(id)?;
        let mut score = 0;
        let mut matched = Vec::new();
        if needle.is_empty() {
            score = 1;
            matched.push("all".to_string());
        } else {
            score += score_text(id, &needle, 30, "id", &mut matched);
            score += score_text(&entry.metadata.node_type, &needle, 12, "type", &mut matched);
            score += score_text(&node.body, &needle, 6, "body", &mut matched);
            for link in &entry.metadata.links {
                score += score_text(link, &needle, 10, "links", &mut matched);
            }
            for (key, value) in &entry.metadata.extra {
                score += score_text(key, &needle, 4, "frontmatter", &mut matched);
                score += score_text(
                    &value_to_search_text(value),
                    &needle,
                    4,
                    "frontmatter",
                    &mut matched,
                );
            }
        }
        if score <= 0 {
            continue;
        }
        matched.sort();
        matched.dedup();
        results.push(SearchResult {
            id: id.to_string(),
            node_type: entry.metadata.node_type.clone(),
            relative_path: entry.relative_path.display().to_string(),
            score,
            matched,
            summary: summarize_body(&node),
            modified_unix,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.modified_unix.cmp(&a.modified_unix))
            .then_with(|| a.id.cmp(&b.id))
    });
    results.truncate(limit);
    Ok(results)
}

pub fn changed_since(index: &GraphIndex, since_unix: i64) -> Vec<GraphChange> {
    let mut changes = Vec::new();
    for (id, entry) in index.iter() {
        if let Some(modified) = modified_unix(index, &entry.relative_path) {
            if modified > since_unix {
                changes.push(GraphChange {
                    id: id.to_string(),
                    relative_path: entry.relative_path.display().to_string(),
                    modified_unix: modified,
                });
            }
        }
    }
    changes.sort_by(|a, b| {
        b.modified_unix
            .cmp(&a.modified_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    changes
}

pub fn is_suppressed(node: &Node) -> bool {
    is_suppressed_extra(&node.metadata.extra)
}

pub fn summarize_body(node: &Node) -> String {
    node.body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(280)
        .collect()
}

fn score_text(
    value: &str,
    needle: &str,
    weight: i32,
    label: &str,
    matched: &mut Vec<String>,
) -> i32 {
    if value.to_ascii_lowercase().contains(needle) {
        matched.push(label.to_string());
        weight
    } else {
        0
    }
}

fn tags_match(extra: &std::collections::BTreeMap<String, Value>, required: &[String]) -> bool {
    if required.is_empty() {
        return true;
    }
    let tags = extra
        .get("tags")
        .map(tags_from_value)
        .unwrap_or_default()
        .into_iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<Vec<_>>();
    required
        .iter()
        .all(|tag| tags.contains(&tag.to_ascii_lowercase()))
}

fn tags_from_value(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn value_to_search_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Sequence(items) => items
            .iter()
            .map(value_to_search_text)
            .collect::<Vec<_>>()
            .join(" "),
        Value::Mapping(map) => map
            .iter()
            .map(|(k, v)| format!("{} {}", value_to_search_text(k), value_to_search_text(v)))
            .collect::<Vec<_>>()
            .join(" "),
        Value::Null => String::new(),
        Value::Tagged(tagged) => value_to_search_text(&tagged.value),
    }
}

fn is_suppressed_extra(extra: &std::collections::BTreeMap<String, Value>) -> bool {
    extra
        .get("suppressed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn modified_unix(index: &GraphIndex, relative_path: &std::path::Path) -> Option<i64> {
    std::fs::metadata(index.root_path().join(relative_path))
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_to_unix)
}

fn system_time_to_unix(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
}
