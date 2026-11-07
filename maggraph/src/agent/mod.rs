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

/// Options shared by the `SKILL.md` and MCP scaffold generators.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScaffoldOptions {
    /// Generate a read-only MCP server: omit the `create_node`, `update_node`, and
    /// `delete_node` tools (and leave them out of the generated docs).
    pub read_only: bool,
}

impl ScaffoldOptions {
    /// Options for a read-only scaffold.
    pub fn read_only() -> Self {
        Self { read_only: true }
    }
}

/// Default directory name, under the scaffold output directory, that holds `SKILL.md`.
pub const SKILL_DIR_NAME: &str = "maggraph-skill";

/// `requirements.txt` line for the published `maggraph` Python bindings, pinned to
/// this crate's compatible release range (same minor for 0.x, same major after 1.0).
pub fn maggraph_python_requirement() -> String {
    let major: u64 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0);
    let minor: u64 = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0);
    if major == 0 {
        format!(
            "maggraph>={major}.{minor},<{major}.{next}",
            next = minor + 1
        )
    } else {
        format!("maggraph>={major}.{minor},<{next}", next = major + 1)
    }
}

const READ_TOOLS_TABLE: &str = "| `list_nodes` | All indexed node ids |
| `get_node` | Node metadata + body as Markdown |
| `traverse_graph` | BFS/DFS traversal Markdown report |
";

const WRITE_TOOLS_TABLE: &str =
    "| `create_node` | Create a new markdown node (optional write policy) |
| `update_node` | Update node body/frontmatter |
| `delete_node` | Remove a node from the index |
";

/// Render `SKILL.md` content from schema and context.
pub fn render_skill_md(schema: &GraphSchema, ctx: &SkillRenderContext<'_>) -> String {
    render_skill_md_with(schema, ctx, ScaffoldOptions::default())
}

/// Render `SKILL.md` content from schema, context, and scaffold options.
pub fn render_skill_md_with(
    schema: &GraphSchema,
    ctx: &SkillRenderContext<'_>,
    options: ScaffoldOptions,
) -> String {
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

When `mcp_server/` exists in the scaffold output directory (next to this skill's
`{skill_dir}/` directory), run from that output directory:

```bash
pip install -r mcp_server/requirements.txt   # fastmcp + maggraph from PyPI
export MAGGRAPH_CONFIG="{config_display}"
python mcp_server/server.py
```

{server_mode}

| Tool | Description |
|------|-------------|
{tools_table}
Regenerate when the graph schema changes: `maggraph scaffold --mcp --skill{read_only_flag}`.
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
        skill_dir = SKILL_DIR_NAME,
        server_mode = if options.read_only {
            "The server is **read-only**: it was generated with `--read-only`, so it exposes no create, update, or delete tools."
        } else {
            "The server is read-write. Regenerate with `--read-only` to omit the create, update, and delete tools."
        },
        tools_table = tools_table(options),
        read_only_flag = if options.read_only {
            " --read-only"
        } else {
            ""
        },
    )
}

fn tools_table(options: ScaffoldOptions) -> String {
    if options.read_only {
        READ_TOOLS_TABLE.to_string()
    } else {
        format!("{READ_TOOLS_TABLE}{WRITE_TOOLS_TABLE}")
    }
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
    write_mcp_scaffold_with(dir, ctx, ScaffoldOptions::default())
}

