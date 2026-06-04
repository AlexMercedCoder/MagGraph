//! Agent-facing artifacts: graph schema introspection, `SKILL.md`, and MCP scaffolds.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::error::{MagGraphError, Result};
use crate::graph::{resolve_target, GraphAdjacency};
use crate::index::GraphIndex;
use crate::node::Node;
use crate::wikilink::extract_wikilink_targets;

/// How an outgoing edge was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeSource {
    Frontmatter,
    Wikilink,
}

/// A resolved directed edge with node types and discovery source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaEdge {
    pub from_id: String,
    pub to_id: String,
    pub from_type: String,
    pub to_type: String,
    pub source: EdgeSource,
}

/// Count of edges between two node types (e.g. `note → note`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgePattern {
    pub from_type: String,
    pub to_type: String,
    pub count: usize,
}

/// Introspected graph schema for agent tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphSchema {
    pub node_count: usize,
    pub node_types: Vec<String>,
    pub nodes_by_type: BTreeMap<String, usize>,
    pub node_ids: Vec<String>,
    pub edge_count: usize,
    pub edges_from_frontmatter: usize,
    pub edges_from_wikilinks: usize,
    pub unresolved_link_count: usize,
    pub edge_patterns: Vec<EdgePattern>,
    pub sample_edges: Vec<SchemaEdge>,
}

impl GraphSchema {
    /// Build schema summary from an open graph index (includes adjacency scan).
    pub fn introspect(index: &GraphIndex) -> Result<Self> {
        let mut nodes_by_type: BTreeMap<String, usize> = BTreeMap::new();
        let mut node_types = BTreeSet::new();
        let mut node_ids = Vec::new();

        for (id, entry) in index.iter() {
            node_ids.push(id.to_string());
            node_types.insert(entry.metadata.node_type.clone());
            *nodes_by_type
                .entry(entry.metadata.node_type.clone())
                .or_insert(0) += 1;
        }
        node_ids.sort();

        let id_set: HashSet<&str> = index.iter().map(|(id, _)| id).collect();
        let path_stem_to_id = build_path_stem_index(index);
        let type_by_id: HashMap<String, String> = index
            .iter()
            .map(|(id, entry)| (id.to_string(), entry.metadata.node_type.clone()))
            .collect();

        let mut edges_from_frontmatter = 0usize;
        let mut edges_from_wikilinks = 0usize;
        let mut pattern_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        let mut sample_edges = Vec::new();

        for (id, _) in index.iter() {
            let node = index.read_node(id)?;
            let collected = collect_labeled_edges(&node, &id_set, &path_stem_to_id);
            for edge in collected {
                let from_type = type_by_id
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let to_type = type_by_id
                    .get(&edge.to_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                match edge.source {
                    EdgeSource::Frontmatter => edges_from_frontmatter += 1,
                    EdgeSource::Wikilink => edges_from_wikilinks += 1,
                }

                *pattern_counts
                    .entry((from_type.clone(), to_type.clone()))
                    .or_insert(0) += 1;

                if sample_edges.len() < 20 {
                    sample_edges.push(SchemaEdge {
                        from_id: id.to_string(),
                        to_id: edge.to_id,
                        from_type,
                        to_type,
                        source: edge.source,
                    });
                }
            }
        }

        let adjacency = GraphAdjacency::from_index(index)?;
        let unresolved_link_count: usize = index
            .iter()
            .map(|(id, _)| adjacency.unresolved_targets(id).len())
            .sum();

        let edge_count = edges_from_frontmatter + edges_from_wikilinks;
        let mut edge_patterns: Vec<EdgePattern> = pattern_counts
            .into_iter()
            .map(|((from_type, to_type), count)| EdgePattern {
                from_type,
                to_type,
                count,
            })
            .collect();
        edge_patterns.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.from_type.cmp(&b.from_type))
                .then_with(|| a.to_type.cmp(&b.to_type))
        });

        Ok(Self {
            node_count: index.len(),
            node_types: node_types.into_iter().collect(),
            nodes_by_type,
            node_ids,
            edge_count,
            edges_from_frontmatter,
            edges_from_wikilinks,
            unresolved_link_count,
            edge_patterns,
            sample_edges,
        })
    }
}

