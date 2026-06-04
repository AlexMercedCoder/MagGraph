# MagGraph

Rust-based in-process graph database for AI semantic layers. Markdown is the source of truth; edges come from `[[wikilinks]]`; Git backs sync and versioning.

## Build & test

```bash
cargo build
cargo test
cargo run -p maggraph-cli -- --config examples/basic/maggraph.toml
cargo run -p maggraph-cli -- query --from welcome --depth 2 --config examples/basic/maggraph.toml
maggraph ui --config examples/basic/maggraph.toml
```

Open http://127.0.0.1:8787 for the local dashboard. See [planning/UI.md](./planning/UI.md).

## Python bindings

```bash
pip install maturin pytest pytest-asyncio
cd python && maturin develop --features python-ext && pytest
python examples/python_agent.py
```

See [planning/PYTHON.md](./planning/PYTHON.md) and [python/README.md](./python/README.md).

See [`examples/README.md`](./examples/README.md) for sample `maggraph.toml` files and a small knowledge graph.

## Documentation

| Doc | Description |
|-----|-------------|
| [PRD.md](./PRD.md) | Product requirements and architecture |
| [planning/](./planning/) | Implementation plan, architecture reference, and progress tracker |

## Planning & progress

Start with [planning/README.md](./planning/README.md). Track work in [planning/PROGRESS.md](./planning/PROGRESS.md); follow phases in [planning/IMPLEMENTATION_PLAN.md](./planning/IMPLEMENTATION_PLAN.md).