/// Write FastMCP server scaffold with options (for example a read-only server).
pub fn write_mcp_scaffold_with(
    dir: &Path,
    ctx: &McpScaffoldContext<'_>,
    options: ScaffoldOptions,
) -> Result<()> {
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
# Installs fastmcp and the published maggraph bindings from PyPI.
pip install -r mcp_server/requirements.txt
export MAGGRAPH_CONFIG="{config}"
python mcp_server/server.py
```

To run against a MagGraph source checkout instead of the PyPI release, install the
bindings from the repository's `python/` directory first
(`pip install maturin && maturin develop --release --features python-ext`) and drop
the `maggraph` line from `requirements.txt`.

## Tools

{server_mode}

| Tool | Description |
|------|-------------|
{tools_table}
See `planning/MCP.md` in the MagGraph repository for deployment (stdio, Cursor, CI).

Regenerate when the graph changes: `maggraph scaffold --mcp --skill{read_only_flag}`.
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
        server_mode = if options.read_only {
            "Read-only server (generated with `--read-only`): no create, update, or delete tools."
        } else {
            "Read-write server. Regenerate with `--read-only` to omit the create, update, and delete tools."
        },
        tools_table = tools_table(options),
        read_only_flag = if options.read_only {
            " --read-only"
        } else {
            ""
        },
    );

    let requirements = format!(
        "fastmcp>=2.0
{maggraph_req}
# maggraph is the MagGraph Python bindings package on PyPI (https://pypi.org/project/maggraph/).
# To use a source checkout instead, remove the line above and run, from the MagGraph
# repository's python/ directory: maturin develop --release --features python-ext
",
        maggraph_req = maggraph_python_requirement(),
    );

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
# True when generated with `maggraph scaffold --read-only` (write tools omitted).
READ_ONLY = {read_only}


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
{write_tools}

if __name__ == "__main__":
    mcp.run()
"##,
        config = ctx.config_path.display(),
        root = ctx.graph_root.display(),
        node_types = schema.node_types,
        node_ids = schema.node_ids,
        read_only = if options.read_only { "True" } else { "False" },
        write_tools = if options.read_only {
            ""
        } else {
            WRITE_TOOLS_PY
        },
    );

    fs::write(dir.join("README.md"), readme)
        .map_err(|e| io_error(format!("write README.md: {e}")))?;
    fs::write(dir.join("requirements.txt"), requirements)
        .map_err(|e| io_error(format!("write requirements.txt: {e}")))?;
    fs::write(dir.join("server.py"), server_py)
        .map_err(|e| io_error(format!("write server.py: {e}")))?;

    Ok(())
}

/// FastMCP write tools appended to `server.py` unless the scaffold is read-only.
const WRITE_TOOLS_PY: &str = r#"

@mcp.tool
def create_node(
    node_id: str,
    node_type: str = "note",
    body: str = "",
    links: list[str] | None = None,
) -> str:
    """Create a new markdown node. Requires write access (local or sync leader with lock)."""
    _index().create_node(node_id, node_type, body, links)
    return f"created node {node_id!r}"


@mcp.tool
def update_node(node_id: str, body: str) -> str:
    """Replace a node's Markdown body (frontmatter preserved)."""
    _index().update_node(node_id, body)
    return f"updated node {node_id!r}"


@mcp.tool
def delete_node(node_id: str) -> str:
    """Delete a node by id."""
    _index().delete_node(node_id)
    return f"deleted node {node_id!r}"

"#;

/// Write `SKILL.md` to `path`.
pub fn write_skill_md(
    path: &Path,
    schema: &GraphSchema,
    ctx: &SkillRenderContext<'_>,
) -> Result<()> {
    write_skill_md_with(path, schema, ctx, ScaffoldOptions::default())
}

/// Write `SKILL.md` to `path` using scaffold options.
pub fn write_skill_md_with(
    path: &Path,
    schema: &GraphSchema,
    ctx: &SkillRenderContext<'_>,
    options: ScaffoldOptions,
) -> Result<()> {
    let content = render_skill_md_with(schema, ctx, options);
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
        assert!(server.contains("def delete_node"));
        assert!(server.contains("READ_ONLY = False"));
        assert!(server.contains("return f\"created node {node_id!r}\""));
    }

    fn basic_schema() -> (PathBuf, GraphSchema) {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let schema = GraphSchema::introspect(&index).expect("introspect");
        (root, schema)
    }

    #[test]
    fn read_only_mcp_scaffold_omits_write_tools() {
        let (root, schema) = basic_schema();
        let tmp = tempfile::tempdir().expect("tempdir");
        let mcp_dir = tmp.path().join("mcp_server");
        let ctx = McpScaffoldContext {
            graph_root: &root,
            config_path: Path::new("maggraph.toml"),
            schema: &schema,
        };
        write_mcp_scaffold_with(&mcp_dir, &ctx, ScaffoldOptions::read_only()).expect("write");
        let server = fs::read_to_string(mcp_dir.join("server.py")).expect("server");
        assert!(server.contains("def traverse_graph"));
        assert!(server.contains("READ_ONLY = True"));
        assert!(!server.contains("create_node"));
        assert!(!server.contains("update_node"));
        assert!(!server.contains("delete_node"));
        let readme = fs::read_to_string(mcp_dir.join("README.md")).expect("readme");
        assert!(!readme.contains("`delete_node`"));
    }

    #[test]
    fn read_only_skill_omits_write_tools() {
        let (root, schema) = basic_schema();
        let ctx = SkillRenderContext {
            graph_root: &root,
            config_path: Some(Path::new("maggraph.toml")),
            storage_mode: Some("local"),
            maggraph_version: "0.4.1",
        };
        let rw = render_skill_md(&schema, &ctx);
        assert!(rw.contains("`create_node`"));
        let ro = render_skill_md_with(&schema, &ctx, ScaffoldOptions::read_only());
        assert!(ro.contains("`traverse_graph`"));
        assert!(!ro.contains("`create_node`"));
        assert!(ro.contains("maggraph scaffold --mcp --skill --read-only"));
    }

    #[test]
    fn requirements_pin_published_bindings_to_compatible_range() {
        let req = maggraph_python_requirement();
        let major = env!("CARGO_PKG_VERSION_MAJOR");
        let minor = env!("CARGO_PKG_VERSION_MINOR");
        assert!(
            req.starts_with(&format!("maggraph>={major}.{minor},<")),
            "{req}"
        );

        let (root, schema) = basic_schema();
        let tmp = tempfile::tempdir().expect("tempdir");
        let ctx = McpScaffoldContext {
            graph_root: &root,
            config_path: Path::new("maggraph.toml"),
            schema: &schema,
        };
        write_mcp_scaffold(tmp.path(), &ctx).expect("write");
        let requirements = fs::read_to_string(tmp.path().join("requirements.txt")).expect("req");
        assert!(requirements.lines().any(|line| line == req));
        assert!(requirements.contains("pypi.org/project/maggraph"));
    }
}
