# MagGraph — Implementation Status (PRD vs v0.1)

What shipped in **v0.2.0** versus what [`PRD.md`](../PRD.md) describes long-term. Use this to set expectations for users and agents reading the PRD.

**Last updated:** 2026-06-04

---

## Summary

| Area | v0.1 status | PRD vision |
|------|-------------|------------|
| Local markdown graph | ✅ Shipped | Match |
| Wikilinks + traversal | ✅ Shipped | Match |
| Search, backlinks, recall bundles | ✅ Shipped | Match for agent retrieval MVP |
| `maggraph.toml` config | ✅ Shipped | Match |
| Git sync leader/follower | ✅ Shipped (libgit2) | Match |
| Lakehouse semantic pointers | ✅ Shipped (read path) | Partial — see below |
| CLI (`query`, `sync`, `scaffold`, `ui`) | ✅ Shipped | Match |
| Python bindings | ✅ Shipped, including LakehouseReader and agent retrieval APIs | Match for current local/lakehouse MVP |
| MCP + SKILL.md | ✅ Shipped | Match |
| Embedded UI | ✅ Shipped (localhost) | Match |
| Security hardening | ✅ MVP review | Ongoing — see [`SECURITY.md`](./SECURITY.md) |

---

## Shipped and aligned with PRD

- Markdown nodes with YAML frontmatter (`id`, `type`, `source`, `links`, extensions)
- Graph index: scan, CRUD, duplicate ID detection
- Structured search, backlinks, changed-since, one-file index refresh, recall bundles
- Agent memory helpers for preferences, project facts, decisions, tasks, summaries, bookmarks, and tool failures
- Memory quality operations: suppress, unsuppress, and merge
- Directed edges from frontmatter `links` + body `[[wikilinks]]`
- BFS/DFS traversal with Markdown reports
- Lakehouse **URI resolution** and pluggable resolvers (file, s3, http schemes)
- On-disk cache for external content metadata
- Git init/clone, commit, pull (FF + merge), push, status
- Leader write lock (`.maggraph/lock.toml`) and follower read-only policy
- CLI, PyO3, FastMCP scaffold, local web UI
- SSRF/path defenses at resolution time (defense in depth)

---

## Partial or stubbed (documented gaps)

### Lakehouse content fetch

| Capability | v0.1 | Backlog |
|------------|------|---------|
| `file://` reads | ✅ With allowlist | — |
| `s3://` fetch | Metadata + snippet stub | `T-F2` |
| `http(s)://` fetch | Metadata stub, **no network I/O** | `T-F1` |
| Full Parquet analytics | Metadata MVP (magic, size, snippet) | PRD long-term |
| Python `LakehouseReader` | ✅ Exposed and tested | — |

See [`LAKEHOUSE.md`](./LAKEHOUSE.md) and [`PYTHON.md`](./PYTHON.md).

### Performance

| Capability | v0.1 | Backlog |
|------------|------|---------|
| In-memory adjacency | ✅ | — |
| mmap adjacency (Phase 3.4) | Deferred | `T-F3` |
| Traversal bench gate | &lt; 1 ms avg on basic fixture | `T-L3` for CI regression |

See [`BENCHMARKS.md`](./BENCHMARKS.md).

### Distribution

| Capability | v0.1 | Backlog |
|------------|------|---------|
| GitHub release CLI binaries | ✅ On `v*` tags | — |
| CI Python wheels (artifact) | ✅ | — |
| PyPI publish | Not yet | `T-F5` |
| docs.rs / published Rust API docs | Not yet | `D-9` |

### Agent surfaces

| Capability | v0.1 | Backlog |
|------------|------|---------|
| MCP read tools | ✅ Tested in smoke | — |
| MCP CRUD tools | ✅ Generated and tested | — |
| UI REST CRUD | ✅ Handlers and integration tests | — |
| OpenAPI for UI API | Not yet | `D-10` |

---

## Intentionally out of scope for v0.1

- Multi-user auth on UI or MCP
- Public bind for `maggraph ui` (loopback only)
- Incremental index watch (full rescan)
- Remote git credentials / credential helper integration beyond libgit2 defaults

---

## Related docs

| Doc | Purpose |
|-----|---------|
| [`BACKLOG.md`](./BACKLOG.md) | Trackable todos for gaps above |
| [`TESTING.md`](./TESTING.md) | What is tested today |
| [`PROGRESS.md`](./PROGRESS.md) | Phase completion history |
| [`PRD.md`](../PRD.md) | Full product spec |
