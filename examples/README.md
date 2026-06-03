# Examples

Manual fixtures for loading `maggraph.toml` and indexing markdown nodes.

| Directory | Purpose |
|-----------|---------|
| [`basic/`](./basic/) | Local-mode config with a two-node sample graph |
| [`lakehouse/`](./lakehouse/) | PRD-style lakehouse + follower sync config and external-asset node |

## Try it

From the repository root:

```bash
cargo test -p maggraph
cargo run -p maggraph-cli -- --config examples/basic/maggraph.toml
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
```

`root_path` in each config is resolved **relative to the config file's directory**, not the current working directory.

Use `GraphIndex::open(resolved.root_path)` in Rust to scan all `*.md` nodes under the graph root.
