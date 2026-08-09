# MagGraph Current Backlog

**Last audited:** 2026-08-09
Historical v0.1 completion records live in [`PROGRESS.md`](./PROGRESS.md).

## 0.3 Priority

| ID | Work | Status |
| --- | --- | --- |
| MG-301 | Atomic node replacement and disk-before-index deletion | Complete in development |
| MG-302 | Preserve valid index state after malformed incremental updates | Complete in development |
| MG-303 | Retry-safe merge provenance | Complete in development |
| MG-304 | Cache parsed bodies/summaries/wikilinks for retrieval performance | Complete in development |
| MG-305 | MagAgent-facing Python behavioral contract tests | Complete in development |
| MG-306 | 1K/10K/100K index benchmark harness and CI artifact | Complete in development |
| MG-307 | Publish current support matrix and replace stale v0.1 status docs | Complete in development |
| MG-308 | Persist benchmark history and define hardware-normalized regression comparison | Complete in development |
| MG-309 | Add process-kill recovery fixture for writes and multi-step merges | Complete in development through durable recovery journal fixture |

## Retrieval Next

| ID | Work | Priority |
| --- | --- | --- |
| MG-R1 | Versioned persisted lexical index with incremental rebuild | Deferred: 100K open/search budgets pass; add only when measurements justify disk-index complexity |
| MG-R2 | Native hybrid ranking across lexical, graph, recency, type, project, and embeddings | Complete in development |
| MG-R3 | Temporal validity and canonical identity (`valid_from`, `valid_until`, `supersedes`) | Complete in development |
| MG-R4 | Richer per-result provenance and score explanation | Complete in development |
| MG-R5 | Atomic graph batches with rollback and merge provenance | Complete in development; process-kill journal remains MG-309 |
| MG-R6 | Optional filesystem watcher feeding the incremental change API | Deferred: `update_file`/`changed_since` cover current embedded clients without a resident watcher |

## Quality And Distribution

| ID | Work | Priority |
| --- | --- | --- |
| MG-Q1 | Run the Python contract suite against the oldest supported MagAgent release | Release-operations gate |
| MG-Q2 | Add memory/retrieval precision fixtures shared with MagAgent evals | Complete in development |
| MG-Q3 | Add Windows-specific atomic replacement and locked-file tests | Packaged Windows acceptance gate |
| MG-Q4 | Publish generated Rust API docs or docs.rs metadata | Complete in CI artifact; docs.rs publication follows release |
| MG-Q5 | Add release performance report with artifact hashes and benchmark deltas | Complete in development for benchmark deltas; release workflow adds artifact hashes |

## Deferred Product Work

- Real HTTP and S3 content retrieval requires explicit credentials, SSRF controls,
  bounded downloads, and integration fixtures.
- mmap adjacency is not justified while the cached in-memory index remains within
  documented performance and memory budgets.
- Hosted accounts, cloud synchronization, and multi-user UI authentication are not
  priorities before local durability and stable APIs.
