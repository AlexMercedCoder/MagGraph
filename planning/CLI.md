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

### `maggraph search`

Search node ids, types, tags, frontmatter, body, links, and recency.

```bash
maggraph search "release checklist" \
  --node-type project_fact \
  --tag magagent \
  --limit 10 \
  --format markdown
```

Use `--format json` for programmatic agent consumers.

### `maggraph recall`

Print a compact retrieval bundle with summary, excerpt, links, backlinks,
metadata, and relevance reason.

```bash
maggraph recall release_process --reason "matched release query"
```

### `maggraph scaffold`

Generate agent-facing artifacts from the indexed graph.

```bash
maggraph scaffold --mcp --skill --config examples/basic/maggraph.toml
```

| Flag | Description |
|------|-------------|
| `--mcp` | Write `mcp_server/` (FastMCP server wired to `maggraph` Python package) under `--output` |
| `--skill` | Write `SKILL.md` into the graph root (schema introspection + operations) |
| `--output` | Directory for MCP output (default: `.`) |

See [MCP.md](./MCP.md) for deployment.

### `maggraph sync`

See [SYNC.md](./SYNC.md).

### `maggraph init`

Initialize graph root; `--git` when `[sync]` is configured; `--skill` writes `SKILL.md` after init.

### `maggraph ui`

Start the embedded local web dashboard. See [UI.md](./UI.md).

```bash
maggraph ui --config examples/basic/maggraph.toml
```

### `maggraph complete`

Emit shell completions: `maggraph complete bash > /tmp/maggraph.bash`.

## Shell completion

```bash
maggraph complete bash >> ~/.bash_completion.d/maggraph
maggraph complete zsh  >> ~/.zfunc/_maggraph
```

## Testing & backlog

| Coverage today | Gap (backlog ID) |
|----------------|------------------|
| Golden `query` (BFS/DFS), `search`, `recall`, scaffold smoke, e2e init/query/scaffold | Add cases as command formats expand |
| Leader `init --git`, follower `sync init`, follower write rejection, conflict paths | Keep sync e2e coverage current |
| Shell completion smoke | Covered for bash, zsh, fish, elvish, powershell |

See [`TESTING.md`](./TESTING.md) and [`BACKLOG.md`](./BACKLOG.md).
