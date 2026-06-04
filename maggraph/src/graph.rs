//! Directed graph adjacency, traversal, and Markdown traversal reports.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::{MagGraphError, Result};
use crate::index::GraphIndex;
use crate::node::Node;
use crate::wikilink::extract_wikilink_targets;

/// Edge direction: all edges are **outgoing** from source node to target node id.
///
/// Frontmatter `links` and body `[[wikilinks]]` both contribute outgoing edges.
/// Reverse navigation is not implicit; use traversal from the target node instead.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphAdjacency {
    outgoing: HashMap<String, Vec<String>>,
    /// Wikilink targets that could not be resolved to a node id (per source node).
    unresolved: HashMap<String, Vec<String>>,
}

impl GraphAdjacency {
    /// Build adjacency by reading all nodes in the index.
    pub fn from_index(index: &GraphIndex) -> Result<Self> {
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
        let mut unresolved: HashMap<String, Vec<String>> = HashMap::new();

        let id_set: HashSet<&str> = index.iter().map(|(id, _)| id).collect();
        let path_stem_to_id = build_path_stem_index(index);

        for (id, _) in index.iter() {
            let node = index.read_node(id)?;
            let (targets, dangling) = collect_outgoing_targets(&node, &id_set, &path_stem_to_id);
            if !targets.is_empty() {
                outgoing.insert(id.to_string(), targets);
            }
            if !dangling.is_empty() {
                unresolved.insert(id.to_string(), dangling);
            }
        }

        Ok(Self {
            outgoing,
            unresolved,
        })
    }

    /// Outgoing neighbor node ids for `id` (empty if unknown or no edges).
    pub fn neighbors(&self, id: &str) -> &[String] {
        self.outgoing.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn unresolved_targets(&self, id: &str) -> &[String] {
        self.unresolved.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn has_node(&self, id: &str) -> bool {
        self.outgoing.contains_key(id) || self.unresolved.contains_key(id)
    }

    /// All resolved outgoing edges as `(from_id, to_id)` pairs.
    pub fn outgoing_edges(&self) -> impl Iterator<Item = (&str, &str)> {
        self.outgoing
            .iter()
            .flat_map(|(from, targets)| targets.iter().map(move |to| (from.as_str(), to.as_str())))
    }

    /// All unresolved wikilink targets as `(from_id, target)` pairs.
    pub fn unresolved_edges(&self) -> impl Iterator<Item = (&str, &str)> {
        self.unresolved.iter().flat_map(|(from, targets)| {
            targets
                .iter()
                .map(move |target| (from.as_str(), target.as_str()))
        })
    }
}

fn build_path_stem_index(index: &GraphIndex) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (id, entry) in index.iter() {
        if let Some(stem) = entry.relative_path.file_stem().and_then(|s| s.to_str()) {
            map.entry(stem.to_string())
                .or_insert_with(|| id.to_string());
        }
    }
    map
}

fn collect_outgoing_targets(
    node: &Node,
    id_set: &HashSet<&str>,
    path_stem_to_id: &HashMap<String, String>,
) -> (Vec<String>, Vec<String>) {
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();
    let mut dangling = Vec::new();

    let mut consider = |raw: &str| {
        if raw.is_empty() {
            return;
        }
        match resolve_target(raw, id_set, path_stem_to_id) {
            Some(id) => {
                if seen.insert(id.clone()) {
                    resolved.push(id);
                }
            }
            None => {
                if seen.insert(format!("__unresolved__{raw}")) {
                    dangling.push(raw.to_string());
                }
            }
        }
    };

    for link in &node.metadata.links {
        consider(link);
    }

    for target in extract_wikilink_targets(&node.body) {
        consider(&target);
    }

    (resolved, dangling)
}

/// Resolve a link target to a node id.
///
/// 1. Exact id match
/// 2. Path stem match (e.g. `welcome` → node at `welcome.md`)
pub fn resolve_target(
    raw: &str,
    id_set: &HashSet<&str>,
    path_stem_to_id: &HashMap<String, String>,
) -> Option<String> {
    let target = raw.trim();
    if target.is_empty() {
        return None;
    }

    if id_set.contains(target) {
        return Some(target.to_string());
    }

    path_stem_to_id.get(target).cloned()
}

/// Traversal order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraversalOrder {
    #[default]
    Bfs,
    Dfs,
}

