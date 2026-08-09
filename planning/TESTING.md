# MagGraph Testing Guide

**Last updated:** 2026-08-09

## Required Checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --features maggraph/ui -- -D warnings
cargo test --all --features maggraph/ui
RUSTDOCFLAGS="-D warnings" cargo doc -p maggraph --no-deps --features ui
cargo bench -p maggraph --bench traversal
MAGGRAPH_BENCH_SIZES=1000,10000 cargo bench -p maggraph --bench index_scale
```

Python bindings require a built extension:

```bash
cd python
python -m venv .venv
.venv/bin/pip install maturin pytest pytest-asyncio
.venv/bin/maturin develop --release --features python-ext
.venv/bin/pytest -v
```

## Current Layout

| Location | Coverage |
| --- | --- |
| `maggraph/src/**` | Node/index/config/query/recall/graph/lakehouse/sync/security/UI unit tests |
| `maggraph-cli/tests/**` | CLI, scaffold, REST CRUD, live UI, and leader/follower E2E |
| `python/tests/test_maggraph.py` | Python CRUD, traversal, async, lakehouse, and retrieval behavior |
| `python/tests/test_magagent_contract.py` | Stable methods, arguments, result keys/types, memory kinds, and errors MagAgent consumes |
| `python/tests/test_mcp_scaffold.py` | Generated FastMCP scaffold behavior |
| `maggraph/benches/**` | Small traversal and 1K/10K/100K scale performance |

The development baseline executes 122 Rust tests, 33 Python tests (including
parameterized memory-kind contracts), and 3 Rust documentation tests. Exact totals
may differ by feature flags.

## Durability Coverage

- same-directory flushed temporary writes replace complete Markdown files atomically;
- orphan temporary files are ignored by graph scans;
- failed disk deletes retain the current in-memory index entry;
- malformed one-file refreshes retain the last valid indexed content;
- repeated merges preserve all provenance;
- retry after target-write/source-delete interruption does not duplicate merged text;
- Git conflicts and follower write policies are exercised at unit and CLI levels.

Process-kill fault injection and journaled multi-node transaction recovery remain in
the current backlog.

## Environment Notes

The live UI integration test binds `127.0.0.1`; restricted sandboxes that prohibit
socket creation must run the suite outside that network namespace. Handler-level UI
tests remain in-process and do not require a port.

Scale benchmarks write many temporary Markdown files. The 100K tier is opt-in so normal
pull requests stay economical. Release validation should run it on consistent hardware.

See [`SUPPORT_MATRIX.md`](./SUPPORT_MATRIX.md), [`BENCHMARKS.md`](./BENCHMARKS.md), and
[`BACKLOG.md`](./BACKLOG.md).