struct LabeledEdge {
    to_id: String,
    source: EdgeSource,
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

fn collect_labeled_edges(
    node: &Node,
    id_set: &HashSet<&str>,
    path_stem_to_id: &HashMap<String, String>,
) -> Vec<LabeledEdge> {
    let mut seen: HashSet<(String, EdgeSource)> = HashSet::new();
    let mut edges = Vec::new();

    let mut consider = |raw: &str, source: EdgeSource| {
        if raw.is_empty() {
            return;
        }
        if let Some(id) = resolve_target(raw, id_set, path_stem_to_id) {
            if seen.insert((id.clone(), source)) {
                edges.push(LabeledEdge { to_id: id, source });
            }
        }
    };

    for link in &node.metadata.links {
        consider(link, EdgeSource::Frontmatter);
    }
    for target in extract_wikilink_targets(&node.body) {
        consider(&target, EdgeSource::Wikilink);
    }

    edges
}

/// Context for rendering `SKILL.md`.
#[derive(Debug, Clone)]
pub struct SkillRenderContext<'a> {
    pub graph_root: &'a Path,
    pub config_path: Option<&'a Path>,
    pub storage_mode: Option<&'a str>,
    pub maggraph_version: &'a str,
}

/// Render `SKILL.md` content from schema and context.
pub fn render_skill_md(schema: &GraphSchema, ctx: &SkillRenderContext<'_>) -> String {
    let config_line = ctx
        .config_path
        .map(|p| format!("config: \"{}\"\n", p.display()))
        .unwrap_or_default();
    let storage_line = ctx
        .storage_mode
        .map(|m| format!("storage_mode: \"{m}\"\n"))
        .unwrap_or_default();

    let types_section = if schema.node_types.is_empty() {
        "_No nodes indexed yet._\n".to_string()
    } else {
        schema
            .nodes_by_type
            .iter()
            .map(|(ty, count)| format!("- `{ty}` — {count} node(s)"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    let ids_section = if schema.node_ids.is_empty() {
        "_No nodes indexed yet._\n".to_string()
    } else {
        schema
            .node_ids
            .iter()
            .map(|id| format!("- `{id}`"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    let patterns_section = if schema.edge_patterns.is_empty() {
        "_No resolved edges yet._\n".to_string()
    } else {
        schema
            .edge_patterns
            .iter()
            .map(|p| {
                format!(
                    "- `{from}` → `{to}` — {count} edge(s)",
                    from = p.from_type,
                    to = p.to_type,
                    count = p.count
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    let sample_edges_section = if schema.sample_edges.is_empty() {
        "_No edges._\n".to_string()
    } else {
        schema
            .sample_edges
            .iter()
            .take(10)
            .map(|e| {
                let via = match e.source {
                    EdgeSource::Frontmatter => "frontmatter",
                    EdgeSource::Wikilink => "wikilink",
                };
                format!(
                    "- `{from}` → `{to}` ({from_type} → {to_type}, via {via})",
                    from = e.from_id,
                    to = e.to_id,
                    from_type = e.from_type,
                    to_type = e.to_type,
                    via = via
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    format!(
        r#"---
maggraph_skill_version: "{version}"
graph_root: "{root}"
{config_line}{storage_line}node_count: {node_count}
edge_count: {edge_count}
---

# MagGraph Agent Skill

Machine-readable tool manual for this graph instance. Regenerate with `maggraph scaffold --skill` or `maggraph init --skill`.

## Graph summary

| Field | Value |
|-------|-------|
| Root | `{root}` |
| Nodes | {node_count} |
| Directed edges | {edge_count} ({edges_fm} frontmatter, {edges_wl} wikilink) |
| Unresolved wikilink targets | {unresolved} |

## Node types

{types_section}
## Node ids

{ids_section}
## Edge patterns (by type)

{patterns_section}
## Sample edges

{sample_edges_section}
## CLI operations

| Command | Purpose |
|---------|---------|
| `maggraph query --from <id> [--depth N] [--order bfs\|dfs]` | Traverse graph; Markdown report on stdout |
| `maggraph scaffold --mcp [--skill]` | Regenerate MCP server and/or `SKILL.md` |
| `maggraph sync pull` / `push` / `status` | Git sync (when `[sync]` configured) |

## Python API (`maggraph` package)

```python
import maggraph

resolved = maggraph.load_config("{config_display}")
index = resolved.open_index()
index.list_nodes()
index.read_node("welcome").to_markdown()
index.traverse("welcome", depth=2, order="bfs").to_markdown(index)
```

Async: `read_node_async`, `traverse_async` (see `planning/PYTHON.md`).

## MCP server

When `mcp_server/` exists:

```bash
pip install -r mcp_server/requirements.txt
# Install maggraph from repo: cd python && maturin develop --release --features python-ext
export MAGGRAPH_CONFIG="{config_display}"
python mcp_server/server.py
```

| Tool | Description |
|------|-------------|
| `list_nodes` | All indexed node ids |
| `get_node` | Node metadata + body as Markdown |
| `traverse_graph` | BFS/DFS traversal Markdown report |
| `create_node` | Create a new markdown node (optional write policy) |
| `update_node` | Update node body/frontmatter |
| `delete_node` | Remove a node from the index |

Regenerate when the graph schema changes: `maggraph scaffold --mcp --skill`.
"#,
        version = ctx.maggraph_version,
        root = ctx.graph_root.display(),
        config_line = config_line,
        storage_line = storage_line,
        node_count = schema.node_count,
        edge_count = schema.edge_count,
        edges_fm = schema.edges_from_frontmatter,
        edges_wl = schema.edges_from_wikilinks,
        unresolved = schema.unresolved_link_count,
        types_section = types_section,
        ids_section = ids_section,
        patterns_section = patterns_section,
        sample_edges_section = sample_edges_section,
        config_display = ctx
            .config_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "maggraph.toml".to_string()),
    )
}

/// Context for MCP scaffold generation.
#[derive(Debug, Clone)]
pub struct McpScaffoldContext<'a> {
    pub graph_root: &'a Path,
    pub config_path: &'a Path,
    pub schema: &'a GraphSchema,
}

/// Write FastMCP server scaffold wired to the `maggraph` Python package.
pub fn write_mcp_scaffold(dir: &Path, ctx: &McpScaffoldContext<'_>) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| io_error(format!("create {}: {e}", dir.display())))?;

    let schema = ctx.schema;
    let types_list = if schema.node_types.is_empty() {
        "_none_".to_string()
    } else {
        schema
            .node_types
            .iter()
            .map(|t| format!("`{t}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let ids_sample = schema
        .node_ids
        .iter()
        .take(10)
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ");

    let patterns_list = if schema.edge_patterns.is_empty() {
        "_none_".to_string()
    } else {
        schema
            .edge_patterns
            .iter()
            .take(8)
            .map(|p| format!("`{}` → `{}` ({})", p.from_type, p.to_type, p.count))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let readme = format!(
        r#"# MagGraph MCP Server

Generated by `maggraph scaffold --mcp`. Uses the `maggraph` Python bindings (PyO3) for graph operations.

## Graph

- **Config:** `{config}`
- **Root:** `{root}`
- **Nodes:** {node_count}
- **Types:** {types_list}
- **Edge patterns:** {patterns_list}
- **Sample ids:** {ids_sample}

## Setup

```bash
# From the MagGraph repository root (if maggraph is not on PyPI yet):
cd python && maturin develop --release --features python-ext

pip install -r mcp_server/requirements.txt
export MAGGRAPH_CONFIG="{config}"
python mcp_server/server.py
```

## Tools

| Tool | Description |
|------|-------------|
| `list_nodes` | List indexed node ids |
| `get_node` | Read node Markdown (metadata + body) |
| `traverse_graph` | BFS/DFS traversal report |
| `create_node` | Create a new node (id, type, body) |
| `update_node` | Update node body |
| `delete_node` | Delete a node by id |

See [`planning/MCP.md`](../../planning/MCP.md) for deployment (stdio, Cursor, CI).

Regenerate when the graph changes: `maggraph scaffold --mcp --skill`.
"#,
        config = ctx.config_path.display(),
        root = ctx.graph_root.display(),
        node_count = schema.node_count,
        types_list = types_list,
        patterns_list = patterns_list,
        ids_sample = if ids_sample.is_empty() {
            "_none_".to_string()
        } else {
            ids_sample
        },
    );

    let requirements = r#"fastmcp>=2.0
# maggraph: install from repo - cd python && maturin develop --release --features python-ext
"#;

    let server_py = format!(
        r##"# MagGraph FastMCP server - wired to maggraph Python bindings.

from __future__ import annotations

import os
from functools import lru_cache

import maggraph
from fastmcp import FastMCP

mcp = FastMCP("MagGraph")

CONFIG_PATH = os.environ.get("MAGGRAPH_CONFIG", r"{config}")
GRAPH_ROOT = r"{root}"
NODE_TYPES = {node_types:?}
KNOWN_NODE_IDS = {node_ids:?}


@lru_cache(maxsize=1)
def _resolved() -> maggraph.ResolvedConfig:
    return maggraph.load_config(CONFIG_PATH)


@lru_cache(maxsize=1)
def _index() -> maggraph.GraphIndex:
    return _resolved().open_index()


@mcp.tool
def list_nodes() -> list[str]:
    """List node ids in the MagGraph index."""
    return _index().list_nodes()


@mcp.tool
def get_node(node_id: str) -> str:
    """Read a node by id (YAML frontmatter + Markdown body)."""
    return _index().read_node(node_id).to_markdown()


@mcp.tool
def traverse_graph(from_id: str, depth: int = 2, order: str = "bfs") -> str:
    """Traverse outgoing edges from from_id; returns a Markdown report."""
    idx = _index()
    result = idx.traverse(from_id, depth, order)
    return result.to_markdown(idx)


@mcp.tool
def create_node(
    node_id: str,
    node_type: str = "note",
    body: str = "",
    links: list[str] | None = None,
) -> str:
    """Create a new markdown node. Requires write access (local or sync leader with lock)."""
    _index().create_node(node_id, node_type, body, links)
    return f"created node {{node_id!r}}"


@mcp.tool
def update_node(node_id: str, body: str) -> str:
    """Replace a node's Markdown body (frontmatter preserved)."""
    _index().update_node(node_id, body)
    return f"updated node {{node_id!r}}"


@mcp.tool
def delete_node(node_id: str) -> str:
    """Delete a node by id."""
    _index().delete_node(node_id)
    return f"deleted node {{node_id!r}}"


if __name__ == "__main__":
    mcp.run()
"##,
        config = ctx.config_path.display(),
        root = ctx.graph_root.display(),
        node_types = schema.node_types,
        node_ids = schema.node_ids,
    );

    fs::write(dir.join("README.md"), readme)
        .map_err(|e| io_error(format!("write README.md: {e}")))?;
    fs::write(dir.join("requirements.txt"), requirements)
        .map_err(|e| io_error(format!("write requirements.txt: {e}")))?;
    fs::write(dir.join("server.py"), server_py)
        .map_err(|e| io_error(format!("write server.py: {e}")))?;

    Ok(())
}

/// Write `SKILL.md` to `path`.
pub fn write_skill_md(
    path: &Path,
    schema: &GraphSchema,
    ctx: &SkillRenderContext<'_>,
) -> Result<()> {
    let content = render_skill_md(schema, ctx);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| io_error(format!("create {}: {e}", parent.display())))?;
    }
    fs::write(path, content).map_err(|e| io_error(format!("write {}: {e}", path.display())))?;
    Ok(())
}

fn io_error(message: String) -> MagGraphError {
    MagGraphError::Index(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn introspect_basic_example_has_edges_and_patterns() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let schema = GraphSchema::introspect(&index).expect("introspect");

        assert!(schema.node_count >= 2);
        assert!(schema.node_types.contains(&"note".to_string()));
        assert!(schema.node_ids.contains(&"welcome".to_string()));
        assert!(schema.edge_count >= 1);
        assert!(schema.edges_from_frontmatter >= 1 || schema.edges_from_wikilinks >= 1);
        assert!(!schema.edge_patterns.is_empty());
    }

    #[test]
    fn render_skill_contains_operations() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let schema = GraphSchema::introspect(&index).expect("introspect");
        let ctx = SkillRenderContext {
            graph_root: &root,
            config_path: Some(Path::new("maggraph.toml")),
            storage_mode: Some("local"),
            maggraph_version: "0.1.0",
        };
        let md = render_skill_md(&schema, &ctx);
        assert!(md.contains("maggraph_skill_version"));
        assert!(md.contains("traverse_graph"));
        assert!(md.contains("maggraph query"));
    }

    #[test]
    fn mcp_scaffold_writes_server_with_maggraph_import() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let schema = GraphSchema::introspect(&index).expect("introspect");
        let tmp = tempfile::tempdir().expect("tempdir");
        let mcp_dir = tmp.path().join("mcp_server");
        let ctx = McpScaffoldContext {
            graph_root: &root,
            config_path: Path::new("examples/basic/maggraph.toml"),
            schema: &schema,
        };
        write_mcp_scaffold(&mcp_dir, &ctx).expect("write");
        let server = fs::read_to_string(mcp_dir.join("server.py")).expect("read");
        assert!(server.contains("import maggraph"));
        assert!(server.contains("def traverse_graph"));
        assert!(server.contains("def create_node"));
    }
}
