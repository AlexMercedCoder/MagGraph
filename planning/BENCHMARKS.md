# MagGraph Benchmarks

MagGraph has a small traversal latency gate and a generated scale benchmark for index
open, structured search, backlinks, recall bundles, and incremental file refresh.

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
nodes,open_ms,search_ms,backlinks_ms,recall_us,update_us
```

## Development Baseline

Measured 2026-08-09 on the maintainer Linux workstation in release mode. These are a
reference, not portable promises; filesystem and CPU differences matter.

| Nodes | Open | Search | Backlinks | Recall bundle | One-file update |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 21.2 ms | 2.1 ms | 1.4 ms | 1.2 ms | 43 us |
| 10,000 | 199 ms | 23.7 ms | 18.1 ms | 16.5 ms | 70 us |
| 100,000 | 1.74 s | 202 ms | 234 ms | 231 ms | 58 us |

The 0.3 in-memory parsed-content index reduced the 1K search measurement from about
45 ms to 2 ms and backlinks from about 38 ms to 1.4 ms by eliminating repeated full
filesystem scans.

## CI Policy

Ordinary CI runs traversal plus 1K and 10K scale tiers and uploads the complete output.
The 100K tier is explicit/manual to control hosted-runner time and filesystem cost.
CI should reject missing result rows and broad performance-budget violations, while
release reports should compare trends on consistent hardware before declaring a
regression.

Current 10K guardrails are intentionally generous for shared runners:

- open under 5 seconds;
- search under 500 ms;
- backlinks under 500 ms;
- one-file update under 50 ms.

See [`BACKLOG.md`](./BACKLOG.md) for persisted benchmark history and normalized
regression work.
