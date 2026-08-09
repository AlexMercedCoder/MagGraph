use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde_yaml::Value;

use crate::error::Result;
use crate::graph::GraphAdjacency;
use crate::index::{GraphIndex, NodeIndexEntry};
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

/// Relative signal weights used by hybrid retrieval.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridWeights {
    pub lexical: f64,
    pub semantic: f64,
    pub graph: f64,
    pub recency: f64,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            lexical: 0.45,
            semantic: 0.30,
            graph: 0.15,
            recency: 0.10,
        }
    }
}

/// Explainable hybrid query over graph-native and optional caller-provided signals.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridQueryOptions {
    pub text: Option<String>,
    pub node_types: Vec<String>,
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub seed_ids: Vec<String>,
    pub semantic_scores: HashMap<String, f64>,
    pub include_suppressed: bool,
    pub include_superseded: bool,
    pub limit: usize,
    pub as_of_unix: Option<i64>,
    pub recency_half_life_days: f64,
    pub weights: HybridWeights,
}

impl Default for HybridQueryOptions {
    fn default() -> Self {
        Self {
            text: None,
            node_types: Vec::new(),
            tags: Vec::new(),
            project: None,
            seed_ids: Vec::new(),
            semantic_scores: HashMap::new(),
            include_suppressed: false,
            include_superseded: false,
            limit: 20,
            as_of_unix: None,
            recency_half_life_days: 30.0,
            weights: HybridWeights::default(),
        }
    }
}

/// Hybrid result with component scores and human-readable retrieval reasons.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridSearchResult {
    pub id: String,
    pub node_type: String,
    pub relative_path: String,
    pub score: f64,
    pub signals: BTreeMap<String, f64>,
    pub reasons: Vec<String>,
    pub summary: String,
    pub modified_unix: Option<i64>,
    pub canonical_id: String,
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

        let mut score = 0;
        let mut matched = Vec::new();
        if needle.is_empty() {
            score = 1;
            matched.push("all".to_string());
        } else {
            score += score_text(id, &needle, 30, "id", &mut matched);
            score += score_text(&entry.metadata.node_type, &needle, 12, "type", &mut matched);
            score += score_text(entry.body(), &needle, 6, "body", &mut matched);
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
            summary: entry.summary.clone(),
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

/// Rank nodes using lexical, graph, recency, and optional semantic scores.
pub fn hybrid_search_index(
    index: &GraphIndex,
    options: &HybridQueryOptions,
) -> Result<Vec<HybridSearchResult>> {
    let now = options.as_of_unix.unwrap_or_else(current_unix);
    let needle = options.text.as_deref().unwrap_or("").to_ascii_lowercase();
    let adjacency = index.adjacency()?;
    let graph_scores = graph_scores(&adjacency, &options.seed_ids);
    let superseded = superseded_ids(index);
    let mut results = Vec::new();

    for (id, entry) in index.iter() {
        if !options.node_types.is_empty() && !options.node_types.contains(&entry.metadata.node_type)
        {
            continue;
        }
        if !options.include_suppressed && is_suppressed_extra(&entry.metadata.extra) {
            continue;
        }
        if !options.include_superseded && superseded.contains(id) {
            continue;
        }
        if !tags_match(&entry.metadata.extra, &options.tags)
            || !project_matches(&entry.metadata.extra, options.project.as_deref())
            || !temporally_valid(&entry.metadata.extra, now)
        {
            continue;
        }

        let modified = modified_unix(index, &entry.relative_path);
        let lexical = lexical_score(id, entry, &needle);
        let semantic = options
            .semantic_scores
            .get(id)
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let graph = graph_scores.get(id).copied().unwrap_or(0.0);
        let recency = recency_score(modified, now, options.recency_half_life_days);
        if needle.is_empty() && semantic == 0.0 && graph == 0.0 {
            continue;
        }

        let score = lexical * options.weights.lexical
            + semantic * options.weights.semantic
            + graph * options.weights.graph
            + recency * options.weights.recency;
        let mut signals = BTreeMap::new();
        signals.insert("graph".to_string(), graph);
        signals.insert("lexical".to_string(), lexical);
        signals.insert("recency".to_string(), recency);
        signals.insert("semantic".to_string(), semantic);
        results.push(HybridSearchResult {
            id: id.to_string(),
            node_type: entry.metadata.node_type.clone(),
            relative_path: entry.relative_path.display().to_string(),
            score,
            signals,
            reasons: signal_reasons(lexical, semantic, graph, recency),
            summary: entry.summary.clone(),
            modified_unix: modified,
            canonical_id: extra_string(&entry.metadata.extra, "canonical_id")
                .unwrap_or_else(|| id.to_string()),
        });
    }

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b.modified_unix.cmp(&a.modified_unix))
            .then_with(|| a.id.cmp(&b.id))
    });
    results.truncate(if options.limit == 0 {
        20
    } else {
        options.limit
    });
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
    summarize_text(&node.body)
}

