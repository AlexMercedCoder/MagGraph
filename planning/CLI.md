# MagGraph CLI

User-facing binary: `maggraph` (`maggraph-cli` crate).

## Global flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Path to `maggraph.toml` (default: `maggraph.toml`) |
| `-v` / `-vv` / `-vvv` | Logging: info, debug, trace (default: warn; `RUST_LOG` overrides) |

## Commands

### `maggraph query`

Traverse the graph from a start node and print a Markdown report (stdout).

```bash
maggraph query --from welcome --depth 2 --order bfs \
  --config examples/basic/maggraph.toml
```

| Flag | Default | Description |
|------|---------|-------------|
| `--from` | (required) | Start node id |
| `--depth` | `2` | Max hops (start node at depth 0) |
| `--order` | `bfs` | `bfs` or `dfs` |
| `--format` | `markdown` | Only `markdown` supported |

### `maggraph scaffold`

Generate agent-facing artifacts from the indexed graph.

```bash
maggraph scaffold --mcp --skill --config examples/basic/maggraph.toml
```

| Flag | Description |
|------|-------------|
| `--mcp` | Write `mcp_server/` (FastMCP Python stub) under `--output` |
| `--skill` | Write `SKILL.md` into the graph root |
| `--output` | Directory for MCP output (default: `.`) |

### `maggraph sync`

See [SYNC.md](./SYNC.md).

### `maggraph init`

Initialize graph root; `--git` when `[sync]` is configured.

### `maggraph complete`

Emit shell completions: `maggraph complete bash > /tmp/maggraph.bash`.

## Shell completion

```bash
maggraph complete bash >> ~/.bash_completion.d/maggraph
maggraph complete zsh  >> ~/.zfunc/_maggraph
```
