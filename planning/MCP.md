# MagGraph MCP Server

Phase 8 agent artifacts: a FastMCP server scaffold generated from your graph schema, wired to the `maggraph` Python bindings.

## Generate

From the repository root (or your graph project):

```bash
maggraph scaffold --mcp --skill --config maggraph.toml
```

This writes:

| Path | Purpose |
|------|---------|
| `mcp_server/server.py` | FastMCP server with graph tools |
| `mcp_server/requirements.txt` | `fastmcp` dependency |
| `mcp_server/README.md` | Setup and tool list |
| `SKILL.md` | Machine-readable agent manual (with `--skill`) |

Regenerate after adding nodes or changing edge patterns so tool docs stay in sync.

## Install dependencies

`maggraph` is not published to PyPI yet. Install from the repo, then FastMCP:

```bash
cd /path/to/MagGraph/python
python -m venv .venv && source .venv/bin/activate
pip install maturin
maturin develop --release --features python-ext

pip install -r /path/to/your/graph/mcp_server/requirements.txt
```

## Run (stdio)

```bash
export MAGGRAPH_CONFIG="/absolute/path/to/maggraph.toml"
python mcp_server/server.py
```

`MAGGRAPH_CONFIG` defaults to the path baked in at scaffold time.

## Tools

| Tool | Description |
|------|-------------|
| `list_nodes` | Indexed node ids |
| `get_node` | Node as Markdown (frontmatter + body) |
| `traverse_graph` | BFS/DFS traversal report |
| `create_node` | New `{id}.md` node |
| `update_node` | Replace body, keep frontmatter |
| `delete_node` | Remove node from disk and index |

## Cursor / Claude Desktop

Add an MCP server entry pointing at the Python module with stdio transport. Example shape (adjust paths):

```json
{
  "mcpServers": {
    "maggraph": {
      "command": "python",
      "args": ["/path/to/graph/mcp_server/server.py"],
      "env": {
        "MAGGRAPH_CONFIG": "/path/to/graph/maggraph.toml"
      }
    }
  }
}
```

Use the same `python` where `maggraph` is installed (venv recommended).

## CI smoke test

The GitHub Actions `python` job runs `maggraph scaffold --mcp` on the basic example and imports the generated server to verify `maggraph` wiring.

## Security

The MCP server runs locally over stdio with **no authentication**. It inherits the same trust model as the CLI: suitable for a single developer machine, not multi-tenant deployment. See [`SECURITY.md`](./SECURITY.md) for the full threat model (path traversal, future network fetch SSRF).

## Testing & backlog

| Coverage today | Gap (backlog ID) |
|----------------|------------------|
| Smoke: `list_nodes`, `get_node`, `traverse_graph` import | `T-H2` — `create_node`, `update_node`, `delete_node` untested |
| Generated server wired to PyO3 | Regenerate after schema changes (`scaffold --mcp`) |

See [`TESTING.md`](./TESTING.md) and [`BACKLOG.md`](./BACKLOG.md).

## Related docs

- [`planning/PYTHON.md`](./PYTHON.md) — Python API reference
- [`planning/CLI.md`](./CLI.md) — `scaffold` and `init --skill`
- [`SKILL.md`](../SKILL.md) — only after scaffold; lives in your graph root