/// A node visited during traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalNode {
    pub id: String,
    pub depth: u32,
    /// Path of node ids from the start node to this node (inclusive).
    pub path: Vec<String>,
}

/// Result of a graph traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalResult {
    pub start: String,
    pub max_depth: u32,
    pub order: TraversalOrder,
    pub nodes: Vec<TraversalNode>,
}

impl TraversalResult {
    /// Format traversal as a Markdown report for LLM consumption.
    pub fn to_markdown(&self, index: &GraphIndex) -> String {
        let mut out = String::new();
        out.push_str("# MagGraph Traversal Report\n\n");
        out.push_str(&format!("- **Start:** `{}`\n", self.start));
        out.push_str(&format!("- **Max depth:** {}\n", self.max_depth));
        out.push_str(&format!(
            "- **Order:** {}\n",
            match self.order {
                TraversalOrder::Bfs => "BFS",
                TraversalOrder::Dfs => "DFS",
            }
        ));
        out.push_str(&format!("- **Nodes reached:** {}\n\n", self.nodes.len()));

        if self.nodes.is_empty() {
            out.push_str("_No nodes reached at the requested depth._\n");
            return out;
        }

        out.push_str("## Nodes\n\n");
        for visited in &self.nodes {
            let entry = index.get(&visited.id);
            let node_type = entry
                .map(|e| e.metadata.node_type.as_str())
                .unwrap_or("unknown");
            let path = visited
                .path
                .iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(" → ");

            out.push_str(&format!("### {} (depth {})\n\n", visited.id, visited.depth));
            out.push_str(&format!("- **Type:** {node_type}\n"));
            out.push_str(&format!("- **Path:** {path}\n"));
            if let Some(entry) = entry {
                out.push_str(&format!(
                    "- **File:** `{}`\n",
                    entry.relative_path.display()
                ));
            }
            out.push('\n');
        }

        out
    }
}

/// Traverse outgoing edges from `from` up to `max_depth` hops.
///
/// The start node is included at depth 0. `max_depth` of 0 returns only the start node.
pub fn traverse(
    adjacency: &GraphAdjacency,
    index: &GraphIndex,
    from: &str,
    max_depth: u32,
    order: TraversalOrder,
) -> Result<TraversalResult> {
    if !index.contains(from) {
        return Err(MagGraphError::NodeNotFound {
            id: from.to_string(),
        });
    }

    let mut nodes = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(from.to_string());
    nodes.push(TraversalNode {
        id: from.to_string(),
        depth: 0,
        path: vec![from.to_string()],
    });

    match order {
        TraversalOrder::Bfs => bfs(adjacency, from, max_depth, &mut visited, &mut nodes),
        TraversalOrder::Dfs => dfs(adjacency, from, max_depth, &mut visited, &mut nodes),
    }

    Ok(TraversalResult {
        start: from.to_string(),
        max_depth,
        order,
        nodes,
    })
}

fn bfs(
    adjacency: &GraphAdjacency,
    from: &str,
    max_depth: u32,
    visited: &mut HashSet<String>,
    nodes: &mut Vec<TraversalNode>,
) {
    let mut queue = VecDeque::new();
    queue.push_back((from.to_string(), 0u32, vec![from.to_string()]));

    while let Some((current_id, depth, path)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        for neighbor in adjacency.neighbors(&current_id) {
            if !visited.insert(neighbor.clone()) {
                continue;
            }
            let next_depth = depth + 1;
            let mut next_path = path.clone();
            next_path.push(neighbor.clone());
            nodes.push(TraversalNode {
                id: neighbor.clone(),
                depth: next_depth,
                path: next_path.clone(),
            });
            queue.push_back((neighbor.clone(), next_depth, next_path));
        }
    }
}

