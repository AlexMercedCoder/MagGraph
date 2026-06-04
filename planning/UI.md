# MagGraph embedded UI

Local web dashboard for auditing nodes, viewing edges, and manual CRUD. Served by `maggraph ui` (Phase 9).

## Start the server

```bash
maggraph ui --config examples/basic/maggraph.toml
```

Default URL: **http://127.0.0.1:8787**

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `127.0.0.1` | Bind address (must be loopback: `127.0.0.1` or `::1`) |
| `--port` | `8787` | TCP port |
| `--dry-run` | — | Print URL and exit without starting |

Press **Ctrl+C** to stop the server.

## Security (MVP)

The UI binds to **loopback only**. Binding to `0.0.0.0` or other public interfaces is rejected at startup. There is no authentication; the dashboard is intended for local development on a trusted machine.

## REST API

Base path: `/api`

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/nodes` | List all nodes (metadata summary) |
| `POST` | `/nodes` | Create a node |
| `GET` | `/nodes/{id}` | Get node metadata + markdown body |
| `PATCH` | `/nodes/{id}` | Update body, type, source, links, or extra fields |
| `DELETE` | `/nodes/{id}` | Delete node |
| `GET` | `/edges` | List resolved edges and unresolved wikilink targets |

### Example

```bash
curl -s http://127.0.0.1:8787/api/nodes | jq
curl -s http://127.0.0.1:8787/api/nodes/welcome | jq .body
```

## Dashboard

Static assets are embedded in the `maggraph` binary (`ui` feature):

- `/` — HTML shell (node list, edge list, editor)
- `/app.js`, `/style.css` — client assets

The browser UI loads nodes and edges from the API and supports viewing/editing markdown bodies with **Save** and **Delete**.

## Implementation

- Rust module: `maggraph::ui` (feature `ui`)
- HTTP stack: Axum + Tower + Tokio
- State: `Arc<Mutex<GraphIndex>>` shared across handlers
- CLI: `maggraph-cli` enables `maggraph/ui` by default
