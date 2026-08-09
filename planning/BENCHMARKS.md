# MagGraph Benchmarks

MagGraph has a small traversal latency gate and a generated scale benchmark for index
open, structured and hybrid search, backlinks, recall bundles, and incremental file
refresh.

## Commands

```bash
# Existing small traversal smoke (<1 ms average)
cargo bench -p maggraph --bench traversal

# Normal development tiers
cargo bench -p maggraph --bench index_scale

# Full 1K/10K/100K run
MAGGRAPH_BENCH_FULL=1 cargo bench -p maggraph --bench index_scale

# Explicit tiers
MAGGRAPH_BENCH_SIZES=1000,10000 cargo bench -p maggraph --bench index_scale
```

The scale fixture uses realistic Markdown frontmatter, tags, links, body text, and a
backlink ring. Results are emitted as CSV:

```text
nodes,open_ms,search_ms,hybrid_ms,backlinks_ms,recall_us,update_us
```

## Development Baseline

Measured 2026-08-09 on the maintainer Linux workstation in release mode. These are a
reference, not portable promises; filesystem and CPU differences matter.

| Nodes | Open | Search | Hybrid | Backlinks | Recall bundle | One-file update |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 24.7 ms | 2.2 ms | 6.8 ms | 1.5 ms | 1.4 ms | 65 us |
| 10,000 | 195 ms | 23.3 ms | 68.1 ms | 15.7 ms | 15.9 ms | 64 us |
| 100,000 | 1.60 s | 205 ms | 712 ms | 209 ms | 211 ms | 63 us |

The 0.3 in-memory parsed-content index reduced the 1K search measurement from about
45 ms to 2 ms and backlinks from about 38 ms to 1.4 ms by eliminating repeated full
filesystem scans.

## CI Policy

Ordinary CI runs traversal plus 1K and 10K scale tiers and uploads the complete output.
The 100K tier is explicit/manual to control hosted-runner time and filesystem cost.
CI should reject missing result rows and broad performance-budget violations, while
release reports should compare trends on consistent hardware before declaring a
regression.

`scripts/benchmark_report.py` converts the raw output into
`maggraph.benchmark-report.v1`. The checked-in baseline preserves the reference
measurements, exact per-metric ratios remain visible, and a deliberately broad ratio
gate absorbs normal hosted-runner variation. Replace the baseline only after a
reviewed same-machine run; never normalize a regression by silently moving the file.
The baseline's `manual_tiers` preserves the latest 100K release-hardware observation
without forcing that expensive tier into ordinary pull-request CI.

Current 10K guardrails are intentionally generous for shared runners:

- open under 5 seconds;
- search under 500 ms;
- hybrid search under 1.5 seconds;
- backlinks under 500 ms;
- one-file update under 50 ms.

The CI artifact contains both raw output and the JSON comparison report.
