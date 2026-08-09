# MagGraph Documentation

This directory contains the maintained architecture, API, operations, testing, and
roadmap documentation for MagGraph. Markdown graph files remain the product's source
of truth; these documents describe the engine that indexes and serves them.

## Start Here

| Document | Purpose |
| --- | --- |
| [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md) | Current shipped, partial, and deferred behavior |
| [`SUPPORT_MATRIX.md`](./SUPPORT_MATRIX.md) | Rust, Python, CLI, UI, platform, and compatibility guarantees |
| [`BACKLOG.md`](./BACKLOG.md) | Active 0.3 and retrieval roadmap |
| [`TESTING.md`](./TESTING.md) | Required checks, test layout, and durability coverage |
| [`BENCHMARKS.md`](./BENCHMARKS.md) | Traversal and 1K/10K/100K performance methodology |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Core storage, graph, sync, and integration design |

## Feature Guides

| Document | Surface |
| --- | --- |
| [`PYTHON.md`](./PYTHON.md) | PyO3 package, typed API, asyncio, wheel behavior |
| [`CLI.md`](./CLI.md) | Commands, output, and shell completion |
| [`SYNC.md`](./SYNC.md) | Git leader/follower topology and write policy |
| [`LAKEHOUSE.md`](./LAKEHOUSE.md) | URI resolution, content readers, and cache |
| [`MCP.md`](./MCP.md) | Generated FastMCP and skill scaffolds |
| [`UI.md`](./UI.md) | Embedded loopback dashboard and REST API |
| [`SECURITY.md`](./SECURITY.md) | Threat model and mitigations |
| [`WIKILINKS.md`](./WIKILINKS.md) | Link parsing, resolution, and graph edges |
| [`PYPI_RELEASE.md`](./PYPI_RELEASE.md) | Trusted publishing and release workflow |

## Historical Records

[`IMPLEMENTATION_PLAN.md`](./IMPLEMENTATION_PLAN.md) and
[`PROGRESS.md`](./PROGRESS.md) record the phases that produced the 0.1 foundation and
0.2 agent-retrieval release. They are retained for context, but are not the current
roadmap or support statement.

## Maintenance Rules

1. Update the support matrix when a public contract or platform guarantee changes.
2. Add behavior to the implementation status only after tests demonstrate it.
3. Record active work in the current backlog, not the historical phase checklist.
4. Include measured benchmark deltas for retrieval or index changes.
5. Release bottom-up: MagGraph first, then dependent MagAgent and Command Center work.