pub(crate) fn summarize_text(body: &str) -> String {
    body.lines()
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

fn lexical_score(id: &str, entry: &NodeIndexEntry, needle: &str) -> f64 {
    if needle.is_empty() {
        return 0.0;
    }
    let mut matched = Vec::new();
    let mut raw = score_text(id, needle, 30, "id", &mut matched)
        + score_text(&entry.metadata.node_type, needle, 12, "type", &mut matched)
        + score_text(entry.body(), needle, 6, "body", &mut matched);
    for link in &entry.metadata.links {
        raw += score_text(link, needle, 10, "links", &mut matched);
    }
    for (key, value) in &entry.metadata.extra {
        raw += score_text(key, needle, 4, "frontmatter", &mut matched);
        raw += score_text(
            &value_to_search_text(value),
            needle,
            4,
            "frontmatter",
            &mut matched,
        );
    }
    (f64::from(raw) / 30.0).clamp(0.0, 1.0)
}

fn graph_scores(adjacency: &GraphAdjacency, seeds: &[String]) -> HashMap<String, f64> {
    let mut scores = HashMap::new();
    for seed in seeds {
        scores.insert(seed.clone(), 1.0);
        for neighbor in adjacency
            .neighbors(seed)
            .iter()
            .chain(adjacency.backlinks(seed).iter())
        {
            scores
                .entry(neighbor.clone())
                .and_modify(|score| *score = f64::max(*score, 0.75))
                .or_insert(0.75);
            for second in adjacency
                .neighbors(neighbor)
                .iter()
                .chain(adjacency.backlinks(neighbor).iter())
            {
                scores
                    .entry(second.clone())
                    .and_modify(|score| *score = f64::max(*score, 0.4))
                    .or_insert(0.4);
            }
        }
    }
    scores
}

fn recency_score(modified: Option<i64>, now: i64, half_life_days: f64) -> f64 {
    let Some(modified) = modified else {
        return 0.0;
    };
    if modified >= now {
        return 1.0;
    }
    let half_life_seconds = half_life_days.max(0.01) * 86_400.0;
    let age = (now - modified) as f64;
    2.0_f64.powf(-age / half_life_seconds).clamp(0.0, 1.0)
}

fn signal_reasons(lexical: f64, semantic: f64, graph: f64, recency: f64) -> Vec<String> {
    let mut reasons = Vec::new();
    if lexical > 0.0 {
        reasons.push("lexical match".to_string());
    }
    if semantic > 0.0 {
        reasons.push("semantic match".to_string());
    }
    if graph > 0.0 {
        reasons.push("graph relationship".to_string());
    }
    if recency >= 0.5 {
        reasons.push("recent memory".to_string());
    }
    reasons
}

fn project_matches(
    extra: &std::collections::BTreeMap<String, Value>,
    project: Option<&str>,
) -> bool {
    let Some(project) = project else {
        return true;
    };
    extra_string(extra, "project")
        .or_else(|| extra_string(extra, "project_id"))
        .map(|value| value == project)
        .unwrap_or(true)
}

fn temporally_valid(extra: &std::collections::BTreeMap<String, Value>, as_of: i64) -> bool {
    let valid_from = extra.get("valid_from").and_then(parse_time_value);
    let valid_until = extra.get("valid_until").and_then(parse_time_value);
    valid_from.map(|value| value <= as_of).unwrap_or(true)
        && valid_until.map(|value| value > as_of).unwrap_or(true)
}

fn parse_time_value(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return Some(value);
    }
    value.as_str().and_then(|raw| {
        raw.parse::<i64>().ok().or_else(|| {
            DateTime::parse_from_rfc3339(raw)
                .ok()
                .map(|value| value.timestamp())
        })
    })
}

