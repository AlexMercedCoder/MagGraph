# MagGraph Python bindings

Phase 7 exposes the Rust core via PyO3 for agent and MCP integration.

## Package layout

| Path | Purpose |
|------|---------|
| `maggraph/src/python/mod.rs` | PyO3 module (`maggraph._maggraph`) |
| `python/pyproject.toml` | Maturin build config |
| `python/maggraph/` | Pure Python package + type stubs |
| `python/tests/` | pytest suite |
| `examples/python_agent.py` | Sync + asyncio demo |

Build with the `python-ext` feature on the `maggraph` crate (includes `extension-module` for wheels):

```bash
cd python && maturin develop --features python-ext
```

## API surface

### Module functions

- `load_config(path: str) -> ResolvedConfig` — load and validate `maggraph.toml`
- `open_index(root_path: str) -> GraphIndex` — open index at graph root

### `ResolvedConfig`

| Member | Description |
|--------|-------------|
| `root_path` | Resolved graph root directory |
| `config_path` | Path to the loaded TOML file |
| `storage_mode` | `"local"` or `"lakehouse"` |
| `open_index()` | Open a `GraphIndex` at `root_path` |
| `open_lakehouse_reader()` | Open a `LakehouseReader` for local or lakehouse content |

### `GraphIndex`

| Method | Description |
|--------|-------------|
| `list_nodes()` | Sorted node ids |
| `read_node(id)` | Full node (metadata + body) |
| `search(query="", node_type=None, tags=None, include_suppressed=False, limit=50, modified_since_unix=None)` | Structured search over ids, types, tags, frontmatter, links, body, and recency |
| `backlinks(id)` | Node ids that link to `id` |
| `changed_since(unix)` | Files modified after a Unix timestamp |
| `update_file(path)` | Refresh one changed Markdown file in the index |
| `create_memory_node(id, kind, body="", links=None)` | Create a typed agent memory node |
| `suppress_node(id, reason=None)` / `unsuppress_node(id)` | Mark/unmark stale or duplicate memory |
| `merge_nodes(target, source)` | Merge a duplicate node into a canonical node |
| `recall_bundle(id, reason="", body_chars=1200)` | Compact agent retrieval bundle as a dict with Markdown |
| `traverse(from_id, depth=2, order="bfs")` | BFS/DFS traversal |
| `read_node_async(id)` | Async wrapper (non-blocking) |
| `traverse_async(...)` | Async traversal |

### `Node`

Properties: `id`, `node_type`, `source`, `links`, `body`, `relative_path`.

Methods: `to_markdown()`, `to_dict()`.

### Errors

Rust `MagGraphError` values surface as `maggraph.MagGraphError` (subclass of `Exception`).

## Async / event loop

Async methods use [`pyo3-async-runtimes`](https://github.com/PyO3/pyo3-async-runtimes) with the **asyncio** integration. Blocking Rust work (index scan, adjacency build, file reads) runs on a Tokio `spawn_blocking` pool so Python's event loop stays responsive.

Requirements:

- Python 3.9+
- An active asyncio event loop (e.g. `asyncio.run()` or `pytest-asyncio`)

Example:

```python
import asyncio
import maggraph

async def main():
    index = maggraph.load_config("maggraph.toml").open_index()
    result = await index.traverse_async("welcome", depth=2)
    print(result.to_markdown(index))

asyncio.run(main())
```

## Type stubs

`python/maggraph/__init__.pyi` and `py.typed` enable static analysis (mypy, pyright).

## CI wheel build

GitHub Actions job `python` (see `.github/workflows/ci.yml`):

1. Install Rust + Python 3.11
2. `maturin build --release` in `python/`
3. `maturin develop` + `pytest`

Wheels are built as CI artifacts; PyPI publish is deferred to Phase 10.

## MCP integration

`maggraph scaffold --mcp` generates `mcp_server/server.py` already wired to this package. See [`planning/MCP.md`](./MCP.md).

## CRUD (Phase 8)

```python
index = maggraph.open_index("/path/to/graph")
index.create_node("new_id", node_type="note", body="# Hi\n", links=["welcome"])
index.update_node("new_id", "# Updated\n")
index.delete_node("new_id")
```

## Agent memory helpers

```python
index.create_memory_node("prefers_cli", "preference", "User prefers CLI-first UX.")
index.suppress_node("old_fact", reason="superseded")
index.merge_nodes("canonical_fact", "duplicate_fact")
bundle = index.recall_bundle("canonical_fact", reason="matched search")
print(bundle["markdown"])
```

Memory kinds: `preference`, `project_fact`, `decision`, `task`,
`session_summary`, `bookmark`, `tool_failure`.

## Lakehouse mode

Python bindings expose `LakehouseReader` and external content resolution.

| Capability | Python | Rust |
|------------|--------|------|
| Index / traverse local nodes | ✅ | ✅ |
| Read node body from disk | ✅ | ✅ |
| Resolve `source` / `source_uri` externally | ✅ | ✅ |
| Cache + file allowlist | ✅ | ✅ |

## Testing & backlog

| Coverage today | Gap (backlog ID) |
|----------------|------------------|
| Config, index, traverse, async, CRUD, lakehouse reader, search/recall, memory quality | Coverage should expand as new agent workflows adopt the APIs |
| MCP scaffold smoke and CRUD | Keep generated server tests aligned with scaffold changes |

See [`TESTING.md`](./TESTING.md), [`BACKLOG.md`](./BACKLOG.md), [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md).
