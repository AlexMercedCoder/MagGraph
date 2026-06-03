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
| [`IMPLEMENTATION_PLAN.md`](./IMPLEMENTATION_PLAN.md) | Phased roadmap, dependencies, deliverables, and acceptance criteria |
| [`PROGRESS.md`](./PROGRESS.md) | Living checklist — update status as work lands |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Condensed architecture reference derived from the PRD |
| [`WIKILINKS.md`](./WIKILINKS.md) | Wikilink syntax, edge resolution, and traversal |
| [`LAKEHOUSE.md`](./LAKEHOUSE.md) | Lakehouse mode, URI resolution, and content resolvers |

## How to use this folder

1. Read **ARCHITECTURE.md** for a quick mental model before coding.
2. Follow phases in **IMPLEMENTATION_PLAN.md** in order unless a dependency note says otherwise.
3. After each milestone (PR merge, feature slice, or phase completion), update **PROGRESS.md**:
   - Change task status: `⬜ Not started` → `🟡 In progress` → `✅ Done`
   - Add a one-line note under **Changelog** with date and what changed.
4. Link PRs or issues in PROGRESS notes when helpful (e.g. `Done in #12`).

## Status legend (PROGRESS.md)

| Symbol | Meaning |
|--------|---------|
| ⬜ | Not started |
| 🟡 | In progress |
| ✅ | Done |
| ⏸️ | Blocked / deferred |
| ❌ | Cancelled or out of scope |
