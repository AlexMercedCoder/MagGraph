# MagGraph — Testing Guide

How tests are organized, how to run them, what is covered today, and **open gaps** tracked as backlog items in [`BACKLOG.md`](./BACKLOG.md).

**Last updated:** 2026-06-04 (post-v0.1 audit)

---

## Quick commands

From the repository root:

```bash
# Rust library + CLI (includes UI feature)
cargo test --all --features maggraph/ui

# Format + lint (match CI)
cargo fmt --all -- --check
cargo clippy --all-targets --features maggraph/ui -- -D warnings

# Release smoke script (build + basic CLI)
bash scripts/smoke_install.sh

# Traversal benchmark (local gate: avg < 1 ms per traversal)
cargo bench -p maggraph --bench traversal

# Python bindings
cd python
python -m venv .venv && source .venv/bin/activate
pip install maturin pytest pytest-asyncio
maturin develop --release --features python-ext
pytest -v
```

---

## Test layout

| Location | Type | What it covers |
|----------|------|----------------|
| `maggraph/src/**` | Unit tests | Config, nodes, index, wikilinks, graph/traversal, lakehouse, sync, security, agent, UI handlers |
| `maggraph/benches/traversal.rs` | Benchmark | 1,000 traversals on `examples/basic`; fails if avg > 1 ms |
| `maggraph-cli/tests/` | Integration | Golden `query` output, scaffold smoke, UI GET endpoints |
| `maggraph-cli/tests/e2e_integration.rs` | E2E | init → query → scaffold; leader push / follower pull |
| `python/tests/` | pytest | Config, index, traverse, async, CRUD, MCP scaffold import |
| `scripts/smoke_install.sh` | Smoke | Release build + CLI help/query |
| `.github/workflows/ci.yml` | CI | fmt, clippy, test, benchmark artifact, Python job |

Approximate counts (v0.1): **~90 Rust tests** (lib + CLI integration), **~10 Python tests**.

---

## Coverage by area

### Strong coverage

| Area | Tests | Notes |
|------|-------|-------|
| Config, nodes, index | Unit | Validation, CRUD, duplicate IDs, round-trip |
| Wikilinks, adjacency, traversal | Unit + golden CLI | BFS golden snapshot for `maggraph query` |
| Lakehouse | Unit | URI rules, resolvers, cache, S3 stub, file allowlist |
| Sync | Unit + E2E | Leader/follower policy, lock, merge conflict (repo layer) |
| Security | Unit | Path traversal, HTTP host blocklist, file allowlist |
| Agent / scaffold | Unit | Schema introspection, SKILL/MCP file generation |
| Python (local mode) | pytest | Sync + asyncio, CRUD round-trip |

### Partial or missing coverage

See [`BACKLOG.md`](./BACKLOG.md) for the full prioritized todo list. Summary:

| Priority | Gap | Backlog ID |
|----------|-----|------------|
| High | UI REST POST/PATCH/DELETE, edges, path traversal via API | `T-H1` |
| High | MCP CRUD tools (`create_node`, `update_node`, `delete_node`) | `T-H2` |
| High | CLI `maggraph complete` (non-empty output per shell) | `T-H3` |
| High | Follower write rejection at CLI / sync push | `T-H4` |
| Medium | `maggraph query --order dfs` golden test | `T-M1` |
| Medium | `maggraph init --skill`, follower `sync init` | `T-M2` |
| Medium | `maggraph sync pull` conflict path reporting (CLI) | `T-M3` |
| Medium | Python lakehouse / content resolution | `T-M4` |
| Medium | `GraphIndex::read_node_with_content` as public API | `T-M5` |
| Low | Rust doc tests (`///` examples on public API) | `T-L1` |
| Low | Coverage tooling in CI (`cargo-llvm-cov`) | `T-L2` |
| Low | Benchmark CI regression gate | `T-L3` |

---

## Intentionally untested (deferred features)

These are **not bugs** — behavior is stubbed or deferred by design. Add tests when the feature is implemented.

| Feature | Status | When to test |
|---------|--------|--------------|
| Real HTTP(S) content fetch | Stub (metadata only) | `T-F1` in BACKLOG — SSRF + integration per [`SECURITY.md`](./SECURITY.md) |
| Real S3 fetch | Stub (metadata + mocked tests) | With live or localstack integration |
| mmap adjacency (Phase 3.4) | Deferred | Benchmark before/after; update [`BENCHMARKS.md`](./BENCHMARKS.md) |
| PyPI-published `maggraph` wheel | CI artifacts only | Install-from-wheel smoke in release job |

---

## Writing new tests

### Rust unit tests

Co-locate in the same module (`#[cfg(test)] mod tests`) or in `maggraph/tests/` for cross-module integration. Prefer existing fixtures under `examples/basic/` and `examples/sync/`.

### CLI integration tests

Add cases to `maggraph-cli/tests/` using `assert_cmd` patterns already in the crate. Use temp directories for graph mutations.

### UI integration tests

Extend `maggraph-cli/tests/ui_integration.rs` (or add sibling file). Spin up the Axum router in-process; no need for a live browser.

### Python tests

Add to `python/tests/`. Use `tmp_path` fixtures for isolated graphs. MCP tests should import the generated `mcp_server/server.py` after `maggraph scaffold --mcp`.

### E2E

Leader/follower scenarios belong in `maggraph-cli/tests/e2e_integration.rs`. Keep them fast (local bare git repos in temp dirs).

---

## CI alignment notes

| Job | Feature flags | Backlog |
|-----|---------------|---------|
| `test` | `--features maggraph/ui` | — |
| `clippy` | Should match test flags | `C-L1` — align clippy with `maggraph/ui` explicitly |
| `benchmark` | Release bench; uploads artifact | `T-L3` — optional fail-on-regression threshold |
| `python` | Builds CLI for scaffold smoke | Does not run UI or sync e2e from Python |

---

## Related docs

| Doc | Purpose |
|-----|---------|
| [`BACKLOG.md`](./BACKLOG.md) | All open testing, documentation, and quality todos |
| [`PROGRESS.md`](./PROGRESS.md) | Mark backlog items done in changelog when PRs land |
| [`SECURITY.md`](./SECURITY.md) | Threat model; references security-related tests |
| [`BENCHMARKS.md`](./BENCHMARKS.md) | Traversal bench commands and targets |
