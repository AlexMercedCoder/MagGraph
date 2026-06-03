# Examples

Manual fixtures for loading and validating `maggraph.toml`.

| Directory | Purpose |
|-----------|---------|
| [`basic/`](./basic/) | Local-mode config with a two-node sample graph |
| [`lakehouse/`](./lakehouse/) | PRD-style lakehouse + follower sync config |

## Try it

From the repository root:

```bash
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
```

`root_path` in each config is resolved **relative to the config file's directory**, not the current working directory.
