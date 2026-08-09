# MagGraph Support Matrix

**Last verified:** 2026-08-09 against release 0.3.0.

## Runtime And Distribution

| Surface | Supported versions/platforms | Validation |
| --- | --- | --- |
| Rust library | Stable Rust, 2021 edition | fmt, Clippy, unit/doc tests |
| CLI | Linux, macOS Intel/Apple Silicon, Windows | integration tests and release binaries |
| Python | CPython 3.9-3.14 | typed PyO3 abi3 wheels and clean-wheel smoke |
| Embedded UI | Loopback HTTP only | in-process handler tests and live HTTP integration |
| Git sync | Local/libgit2-compatible remotes | leader/follower and conflict E2E tests |

## Rust And Python Graph Contract

| Capability | Rust | Python | Notes |
| --- | --- | --- | --- |
| Open/rescan/index listing | Yes | Yes | Markdown files remain authoritative |
| Read/create/update/delete | Yes | Yes | Writes are atomic in the 0.3 development line |
| `update_file` | Yes | Yes | Explicit one-file create/update/delete refresh |
| `changed_since` | Yes | Yes | Unix modification-time change records |
| Structured search | Yes | Yes | ID, type, tags/frontmatter, links, body, recency |
| Explainable hybrid retrieval | Yes | Yes | Lexical, graph, recency, optional caller-supplied semantic scores, temporal/project scope |
| Embedding generation | Adapter input | Adapter input | Core accepts normalized scores; callers choose local or provider-backed embeddings |
| Backlinks/traversal | Yes | Yes | Frontmatter links and body wikilinks |
| Recall bundle | Yes | Yes | Bounded body excerpt, metadata, neighbors, reason, Markdown |
| Memory schemas | Yes | Yes | preference, project_fact, decision, task, session_summary, bookmark, tool_failure |
| Memory provenance/temporal context | Yes | Yes | Project, task/session/tool source, extraction, confidence, validity, supersession, canonical identity |
| Suppress/unsuppress/merge | Yes | Yes | Suppression filtering and merge provenance |
| Reviewed memory batches | Yes | Yes | Prevalidated update/suppress/unsuppress/merge with operation rollback and reopen recovery journal |
| Lakehouse reader | Yes | Yes | Local/file content; remote metadata stubs |
| Async conveniences | N/A | Yes | Python wrappers retain sync methods as source of truth |

## Compatibility Policy

Until 1.0, additive fields and methods may appear in minor releases. MagGraph will not
remove or rename Python methods consumed by a released MagAgent version without:

1. behavioral contract coverage in `python/tests/test_magagent_contract.py`;
2. a deprecation note in the changelog;
3. at least one compatible release when practical;
4. coordinated MagGraph-before-MagAgent release validation.

Search and recall dictionaries may gain keys, so consumers should read documented
keys rather than reject additive metadata. Existing keys and value types are pinned by
the MagAgent contract suite.

## Security Boundaries

- Node paths cannot be absolute, escape the graph root, or enter `.maggraph`.
- The embedded UI rejects non-loopback binds.
- Remote content is not fetched unless a resolver explicitly implements and permits it.
- MagGraph stores graph data and provenance; it does not grant agent tool permissions
  or interpret node text as authorization.
- Batch recovery manifests and backups remain under `.maggraph`, are never indexed as
  nodes, and reject paths escaping the graph root.
