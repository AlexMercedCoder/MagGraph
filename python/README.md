# MagGraph

**In-process Git-backed graph engine for AI semantic layers — powered by Rust**

[![CI](https://github.com/AlexMercedCoder/MagGraph/actions/workflows/ci.yml/badge.svg)](https://github.com/AlexMercedCoder/MagGraph/actions)
[![PyPI](https://img.shields.io/pypi/v/maggraph)](https://pypi.org/project/maggraph/)
[![Python](https://img.shields.io/pypi/pyversions/maggraph)](https://pypi.org/project/maggraph/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/AlexMercedCoder/MagGraph)

> **Why "MagGraph"?** The name is short for **Magpie** — a Corvid. Corvids (ravens,
> crows, jays, and magpies) are renowned in animal cognition research for their
> remarkable intelligence, long-term memory, and sophisticated tool use.
> MagGraph is built to be the memory and knowledge layer for AI agents with those
> same qualities: a graph that thinks, remembers, and uses tools.

MagGraph stores knowledge as versioned Markdown nodes in your Git repository,
with BFS/DFS traversal, Git-backed sync, external lakehouse content resolution,
and a built-in MCP server scaffold — all from a zero-dependency `pip install`.

---

## Install

```bash
pip install maggraph
```

Pre-built wheels are available for:

| Platform | Architectures |
|----------|---------------|
| Linux (manylinux_2_28) | x86\_64 · aarch64 |
| macOS | Intel (x86\_64) · Apple Silicon (arm64) |
| Windows | x86\_64 |

> **No Rust toolchain required** — the Rust core is compiled into the wheel.

---

## Quick start

```python
import maggraph

# Load config + open the graph index
config = maggraph.load_config("maggraph.toml")
index  = config.open_index()

# List nodes
print(index.list_nodes())          # ['getting_started', 'welcome', ...]

# Read a node
node = index.read_node("welcome")
print(node.body)                   # full markdown body

# BFS traversal
result = index.traverse("welcome", depth=2, order="bfs")
print(result.to_markdown(index))   # formatted traversal report

# CRUD
index.create_node("new_note", node_type="note", body="# Hi\n", links=["welcome"])
index.update_node("new_note", "# Updated\n")
index.delete_node("new_note")
```

---

## Async support

```python
import asyncio, maggraph

async def main():
    index = maggraph.open_index("examples/basic/knowledge_graph")
    node  = await index.read_node_async("welcome")
    result = await index.traverse_async("welcome", depth=3, order="dfs")
    print(result.to_markdown(index))

asyncio.run(main())
```

Blocking Rust work runs on a Tokio thread pool — Python's event loop stays responsive.

---

## Lakehouse content resolution

Resolve external data sources (S3, file://, HTTP) referenced from node frontmatter:

```python
import maggraph

config = maggraph.load_config("maggraph.toml")  # mode = "lakehouse"
index  = config.open_index()
reader = config.open_lakehouse_reader()

# Resolve a node's external source (e.g. s3://bucket/data.parquet)
result = reader.read_node(index, "customer_churn_q2")
print(result.content.kind)          # "external_asset"
print(result.content.uri)           # "s3://corp-data/lake/churn.parquet"
print(result.content.format)        # "parquet"
print(result.content.to_markdown()) # agent-friendly summary

# Cache stats
print(reader.cache_len())    # 1
print(reader.cache_bytes())  # ~128

# Also callable directly on the index
result2 = index.read_node_with_content(reader, "customer_churn_q2")
```

**`maggraph.toml`** for lakehouse mode:

```toml
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [
  { uri = "s3://corp-data/lake", format = "parquet" }
]
```

---

## MCP server scaffold

```bash
maggraph scaffold --mcp --output ./mcp_server
```

Generates a ready-to-run FastMCP server at `./mcp_server/server.py` wired to
your graph index — expose `list_nodes`, `read_node`, `traverse`, `create_node`,
and `delete_node` as MCP tools with one command.

---

## Git-backed sync

```bash
# Leader pushes a snapshot
maggraph sync push --message "Add Q2 analysis nodes"

# Follower (read-only) pulls
maggraph sync pull
```

---

## API reference

| Class / Function | Description |
|-----------------|-------------|
| `load_config(path)` | Load `maggraph.toml` → `ResolvedConfig` |
| `open_index(root_path)` | Open graph index directly → `GraphIndex` |
| `ResolvedConfig.open_index()` | Open index from config |
| `ResolvedConfig.open_lakehouse_reader()` | Create a `LakehouseReader` |
| `GraphIndex.list_nodes()` | All node ids (sorted) |
| `GraphIndex.read_node(id)` | `Node` with metadata + body |
| `GraphIndex.read_node_async(id)` | Async version |
| `GraphIndex.traverse(id, depth, order)` | BFS/DFS → `TraversalResult` |
| `GraphIndex.traverse_async(...)` | Async version |
| `GraphIndex.create_node(...)` | Write new node to disk + index |
| `GraphIndex.update_node(id, body)` | Update body on disk |
| `GraphIndex.delete_node(id)` | Delete node from disk + index |
| `GraphIndex.read_node_with_content(reader, id)` | Resolve external content |
| `LakehouseReader.read_node(index, id)` | → `NodeWithContent` |
| `LakehouseReader.read_node_async(index, id)` | Async version |
| `LakehouseReader.cache_len()` | Entries in content cache |
| `LakehouseReader.cache_bytes()` | Bytes in content cache |
| `Node.id / .node_type / .body / .links / .source` | Node properties |
| `Node.to_markdown()` | Full node as Markdown string |
| `Node.to_dict()` | Node as plain Python dict |
| `ResolvedContent.kind` | `"local"` / `"text"` / `"external_asset"` |
| `ResolvedContent.body / .uri / .format` | Content details |
| `ResolvedContent.to_markdown()` | Agent-friendly summary |
| `NodeWithContent.node / .content` | Node + resolved content |

---

## Links

- [GitHub](https://github.com/AlexMercedCoder/MagGraph)
- [Architecture & planning docs](https://github.com/AlexMercedCoder/MagGraph/tree/main/planning)
- [Examples](https://github.com/AlexMercedCoder/MagGraph/tree/main/examples)
- [CONTRIBUTING.md](https://github.com/AlexMercedCoder/MagGraph/blob/main/CONTRIBUTING.md)

---

## License

MIT OR Apache-2.0
