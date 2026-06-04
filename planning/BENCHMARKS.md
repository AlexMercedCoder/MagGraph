# MagGraph — Benchmarks

Traversal latency on the [`examples/basic`](../examples/basic/) fixture (2 nodes, depth-2 BFS).

## Running locally

```bash
cargo run --release -p maggraph --bench traversal
```

Or via the workspace bench target:

```bash
cargo bench -p maggraph --bench traversal
```

The bench runs 1,000 traversals and prints total time and microseconds per traversal. It fails if the average exceeds **1 ms** (Phase 3 smoke gate).

## CI

The `benchmark` job in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs the bench in release mode on every push to `main` and uploads `target/benchmark.txt` as an artifact.

## Reference numbers (v0.1)

Numbers vary by hardware. On typical CI runners (Ubuntu, release build):

| Metric | Target | Typical |
|--------|--------|---------|
| Per traversal (µs) | &lt; 1,000 | ~10–100 |
| 100 traversals (ms) | &lt; 100 | &lt; 10 |

See also the unit test `basic_example_traversal_under_one_ms` in `maggraph/src/graph.rs`.

## Testing & backlog

| Coverage today | Gap (backlog ID) |
|----------------|------------------|
| In-crate bench fails if avg > 1 ms | `T-L3` — CI bench job doesn't fail on regression (artifact only) |
| In-memory adjacency | `T-F3` — mmap adjacency (Phase 3.4) deferred; re-bench when added |

See [`TESTING.md`](./TESTING.md) and [`BACKLOG.md`](./BACKLOG.md).
