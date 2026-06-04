# MagGraph Python bindings

PyO3 extension exposing the MagGraph Rust core to Python agents.

## Install (development)

From the repository root:

```bash
pip install maturin pytest pytest-asyncio
maturin develop --manifest-path maggraph/Cargo.toml --features python-ext -m python
```

Or from this directory:

```bash
cd python
maturin develop --features python
pytest
```

## Quick start

```python
import maggraph

config = maggraph.load_config("examples/basic/maggraph.toml")
index = config.open_index()

print(index.list_nodes())
node = index.read_node("welcome")
result = index.traverse("welcome", depth=2, order="bfs")
print(result.to_markdown(index))
```

See [`examples/python_agent.py`](../examples/python_agent.py) for sync and asyncio usage.

## Event loop

Async methods (`read_node_async`, `traverse_async`) use `pyo3-async-runtimes` with Python's asyncio loop. Blocking Rust work runs on a Tokio thread pool so the event loop stays responsive.

See [`planning/PYTHON.md`](../planning/PYTHON.md) for API details and CI wheel builds.
