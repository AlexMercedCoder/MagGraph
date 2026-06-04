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

### `GraphIndex`

| Method | Description |
|--------|-------------|
| `list_nodes()` | Sorted node ids |
| `read_node(id)` | Full node (metadata + body) |
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
