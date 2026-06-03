# Product Requirements Document: MagGraph
**Core Concept:** An in-process, Git-backed graph database for Agentic AI. MagGraph treats Markdown as the source of truth, enabling a distributed semantic data lake that functions as both a database and a document store.

## 1. Technical Core & Architecture
- **Engine:** Rust-based embedded database using `libgit2` for file-system persistence and versioned synchronization.
- **Data Model:** - **Nodes:** Markdown files with YAML frontmatter (metadata, node type, source pointers).
    - **Edges:** Implicitly defined via `[[wikilinks]]` within the Markdown content.
- **Performance:** Sub-millisecond local graph traversal; mmap-optimized storage.
- **Dual-Mode Operation:** - **Local Mode:** Full storage of content within local `.md` files.
    - **Lakehouse Mode:** Nodes function as semantic pointers to external data (S3/Parquet/etc.) via `source_uri` properties.

## 2. Sync & Consistency
- **Replication:** Single-Writer (Leader) / Multi-Reader (Follower) topology.
- **Sync Engine:** Git-based sync ensures atomic, versioned state management.
- **Conflict Resolution:** Git-native tree merging. The Leader node maintains a `lock.toml` to serialize writes; followers perform read-only operations on local clones.

## 3. Agent Integration & Bindings
- **PyO3 Bindings:** Rust engine exposed to Python via `pyo3`. Uses `pyo3-asyncio` for non-blocking agent reasoning loops.
- **Agent Skill:** Every graph instance generates a `SKILL.md` (machine-readable "Tool Manual").
- **MCP Integration:** CLI `maggraph scaffold --mcp` auto-generates a `FastMCP` Python server tailored to the graph schema.
- **Query Format:** All queries return Markdown-formatted reports optimized for LLM context windows.

## 4. Operational Features
- **Local UI:** Embedded web dashboard for node/edge auditing and manual CRUD.
- **CLI Utility:**
    - `query`: Execute traversals and return formatted Markdown.
    - `scaffold`: Generate MCP server boilerplates.
    - `sync`: Manage Git-based state updates between distributed instances.

## 5. Configuration (maggraph.toml)
```toml
[storage]
mode = "lakehouse" 
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]

[sync]
role = "follower"
remote_url = "git@github.com:org/maggraph-sync.git"
```

## 6. Implementation Schema (Node Example)

```YAML
---
id: "customer_churn_q2"
type: "external_asset"
source: "s3://lake/churn_data.parquet"
importance: 8
links: ["retention_strategy_01"]
---
# Customer Churn Q2 Analysis
```

Nodes act as smart pointers. The graph engine resolves 
external data dynamically when the agent requests content.