fn superseded_ids(index: &GraphIndex) -> HashSet<String> {
    index
        .iter()
        .filter_map(|(_, entry)| extra_string(&entry.metadata.extra, "supersedes"))
        .collect()
}

fn extra_string(extra: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<String> {
    extra.get(key).and_then(Value::as_str).map(str::to_string)
}

fn current_unix() -> i64 {
    Utc::now().timestamp()
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write_node(root: &std::path::Path, id: &str, extra: &str, body: &str) {
        fs::write(
            root.join(format!("{id}.md")),
            format!("---\nid: {id}\ntype: project_fact\n{extra}---\n{body}\n"),
        )
        .expect("write node");
    }

    #[test]
    fn hybrid_search_combines_explainable_signals_and_filters_stale_nodes() {
        let temp = TempDir::new().expect("tempdir");
        write_node(
            temp.path(),
            "anchor",
            "project: demo\nlinks: [neighbor]\n",
            "release architecture",
        );
        write_node(
            temp.path(),
            "neighbor",
            "project: demo\n",
            "connected decision",
        );
        write_node(
            temp.path(),
            "semantic",
            "project: demo\n",
            "unrelated vocabulary",
        );
        write_node(
            temp.path(),
            "expired",
            "project: demo\nvalid_until: 2020-01-01T00:00:00Z\n",
            "release architecture",
        );
        write_node(
            temp.path(),
            "old",
            "project: demo\n",
            "release architecture",
        );
        write_node(
            temp.path(),
            "new",
            "project: demo\nsupersedes: old\ncanonical_id: decision-current\n",
            "release architecture",
        );
        write_node(
            temp.path(),
            "foreign",
            "project: elsewhere\n",
            "release architecture",
        );
        write_node(temp.path(), "global", "", "release architecture");
        let index = GraphIndex::open(temp.path()).expect("open");
        let options = HybridQueryOptions {
            text: Some("release architecture".to_string()),
            project: Some("demo".to_string()),
            seed_ids: vec!["anchor".to_string()],
            semantic_scores: HashMap::from([("semantic".to_string(), 0.95)]),
            as_of_unix: Some(1_800_000_000),
            limit: 20,
            ..HybridQueryOptions::default()
        };

        let results = hybrid_search_index(&index, &options).expect("hybrid search");
        let ids = results
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"anchor"));
        assert!(ids.contains(&"neighbor"));
        assert!(ids.contains(&"semantic"));
        assert!(ids.contains(&"new"));
        assert!(ids.contains(&"global"));
        assert!(!ids.contains(&"expired"));
        assert!(!ids.contains(&"old"));
        assert!(!ids.contains(&"foreign"));
        assert_eq!(
            results
                .iter()
                .find(|item| item.id == "new")
                .unwrap()
                .canonical_id,
            "decision-current"
        );
        assert!(results
            .iter()
            .find(|item| item.id == "neighbor")
            .unwrap()
            .reasons
            .contains(&"graph relationship".to_string()));
        assert!(results
            .iter()
            .find(|item| item.id == "semantic")
            .unwrap()
            .reasons
            .contains(&"semantic match".to_string()));
    }
}
