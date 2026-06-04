# Examples

Manual fixtures for loading `maggraph.toml`, indexing markdown nodes, and trying CLI features.

| Directory | Purpose |
|-----------|---------|
| [`basic/`](./basic/) | Local-mode config with a two-node sample graph |
| [`lakehouse/`](./lakehouse/) | Lakehouse-mode config and external-asset node (`source` URI) |
| [`sync/`](./sync/) | Leader/follower Git sync configs (bare remote workflow) |
| [`python_agent.py`](./python_agent.py) | Python bindings demo (sync + asyncio) |

> **Note:** Lakehouse HTTP/S3 fetch is stubbed in v0.1 — see [`planning/IMPLEMENTATION_STATUS.md`](../planning/IMPLEMENTATION_STATUS.md).

## Try it

From the repository root:

```bash
cargo test -p maggraph
cargo run -p maggraph-cli -- query --from welcome --depth 2 --config examples/basic/maggraph.toml
cargo run -p maggraph-cli -- ui --config examples/basic/maggraph.toml
cargo run -p maggraph-cli -- scaffold --mcp --skill --config examples/basic/maggraph.toml
cargo run -p maggraph-cli -- --config examples/basic/maggraph.toml --init
```

The `--init` flag creates `knowledge_graph/` (if missing) and the optional `.maggraph/` metadata directory.

## Layout

```
examples/basic/
├── maggraph.toml
└── knowledge_graph/
    ├── welcome.md
    └── getting_started.md

examples/lakehouse/
├── maggraph.toml
└── knowledge_graph/
    └── customer_churn_q2.md

examples/sync/
├── README.md
├── leader/
└── follower/
```

`root_path` in each config is resolved **relative to the config file's directory**, not the current working directory.

Use `GraphIndex::open(resolved.root_path)` in Rust to scan all `*.md` nodes under the graph root.

## Sync example

See [`sync/README.md`](./sync/README.md) for leader push / follower pull steps. Backlog: automated e2e for follower `sync init` — [`planning/BACKLOG.md`](../planning/BACKLOG.md) (`T-M2`).

## Python

After `maturin develop` in `python/`:

```bash
python examples/python_agent.py
```

## Related docs

| Doc | Purpose |
|-----|---------|
| [`planning/CLI.md`](../planning/CLI.md) | All CLI commands |
| [`planning/SYNC.md`](../planning/SYNC.md) | Git sync roles |
| [`planning/LAKEHOUSE.md`](../planning/LAKEHOUSE.md) | Lakehouse mode |
| [`planning/TESTING.md`](../planning/TESTING.md) | Running tests against fixtures |
