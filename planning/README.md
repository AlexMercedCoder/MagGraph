# MagGraph Planning

This folder holds implementation planning and progress tracking for [MagGraph](https://github.com/AlexMercedCoder/MagGraph): an in-process, Git-backed graph database for agentic AI with Markdown as the source of truth.

## Source documents

| Document | Location | Purpose |
|----------|----------|---------|
| Product requirements | [`../PRD.md`](../PRD.md) | Authoritative feature and architecture spec |
| Project readme | [`../README.md`](../README.md) | Public repo entry point |

## Planning artifacts

| File | Purpose |
|------|---------|
| [`IMPLEMENTATION_PLAN.md`](./IMPLEMENTATION_PLAN.md) | Phased roadmap (0–10), dependencies, deliverables, acceptance criteria |
| [`PROGRESS.md`](./PROGRESS.md) | Living checklist — update status as work lands |
| [`BACKLOG.md`](./BACKLOG.md) | **Post-v0.1 todos** — testing gaps, docs, CI, PRD follow-ups |
| [`TESTING.md`](./TESTING.md) | How to run tests, coverage map, gap summary |
| [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md) | PRD vs v0.1 shipped behavior (stubs and deferred features) |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Condensed architecture reference derived from the PRD |
| [`WIKILINKS.md`](./WIKILINKS.md) | Wikilink syntax, edge resolution, and traversal |
| [`LAKEHOUSE.md`](./LAKEHOUSE.md) | Lakehouse mode, URI resolution, and content resolvers |
| [`SYNC.md`](./SYNC.md) | Git sync, leader/follower roles, and write lock protocol |
| [`CLI.md`](./CLI.md) | CLI commands, flags, and shell completion |
| [`PYTHON.md`](./PYTHON.md) | PyO3 bindings, asyncio, maturin, and type stubs |
| [`MCP.md`](./MCP.md) | FastMCP server scaffold, deployment, and agent tools |
| [`UI.md`](./UI.md) | Embedded local web dashboard (`maggraph ui`) |
| [`SECURITY.md`](./SECURITY.md) | Threat model, mitigations, and test references |
| [`BENCHMARKS.md`](./BENCHMARKS.md) | Traversal latency benchmarks and CI |

## How to use this folder

1. Read **ARCHITECTURE.md** for a quick mental model before coding.
2. Phases **0–10** are complete for v0.1 — see **PROGRESS.md** for history.
3. For **v0.1.1+** work, pick items from **BACKLOG.md** (IDs like `T-H1`, `D-4`).
4. Before adding tests, read **TESTING.md** for layout and conventions.
5. When explaining PRD vs reality to users, use **IMPLEMENTATION_STATUS.md**.
6. After each milestone, update **PROGRESS.md** changelog and mark backlog items ✅ in **BACKLOG.md**.

## Status legend (PROGRESS.md / BACKLOG.md)

| Symbol | Meaning |
|--------|---------|
| ⬜ | Not started |
| 🟡 | In progress |
| ✅ | Done |
| ⏸️ | Blocked / deferred |
| ❌ | Cancelled or out of scope |
