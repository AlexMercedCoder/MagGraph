# MagGraph Implementation Status

**Last audited:** 2026-08-09
**Release:** 0.3.0
**Next target:** persisted hybrid retrieval and transaction journals

This is the current PRD-to-implementation summary. The original phase checklist in
[`IMPLEMENTATION_PLAN.md`](./IMPLEMENTATION_PLAN.md) and completion log in
[`PROGRESS.md`](./PROGRESS.md) are retained as historical records.

## Current Product

MagGraph is a Rust graph engine with Markdown/YAML files as the durable source of
truth. It provides a Rust library, CLI, Python package, FastMCP scaffold, embedded
loopback UI, Git synchronization, and lakehouse content references.

| Surface | Status | Contract |
| --- | --- | --- |
| Markdown node CRUD | Supported | Atomic same-directory replacement; path-safe IDs and files |
| Index and traversal | Supported | Full open/rescan, one-file refresh, BFS/DFS, wikilinks |
| Agent retrieval | Supported | Structured search, backlinks, recall bundles, change feed |
| Agent memory lifecycle | Supported | Seven memory kinds, suppress, unsuppress, merge provenance |
| Python API | Supported and typed | Python 3.9-3.14, PyO3 abi3 wheels, async conveniences |
| Git sync | Supported | Leader/follower policy, local lock, push/pull/conflict reporting |
| Embedded UI | Supported | Loopback-only REST and static dashboard |
| MCP and skills | Scaffolded | Generated FastMCP server and `SKILL.md`; MagGraph is not an MCP host |
| Lakehouse pointers | Partial | Local/file resolution and metadata stubs for S3/HTTP |

The exact Rust, Python, CLI, UI, and distribution guarantees are listed in
[`SUPPORT_MATRIX.md`](./SUPPORT_MATRIX.md).

## 0.3 Release

- Node writes use flushed same-directory temporary files and atomic replacement.
- Disk deletion completes before the in-memory index entry is removed.
- Malformed incremental updates preserve the last valid indexed state.
- Merge provenance accumulates across sources and interrupted merges are retry-safe.
- Parsed bodies, summaries, and wikilinks are retained in the in-memory index so
  search, backlinks, and recall do not repeatedly scan the filesystem.
- Scale benchmarks cover 1K, 10K, and opt-in 100K-node graphs.
- Python behavioral contract tests pin every API currently consumed by MagAgent.

## Partial Or Deferred

| Capability | Current behavior | Direction |
| --- | --- | --- |
| Persisted lexical/vector index | In-memory parsed-content index | Versioned persisted hybrid index |
| Temporal memory | Modification-time filtering | `valid_from`, `valid_until`, `supersedes`, canonical identity |
| Multi-node transactions | Atomic single-node writes; retry-safe merge | Journaled graph batches |
| Filesystem watch | Explicit `update_file` / `rescan` | Optional watcher/change stream |
| HTTP/S3 retrieval | Metadata and policy stubs | Opt-in implementations with SSRF and credential controls |
| Team/cloud graph sync | Git remote configured by user | No hosted control plane planned yet |

## Distribution

- PyPI publishes abi3 wheels for supported platforms and Python 3.9-3.14.
- GitHub releases publish CLI binaries and Python artifacts.
- CI runs Rust formatting, Clippy, tests, coverage, benchmarks, wheel tests, CLI
  smoke tests, and warning-free Rust API docs.

See [`BACKLOG.md`](./BACKLOG.md) for prioritized work rather than using historical
v0.1 checklist IDs as the active roadmap.