fn dfs(
    adjacency: &GraphAdjacency,
    from: &str,
    max_depth: u32,
    visited: &mut HashSet<String>,
    nodes: &mut Vec<TraversalNode>,
) {
    fn visit(
        adjacency: &GraphAdjacency,
        current_id: &str,
        depth: u32,
        max_depth: u32,
        path: &[String],
        visited: &mut HashSet<String>,
        nodes: &mut Vec<TraversalNode>,
    ) {
        if depth >= max_depth {
            return;
        }

        for neighbor in adjacency.neighbors(current_id) {
            if !visited.insert(neighbor.clone()) {
                continue;
            }
            let next_depth = depth + 1;
            let mut next_path = path.to_vec();
            next_path.push(neighbor.clone());
            nodes.push(TraversalNode {
                id: neighbor.clone(),
                depth: next_depth,
                path: next_path.clone(),
            });
            visit(
                adjacency, neighbor, next_depth, max_depth, &next_path, visited, nodes,
            );
        }
    }

    let start_path = vec![from.to_string()];
    visit(adjacency, from, 0, max_depth, &start_path, visited, nodes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use tempfile::TempDir;

    fn write_traversal_fixture(root: &Path) {
        fs::create_dir_all(root).expect("create root");
        fs::write(
            root.join("welcome.md"),
            r#"---
id: "welcome"
type: "note"
links: ["getting_started"]
---
# Welcome

See [[getting_started]] and [[orphan_link]].
"#,
        )
        .expect("welcome");
        fs::write(
            root.join("getting_started.md"),
            r#"---
id: "getting_started"
type: "note"
links: ["welcome"]
---
# Getting Started

Back to [[welcome]].
"#,
        )
        .expect("getting_started");
        fs::write(
            root.join("leaf.md"),
            r#"---
id: "leaf"
type: "note"
---
# Leaf

Linked via path stem: [[welcome]].
"#,
        )
        .expect("leaf");
    }

    #[test]
    fn adjacency_merges_frontmatter_and_wikilinks() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        let welcome_neighbors = adj.neighbors("welcome");
        assert!(welcome_neighbors.contains(&"getting_started".to_string()));
        assert_eq!(welcome_neighbors.len(), 1);

        let unresolved = adj.unresolved_targets("welcome");
        assert!(unresolved.contains(&"orphan_link".to_string()));
    }

    #[test]
    fn resolves_path_stem_wikilinks() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        assert!(adj.neighbors("leaf").contains(&"welcome".to_string()));
    }

    #[test]
    fn bfs_traversal_respects_depth() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        let result = traverse(&adj, &index, "welcome", 1, TraversalOrder::Bfs).expect("traverse");
        let ids: Vec<_> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["welcome", "getting_started"]);

        let depth2 = traverse(&adj, &index, "welcome", 2, TraversalOrder::Bfs).expect("depth2");
        assert!(depth2.nodes.len() >= 2);
    }

    #[test]
    fn dfs_traversal_visits_neighbors() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        let result = traverse(&adj, &index, "welcome", 1, TraversalOrder::Dfs).expect("dfs");
        assert_eq!(result.nodes.len(), 2);
    }

    #[test]
    fn markdown_report_contains_sections() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");
        let result = traverse(&adj, &index, "welcome", 1, TraversalOrder::Bfs).expect("traverse");

        let md = result.to_markdown(&index);
        assert!(md.contains("# MagGraph Traversal Report"));
        assert!(md.contains("### welcome"));
        assert!(md.contains("### getting_started"));
    }

    #[test]
    fn traverse_errors_on_missing_start() {
        let temp = TempDir::new().expect("temp");
        let root = temp.path().join("graph");
        write_traversal_fixture(&root);

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        let err = traverse(&adj, &index, "missing", 1, TraversalOrder::Bfs).expect_err("err");
        assert!(matches!(err, MagGraphError::NodeNotFound { .. }));
    }

    #[test]
    fn basic_example_traversal_under_one_ms() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir.join("../examples/basic/knowledge_graph");

        let index = GraphIndex::open(&root).expect("open");
        let adj = GraphAdjacency::from_index(&index).expect("adjacency");

        let start = Instant::now();
        for _ in 0..100 {
            let _ = traverse(&adj, &index, "welcome", 2, TraversalOrder::Bfs).expect("traverse");
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "100 traversals took {:?}, expected <100ms total",
            elapsed
        );
    }
}
