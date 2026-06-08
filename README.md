<div align="center">

<img src="docs/assets/brand/maggraph-logo.png" alt="MagGraph logo" width="220">

</div>

# MagGraph

> **Why "MagGraph"?** The name is short for **Magpie** — a Corvid.
> Corvids (ravens, crows, jays, and magpies) are renowned in animal cognition
> research for their remarkable **intelligence**, long-term **memory**, and
> sophisticated **tool use**. MagGraph is built to be the memory and knowledge
> layer for AI agents with those same qualities: a graph that thinks,
> remembers, and uses tools.

[![CI](https://github.com/AlexMercedCoder/MagGraph/actions/workflows/ci.yml/badge.svg)](https://github.com/AlexMercedCoder/MagGraph/actions)
[![PyPI](https://img.shields.io/pypi/v/maggraph)](https://pypi.org/project/maggraph/)
[![Python](https://img.shields.io/pypi/pyversions/maggraph)](https://pypi.org/project/maggraph/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**MagGraph** is a Rust-powered, in-process graph database designed for AI
semantic layers. Knowledge is stored as versioned **Markdown nodes** in your
Git repository; edges emerge automatically from `[[wikilinks]]`; Git provides
sync and versioning; a Python API and auto-generated MCP server make it
immediately consumable by any agent framework.

---

## Mag Ecosystem

MagGraph is part of a local-first AI productivity stack:

- [MagGraph](https://github.com/AlexMercedCoder/MagGraph) — Rust-powered Markdown graph memory, search, backlinks, recall bundles, and Python bindings.
- [MagAgent](https://github.com/AlexMercedCoder/MagAgent) — terminal-native AI coding and productivity agent powered by MagGraph memory.
- [Mag Command Center](https://github.com/AlexMercedCoder/MagCommandCenter) — cross-platform desktop app for MagAgent projects, chat, configuration, memory, plugins, and local workbench views.

---

## Table of Contents

- [Why MagGraph?](#why-maggraph)
- [Mag Ecosystem](#mag-ecosystem)
- [Install](#install)
- [Quickstart — Python](#quickstart--python)
- [Core concepts](#core-concepts)
  - [Node format](#node-format)
  - [Edges](#edges)
  - [maggraph.toml](#maggraphtoml)
- [Python API](#python-api)
  - [Config & index](#config--index)
  - [Reading nodes](#reading-nodes)
  - [Search, backlinks, and recall](#search-backlinks-and-recall)
  - [Traversal](#traversal)
  - [CRUD](#crud)
  - [Async support](#async-support)
  - [LakehouseReader](#lakehousereader)
- [CLI](#cli)
  - [query](#query)
  - [search](#search)
  - [recall](#recall)
  - [init](#init)
  - [scaffold](#scaffold)
  - [ui](#ui)
  - [sync](#sync)
  - [complete](#complete)
- [Lakehouse mode](#lakehouse-mode)
- [Git sync (leader / follower)](#git-sync-leader--follower)
- [MCP server scaffold](#mcp-server-scaffold)
- [Embedded web dashboard](#embedded-web-dashboard)
- [Building from source](#building-from-source)
- [Security model](#security-model)
- [Documentation index](#documentation-index)
- [Contributing](#contributing)
- [License](#license)

---

## Why MagGraph?

Most agent memory systems bolt a vector database onto an LLM and call it done.
MagGraph takes a different approach:

| Property | MagGraph | Typical vector DB approach |
|----------|----------|---------------------------|
| Storage format | Plain Markdown — readable by humans, editors, and LLMs | Opaque binary blobs |
| Versioning | Git — free, distributed, auditable history | Custom or none |
| Edges | Implicit from `[[wikilinks]]` + explicit frontmatter links | Manual or none |
| Traversal | BFS / DFS with depth control | Similarity search only |
| External data | Lakehouse mode: pointer nodes to S3/Parquet/files | — |
| Agent interface | MCP server scaffold + Python bindings + CLI | Python SDK only |
| Deployment | Zero infrastructure — runs in-process | Requires server |

---

## Install

### Python (recommended)

```bash
pip install maggraph
```

Pre-built wheels for **Linux** (manylinux_2_28, x86_64 + aarch64),
**macOS** (Intel + Apple Silicon), and **Windows** (x86_64). No Rust toolchain
needed.

### CLI (from source)

```bash
git clone https://github.com/AlexMercedCoder/MagGraph.git
cd MagGraph
cargo install --path maggraph-cli --features maggraph/ui
```

Pre-built CLI binaries are attached to
[GitHub Releases](https://github.com/AlexMercedCoder/MagGraph/releases).

---

## Quickstart — Python

```python
import maggraph

# 1. Load config + open the graph
config = maggraph.load_config("maggraph.toml")
index  = config.open_index()

# 2. Explore what's in the graph
print(index.list_nodes())
# → ['getting_started', 'research_overview', 'welcome']

# 3. Read a node
node = index.read_node("welcome")
print(node.body)       # full Markdown body
print(node.links)      # ['getting_started']   ← frontmatter links
print(node.node_type)  # 'note'

# 4. Traverse the graph (BFS, depth 2)
result = index.traverse("welcome", depth=2, order="bfs")
for n in result.nodes:
    print(f"  depth {n.depth}: {n.id}  path={n.path}")
# Agent-friendly Markdown report:
print(result.to_markdown(index))

# 5. Create / update / delete
index.create_node("my_finding", node_type="insight",
                  body="# Key Finding\nThe model performs best on…\n",
                  links=["welcome", "research_overview"])
index.update_node("my_finding", "# Updated Finding\nNew evidence shows…\n")
index.delete_node("my_finding")
```

That's all you need to start. Everything below covers the deeper features.

---

## Core concepts

### Node format

Every node is a Markdown file with YAML frontmatter:

```markdown
---
id: "customer_churn_q2"
type: "analysis"
links: ["revenue_model", "product_roadmap"]
source: "s3://corp-data/lake/churn_q2.parquet"
importance: "high"
---
# Customer Churn — Q2 2026

Key insight: churn is concentrated in the 30-day cohort.
See also [[revenue_model]] and [[product_roadmap]].
```

| Frontmatter field | Required | Description |
|-------------------|----------|-------------|
| `id` | ✅ | Unique identifier used by all APIs |
| `type` | ✅ | Semantic category (e.g. `"note"`, `"analysis"`, `"decision"`) |
| `links` | — | Explicit outgoing edges (list of node ids) |
| `source` / `source_uri` | — | External data URI for lakehouse mode |
| Any extra key | — | Preserved as-is; accessible via `node.to_dict()["your_key"]` |

### Edges

Edges are resolved from two sources, automatically combined:

1. **Frontmatter `links`** — explicit list in node metadata
2. **`[[wikilinks]]`** in the Markdown body — `[[target]]`, `[[target|alias]]`

You don't need to maintain both. Many workflows use only wikilinks; structured
pipelines use only frontmatter links. MagGraph handles either.

### maggraph.toml

Minimal config (local mode):

```toml
[storage]
mode = "local"
root_path = "./knowledge_graph"
```

Full config (lakehouse + sync):

```toml
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [
  { uri = "s3://corp-data/lake", format = "parquet" }
]

[lakehouse.cache]
ttl_secs  = 300       # 0 = no expiry
max_bytes = 10485760  # 10 MB

[sync]
role       = "leader"   # or "follower"
remote_url = "git@github.com:your-org/knowledge-graph.git"
```

---

## Python API

### Config & index

```python
import maggraph

# Load from a config file
config = maggraph.load_config("maggraph.toml")
print(config.root_path)     # absolute path to graph dir
print(config.storage_mode)  # "local" or "lakehouse"

# Open index from config (resolves root_path automatically)
index = config.open_index()

# Or open an index directly (no config needed)
index = maggraph.open_index("/path/to/knowledge_graph")

print(len(index))           # number of nodes
print(index.root_path)      # absolute path
```

### Reading nodes

```python
# List all node ids (sorted)
ids = index.list_nodes()

# Read a single node (full metadata + body)
node = index.read_node("welcome")

print(node.id)            # "welcome"
print(node.node_type)     # "note"
print(node.body)          # Markdown body (no frontmatter)
print(node.links)         # ["getting_started"]
print(node.source)        # None (or "s3://…" in lakehouse mode)
print(node.relative_path) # "welcome.md"

# Markdown round-trip (frontmatter + body as string)
print(node.to_markdown())

# As a plain Python dict (all frontmatter fields + body)
d = node.to_dict()
# → {"id": "welcome", "type": "note", "links": [...], "body": "…"}
```

### Traversal

```python
# BFS traversal, depth 2
result = index.traverse("welcome", depth=2, order="bfs")

print(result.start)      # "welcome"
print(result.order)      # "bfs"
print(result.max_depth)  # 2

for node in result.nodes:
    print(f"  {node.id:20s}  depth={node.depth}  path={node.path}")

# Depth-first search
dfs = index.traverse("welcome", depth=3, order="dfs")

# Get a Markdown-formatted traversal report (great for LLM context injection)
report = result.to_markdown(index)
print(report)
# → # MagGraph Traversal Report
#   **Start:** welcome | **Depth:** 2 | **Order:** BFS
#   ### welcome
#   …body…
#   ### getting_started
#   …body…
```

> **Tip for agents:** pass `result.to_markdown(index)` directly as context to
> your LLM call. The report is structured for readability by both humans and
> language models.

### Search, backlinks, and recall

MagGraph can return compact, agent-friendly retrieval packets without requiring
an external vector database:

```python
# Search ids, types, body, links, frontmatter, tags, and recency.
results = index.search("release checklist", node_type="project_fact", limit=5)
for item in results:
    print(item["id"], item["score"], item["matched"])

# Reverse edges: who links to this node?
print(index.backlinks("release_process"))

# Incremental change feed for fast memory refresh.
changed = index.changed_since(1_717_200_000)

# Refresh one changed file without a full rescan.
index.update_file("release_process.md")

# Agent-grade recall bundle with summary, excerpt, links, backlinks, metadata,
# relevance reason, and Markdown rendering.
bundle = index.recall_bundle("release_process", reason="matched release query")
print(bundle["markdown"])
```

For common agent memory records, use typed helpers:

```python
index.create_memory_node(
    "prefers_cli",
    "preference",
    "User prefers CLI-first workflows over config-file editing.",
)
```

Supported memory kinds: `preference`, `project_fact`, `decision`, `task`,
`session_summary`, `bookmark`, and `tool_failure`.

### CRUD

```python
# Create a new node
node = index.create_node(
    "sprint_retro_june",
    node_type="retrospective",
    body="# Sprint Retro — June 2026\n\nWhat went well: …\n",
    links=["team_norms", "roadmap_q3"],
)

# Update the body (frontmatter preserved)
index.update_node("sprint_retro_june", "# Sprint Retro — June 2026\n\nRevised notes…\n")

# Delete
index.delete_node("sprint_retro_june")

# Quality operations for agent memory maintenance
index.suppress_node("stale_fact", reason="superseded")
index.unsuppress_node("stale_fact")
index.merge_nodes("canonical_release_process", "duplicate_release_note")

# Error handling
try:
    index.read_node("does-not-exist")
except maggraph.MagGraphError as e:
    print(f"Caught: {e}")
```

### Async support

All read and traversal operations have async equivalents. Blocking Rust work
runs on a Tokio thread pool — your asyncio event loop is never blocked:

```python
import asyncio, maggraph

async def agent_loop():
    config = maggraph.load_config("maggraph.toml")
    index  = config.open_index()

    # Non-blocking reads
    node   = await index.read_node_async("welcome")
    result = await index.traverse_async("welcome", depth=3, order="bfs")

    print(result.to_markdown(index))

    # LakehouseReader also has an async path
    reader = config.open_lakehouse_reader()
    nwc    = await reader.read_node_async(index, "customer_churn_q2")
    print(nwc.content.to_markdown())

asyncio.run(agent_loop())
```

### LakehouseReader

When `storage_mode = "lakehouse"`, nodes can point at external data sources.
`LakehouseReader` resolves those pointers at read time and caches results:

```python
import maggraph

config = maggraph.load_config("maggraph.toml")  # mode = "lakehouse"
index  = config.open_index()
reader = config.open_lakehouse_reader()

# Resolve a node's external content
result = reader.read_node(index, "customer_churn_q2")

print(result.node.id)          # "customer_churn_q2"
print(result.content.kind)     # "local" | "text" | "external_asset"
print(result.content.uri)      # "s3://corp-data/lake/churn_q2.parquet"
print(result.content.format)   # "parquet"
print(result.content.size_bytes)         # bytes, if known
print(result.content.parquet_magic_valid)# True/False for Parquet files

# Agent-friendly Markdown summary
print(result.content.to_markdown())
# → ## External asset
#   **URI:** `s3://corp-data/lake/churn_q2.parquet`
#   **Format:** `parquet`

# Convenience: call directly on the index
result2 = index.read_node_with_content(reader, "customer_churn_q2")

# Inspect the in-memory content cache
print(reader.cache_len())    # number of cached entries
print(reader.cache_bytes())  # total bytes cached
```

**`content.kind` values:**

| `kind` | `body` | `uri` | `format` | When |
|--------|--------|-------|----------|------|
| `"local"` | ✅ Markdown text | None | None | Local mode, or lakehouse node without `source` |
| `"text"` | ✅ Fetched text | ✅ | None | External text file |
| `"external_asset"` | None | ✅ | ✅ | S3, Parquet, or other binary asset |

---

## CLI

Install once:

```bash
cargo install --path maggraph-cli --features maggraph/ui
# or download a pre-built binary from GitHub Releases
```

All commands accept `--config <path>` (default: `maggraph.toml`).
Logging: `-v` info, `-vv` debug, `-vvv` trace. Also respects `RUST_LOG`.

### query

Traverse the graph and print a Markdown report to stdout:

```bash
maggraph query \
  --from welcome \
  --depth 2 \
  --order bfs \
  --config examples/basic/maggraph.toml
```

| Flag | Default | Description |
|------|---------|-------------|
| `--from` | required | Start node id |
| `--depth` | `2` | Max traversal hops (start node = depth 0) |
| `--order` | `bfs` | `bfs` (breadth-first) or `dfs` (depth-first) |

### search

Search nodes and print Markdown or JSON:

```bash
maggraph search "release checklist" \
  --node-type project_fact \
  --tag magagent \
  --limit 10 \
  --format markdown
```

| Flag | Default | Description |
|------|---------|-------------|
| positional query | `""` | Text searched across ids, types, links, frontmatter, and body |
| `--node-type` | none | Filter by type |
| `--tag` | repeatable | Require one or more tags |
| `--include-suppressed` | false | Include nodes with `suppressed: true` |
| `--modified-since-unix` | none | Recency filter |
| `--format` | `markdown` | `markdown` or `json` |

### recall

Render a compact agent retrieval bundle:

```bash
maggraph recall release_process \
  --reason "matched release query" \
  --body-chars 1200
```

Use `--format json` for programmatic consumers.

### init

Initialize a graph root directory. With `--git`, also initialises a Git
repository (required when `[sync]` is configured).

```bash
maggraph init --config maggraph.toml
maggraph init --git --config maggraph.toml

# Also write a SKILL.md agent manual into the graph root
maggraph init --skill --config maggraph.toml
```

### scaffold

Generate agent-facing artifacts from your graph's schema:

```bash
maggraph scaffold \
  --mcp \
  --skill \
  --output ./agent_artifacts \
  --config maggraph.toml
```

| Flag | Description |
|------|-------------|
| `--mcp` | Write a ready-to-run FastMCP server under `--output/mcp_server/` |
| `--skill` | Write `SKILL.md` into the graph root (schema introspection + operation docs) |
| `--output` | Destination directory (default: `.`) |

### ui

Start the embedded local web dashboard (loopback only):

```bash
maggraph ui --config maggraph.toml --port 8787
# → http://127.0.0.1:8787

maggraph ui --dry-run   # print URL and exit (no server started)
```

### sync

```bash
# Initialise Git sync (leader first time, or follower clone)
maggraph init --git --config maggraph.toml
maggraph sync init --config follower/maggraph.toml  # follower: clones remote

# Check status
maggraph sync status --config maggraph.toml

# Leader: commit all changes and push
maggraph sync push --message "Add Q2 analysis nodes" --config maggraph.toml

# Follower: pull latest (read-only)
maggraph sync pull --config follower/maggraph.toml
```

### complete

Generate shell completion scripts:

```bash
maggraph complete bash >> ~/.bash_completion.d/maggraph
maggraph complete zsh  >> ~/.zfunc/_maggraph
maggraph complete fish >> ~/.config/fish/completions/maggraph.fish
maggraph complete powershell >> $PROFILE
maggraph complete elvish
```

---

## Lakehouse mode

In **lakehouse mode**, graph nodes are lightweight semantic pointers. Bulk data
lives in external stores (S3, local files, HTTP endpoints). MagGraph resolves
the external content at query time and caches it in memory.

**Example node** (`knowledge_graph/churn_analysis.md`):

```markdown
---
id: "churn_analysis"
type: "external_asset"
source: "s3://corp-data/lake/churn_q2.parquet"
---
# Customer Churn Q2

Monthly churn analysis — see linked Parquet file for raw data.
```

**Config:**

```toml
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [
  { uri = "s3://corp-data/lake", format = "parquet" }
]

[lakehouse.cache]
ttl_secs  = 300       # cache TTL in seconds; 0 = no expiry
max_bytes = 10485760  # max cache size (10 MB)
```

**URI resolution rules:**

| `source` value | Resolved to |
|----------------|-------------|
| Absolute URI (`s3://…`, `file://…`, `https://…`) | Used as-is |
| Relative path (`churn_q2.parquet`) | Joined to the first `remote_sources` URI |

**Supported resolvers (v0.1):**

| Scheme | Behavior |
|--------|----------|
| `file://` | Reads file from disk; validates Parquet magic header |
| `s3://` | Metadata + stub (no AWS SDK yet — real fetch coming in v0.2) |
| `http(s)://` | Metadata stub only; SSRF host blocklist enforced |

Register custom resolvers via `ResolverRegistry` for tests or private integrations.

---

## Git sync (leader / follower)

MagGraph uses **Git** (via `libgit2`) to replicate graph state across machines
or environments. The topology is a simple leader/follower model:

| Role | Writes | Reads | Push | Pull |
|------|--------|-------|------|------|
| **leader** | ✅ (with lock) | ✅ | ✅ | ✅ |
| **follower** | ❌ | ✅ | ❌ | ✅ |

**Leader `maggraph.toml`:**

```toml
[storage]
mode = "local"
root_path = "./knowledge_graph"

[sync]
role       = "leader"
remote_url = "git@github.com:your-org/knowledge-graph.git"
```

**Follower `maggraph.toml`:**

```toml
[storage]
mode = "local"
root_path = "./knowledge_graph"

[sync]
role       = "follower"
remote_url = "git@github.com:your-org/knowledge-graph.git"
```

**Typical workflow:**

```bash
# One-time leader init
git init --bare /tmp/shared-graph.git
maggraph init --git --config leader/maggraph.toml
maggraph sync push --message "Initial graph" --config leader/maggraph.toml

# One-time follower setup (clones remote)
maggraph sync init --config follower/maggraph.toml

# Day-to-day
maggraph sync push --message "Add sprint notes" --config leader/maggraph.toml
maggraph sync pull --config follower/maggraph.toml  # follower refresh
```

Conflict resolution is **Git-native**: fast-forward when possible; three-way
merge when both sides diverged. Resolve conflicts with standard Git tooling.

---

## MCP server scaffold

MagGraph can generate a ready-to-run **Model Context Protocol** server from
your graph's schema, exposing graph operations as MCP tools that any MCP-compatible
agent framework (Claude Desktop, Cursor, custom agents) can call directly.

**Generate:**

```bash
maggraph scaffold --mcp --skill --output ./agent --config maggraph.toml
```

Creates:

```
agent/
├── mcp_server/
│   ├── server.py          ← FastMCP server, wired to maggraph Python package
│   ├── requirements.txt   ← fastmcp
│   └── README.md
└── SKILL.md               ← Machine-readable graph schema + operation docs
```

**Install and run:**

```bash
pip install maggraph fastmcp
export MAGGRAPH_CONFIG="/absolute/path/to/maggraph.toml"
python agent/mcp_server/server.py
```

**Available MCP tools:**

| Tool | Description |
|------|-------------|
| `list_nodes` | Return all indexed node ids |
| `get_node` | Full node as Markdown (frontmatter + body) |
| `traverse_graph` | BFS/DFS traversal report |
| `create_node` | Write a new node to disk and index |
| `update_node` | Replace node body (frontmatter preserved) |
| `delete_node` | Remove node from disk and index |

**Claude Desktop / Cursor config:**

```json
{
  "mcpServers": {
    "maggraph": {
      "command": "python",
      "args": ["/path/to/agent/mcp_server/server.py"],
      "env": {
        "MAGGRAPH_CONFIG": "/path/to/maggraph.toml"
      }
    }
  }
}
```

---

## Embedded web dashboard

The `maggraph ui` command starts a local web dashboard for auditing your graph:

```bash
maggraph ui --config maggraph.toml
# → http://127.0.0.1:8787
```

- Browse and search all nodes
- View and edit node bodies (Markdown)
- Visualise edges
- REST API at `/api/nodes`, `/api/edges`

The dashboard binds to **loopback only** (`127.0.0.1` or `::1`). Binding to
public interfaces is rejected at startup. There is no authentication — intended
for local development use.

**REST API (also usable from curl / scripts):**

```bash
# List all nodes
curl http://127.0.0.1:8787/api/nodes | jq

# Read a node
curl http://127.0.0.1:8787/api/nodes/welcome | jq

# Create a node
curl -X POST http://127.0.0.1:8787/api/nodes \
  -H 'Content-Type: application/json' \
  -d '{"id":"new_node","type":"note","body":"# New\n","relative_path":"new_node.md","links":[]}'

# Update
curl -X PATCH http://127.0.0.1:8787/api/nodes/new_node \
  -H 'Content-Type: application/json' \
  -d '{"body":"# Updated\n"}'

# Delete
curl -X DELETE http://127.0.0.1:8787/api/nodes/new_node

# List edges
curl http://127.0.0.1:8787/api/edges | jq
```

The full OpenAPI 3.1 spec is at [`docs/openapi.yaml`](./docs/openapi.yaml).

---

## Building from source

Prerequisites: **Rust stable** (≥ 1.75), **Python 3.9+**, **maturin ≥ 1.7**.

```bash
git clone https://github.com/AlexMercedCoder/MagGraph.git
cd MagGraph

# Build everything (Rust + Python extension)
cargo build --all --features maggraph/ui

# Run all tests
cargo test --all --features maggraph/ui

# Build the Python extension in dev mode
cd python && maturin develop --features python-ext

# Run the Python test suite
cd python && pytest -v

# Benchmark traversal
cargo bench -p maggraph --bench traversal
```

### Crate features

| Feature | Enables |
|---------|---------|
| `maggraph/ui` | Axum embedded dashboard (`maggraph ui`) |
| `maggraph/python` | PyO3 module (used by maturin) |
| `maggraph/python-ext` | Python extension module for wheel builds |

---

## Security model

MagGraph is designed for **local development** and **single-operator** use.

| Surface | Mitigation |
|---------|------------|
| **Node path traversal** | `validate_relative_node_path()` blocks `..`, absolute paths, and `.maggraph/` directory access on every create |
| **`file://` reads** | `FileResolver` requires an allowlist derived from `[lakehouse].remote_sources`; paths are canonicalised and checked against allowed roots |
| **HTTP SSRF** | HTTP/HTTPS resolvers are stubs in v0.1; `validate_http_uri_host()` blocks loopback, RFC1918, and link-local addresses |
| **UI exposure** | Binds to loopback only; public-interface binding rejected at startup |
| **Follower writes** | `[sync].role = "follower"` enforces read-only via `WritePolicy` at the engine level |

See [`planning/SECURITY.md`](./planning/SECURITY.md) for the full threat model.

---

## Documentation index

| Document | Description |
|----------|-------------|
| **[planning/ARCHITECTURE.md](./planning/ARCHITECTURE.md)** | System design, data model, agent integration surface |
| **[planning/PYTHON.md](./planning/PYTHON.md)** | Python bindings API reference |
| **[planning/CLI.md](./planning/CLI.md)** | All CLI commands and flags |
| **[planning/SYNC.md](./planning/SYNC.md)** | Git sync topology, write lock, conflict resolution |
| **[planning/LAKEHOUSE.md](./planning/LAKEHOUSE.md)** | Lakehouse mode, URI resolution, resolvers |
| **[planning/MCP.md](./planning/MCP.md)** | MCP server scaffold and tool reference |
| **[planning/UI.md](./planning/UI.md)** | Embedded dashboard, REST API reference |
| **[planning/SECURITY.md](./planning/SECURITY.md)** | Threat model and mitigations |
| **[planning/BENCHMARKS.md](./planning/BENCHMARKS.md)** | Traversal latency benchmarks |
| **[planning/TESTING.md](./planning/TESTING.md)** | Test layout, commands, coverage gaps |
| **[planning/IMPLEMENTATION_STATUS.md](./planning/IMPLEMENTATION_STATUS.md)** | PRD vs v0.1 shipped behaviour |
| **[planning/BACKLOG.md](./planning/BACKLOG.md)** | Post-v0.1 open work |
| **[planning/PROGRESS.md](./planning/PROGRESS.md)** | Phase completion tracker |
| **[planning/PYPI_RELEASE.md](./planning/PYPI_RELEASE.md)** | PyPI Trusted Publishing setup and release workflow |
| **[docs/openapi.yaml](./docs/openapi.yaml)** | OpenAPI 3.1 spec for the REST API |
| **[CONTRIBUTING.md](./CONTRIBUTING.md)** | Dev setup, test commands, PR guidelines |
| **[CHANGELOG.md](./CHANGELOG.md)** | Release history |
| **[PRD.md](./PRD.md)** | Product requirements document (canonical) |

---

## Contributing

See **[CONTRIBUTING.md](./CONTRIBUTING.md)** for the full guide. The short version:

```bash
# Format + lint
cargo fmt --all
cargo clippy --all-targets --features maggraph/ui -- -D warnings

# All tests must pass
cargo test --all --features maggraph/ui
cd python && pytest -v

# Build docs (must be warning-free)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features maggraph/ui
```

PRs that add public APIs must include `///` doc examples. New features need tests.

---

## License

Dual-licensed under **MIT OR Apache-2.0** — your choice.

- [LICENSE-MIT](./LICENSE-MIT)
- [LICENSE-APACHE](./LICENSE-APACHE)
