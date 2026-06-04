# MagGraph — Post-v0.1 Backlog

Open work identified in the **v0.1 audit** (2026-06-04). Phases 0–10 are complete; this file tracks hardening, test coverage, documentation, and PRD follow-ups for **v0.1.1+**.

**How to use:** Pick an ID, implement, then update status here and add a line to the changelog in [`PROGRESS.md`](./PROGRESS.md).

**Status legend:** ⬜ Not started · 🟡 In progress · ✅ Done · ⏸️ Blocked / deferred

---

## Testing — high priority

User-facing paths with little or no automated coverage today.

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T-H1 | UI REST CRUD integration tests | ⬜ | Extend `maggraph-cli/tests/ui_integration.rs`: `POST /api/nodes`, `PATCH`, `DELETE`, `GET /api/edges`; reject path traversal via API |
| T-H2 | MCP CRUD tool tests | ⬜ | Extend `python/tests/test_mcp_scaffold.py`: call `create_node`, `update_node`, `delete_node` on tmp graph |
| T-H3 | CLI `maggraph complete` smoke test | ⬜ | Assert non-empty stdout for at least `bash` (optionally zsh, fish, elvish, powershell) |
| T-H4 | Follower write rejection (CLI / e2e) | ⬜ | E2E: follower `sync push` or write via policy fails after leader seed; today only follower *query* is checked |

---

## Testing — medium priority

Feature slices with partial coverage; regressions possible but lower blast radius.

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T-M1 | Golden test for `query --order dfs` | ⬜ | Only BFS has a snapshot today |
| T-M2 | E2E for `maggraph init --skill` and follower `sync init` | ⬜ | Leader `init --git` covered; `--skill` and follower clone path are manual-only |
| T-M3 | Sync conflict CLI path | ⬜ | `merge_conflict_surfaces_paths` exists in `sync/repo.rs`; no CLI test that `sync pull` prints conflict paths |
| T-M4 | Python lakehouse tests | ⬜ | No pytest for lakehouse mode; bindings don't expose `LakehouseReader` yet — see [`PYTHON.md`](./PYTHON.md) |
| T-M5 | `GraphIndex::read_node_with_content` API test | ⬜ | Public helper; only exercised inside lakehouse module tests |

---

## Testing — low priority

Quality and maintenance improvements.

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T-L1 | Rust doc tests on public API | ⬜ | Add `///` examples on `GraphIndex`, `traverse`, `MagGraphConfig` in `lib.rs` |
| T-L2 | Coverage tooling in CI | ⬜ | e.g. `cargo-llvm-cov` or tarpaulin; publish summary artifact |
| T-L3 | Benchmark CI regression gate | ⬜ | Bench job uploads artifact but doesn't fail on slowdown; in-crate 1 ms assertion remains |

---

## Documentation

Index fixes, discoverability, and stale user-facing docs.

| ID | Task | Status | Notes |
|----|------|--------|-------|
| D-1 | `planning/README.md` index | ✅ | Includes TESTING, BACKLOG, SECURITY, BENCHMARKS, IMPLEMENTATION_STATUS |
| D-2 | `planning/TESTING.md` | ✅ | Test layout, commands, gap summary |
| D-3 | Root `README.md` planning links | ✅ | Feature guides + TESTING/BACKLOG/IMPLEMENTATION_STATUS linked |
| D-4 | `examples/README.md` refresh | ✅ | Added sync/, query/ui/scaffold, lakehouse note |
| D-5 | `planning/PYTHON.md` lakehouse note | ✅ | Document Rust-only content resolution |
| D-6 | `planning/MCP.md` security cross-link | ✅ | Local-only, no auth; link [`SECURITY.md`](./SECURITY.md) |
| D-7 | `planning/IMPLEMENTATION_STATUS.md` | ✅ | PRD vs shipped behavior (stubs, deferred) |
| D-8 | `CONTRIBUTING.md` | ⬜ | Extract contributing steps from root README |
| D-9 | Rust API reference publishing | ⬜ | Optional CI `cargo doc` job or docs.rs prep |
| D-10 | OpenAPI / JSON schema for UI REST | ⬜ | Machine-readable API alongside [`UI.md`](./UI.md) tables |

---

## CI & quality

| ID | Task | Status | Notes |
|----|------|--------|-------|
| C-L1 | Align clippy with test feature set | ⬜ | Run `clippy --features maggraph/ui` explicitly to avoid drift if CLI deps change |
| C-L2 | Python job: optional sync/UI smoke | ⬜ | Currently scaffold import only |

---

## Features — PRD vs implementation

Documented in [`IMPLEMENTATION_STATUS.md`](./IMPLEMENTATION_STATUS.md). Implement + test when prioritized.

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T-F1 | Real HTTP(S) content fetch | ⏸️ | Stub today; add SSRF tests per SECURITY when enabled |
| T-F2 | Real S3 / Parquet analytics fetch | ⏸️ | Metadata MVP only |
| T-F3 | mmap adjacency (Phase 3.4) | ⏸️ | In-memory adjacency passes bench gate |
| T-F4 | Python `LakehouseReader` bindings | ⬜ | Expose content resolution to MCP/agents from Python |
| T-F5 | PyPI publish + install-from-wheel smoke | ⬜ | Release workflow builds wheels; PyPI upload deferred |

---

## Per-feature doc todos

Cross-links added in feature guides; implementation tracked by IDs above.

| Doc | Open items |
|-----|------------|
| [`CLI.md`](./CLI.md) | T-H3, T-M1, T-M2, T-M3 |
| [`UI.md`](./UI.md) | T-H1, D-10 |
| [`MCP.md`](./MCP.md) | T-H2, security note (D-6) |
| [`PYTHON.md`](./PYTHON.md) | T-M4, T-F4 |
| [`SYNC.md`](./SYNC.md) | T-H4, T-M2, T-M3 |
| [`LAKEHOUSE.md`](./LAKEHOUSE.md) | T-M5, T-F1, T-F2 |
| [`BENCHMARKS.md`](./BENCHMARKS.md) | T-L3, T-F3 |
| [`SECURITY.md`](./SECURITY.md) | T-H1 (UI path traversal API), T-F1 |

---

## Suggested PR slices

**Quick wins (1–2 PRs):**

1. D-4 + D-3 — refresh example and root README indexes  
2. T-H1 + T-H2 — UI CRUD + MCP CRUD tests  
3. T-H3 + T-M1 — CLI completion + DFS golden  

**Next slice:**

4. T-H4 + T-M3 — sync edge cases at CLI  
5. T-L1 + D-8 — doc tests + CONTRIBUTING.md  

**When implementing deferred features:**

6. T-F1 + T-F2 — network fetch + integration tests  
7. T-F3 — mmap + benchmark comparison  

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-04 | Created BACKLOG from v0.1 audit; linked from planning index and feature docs |
