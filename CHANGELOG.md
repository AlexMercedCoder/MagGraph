# Changelog

All notable changes to MagGraph are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-06

### Added

- Graph-native structured search over ids, node types, tags, frontmatter, body text, links, suppression state, and recency.
- Incremental index helpers: `GraphIndex::update_file` and `changed_since` for fast agent memory writes and promotion flows.
- Agent memory schema helpers for `preference`, `project_fact`, `decision`, `task`, `session_summary`, `bookmark`, and `tool_failure`.
- Reverse-edge backlinks and incoming edge iteration.
- Memory quality primitives: suppress, unsuppress, and merge nodes while preserving provenance.
- Agent-grade recall bundles with summary, body excerpt, links, backlinks, metadata, relevance reason, and Markdown rendering.
- CLI `maggraph search` and `maggraph recall` commands with Markdown and JSON output.
- Python bindings and type stubs for search, backlinks, change feed, incremental file update, memory node creation, suppress/unsuppress, merge, and recall bundles.

### Documentation

- Updated README, Python guide, CLI guide, and implementation status for the new agent-facing APIs.
- Refreshed stale Python LakehouseReader status; it is now exposed and covered by pytest.

## [0.1.0] - 2026-06-04

First public release — local markdown graph, lakehouse pointers, Git sync, CLI, Python bindings, agent artifacts, and embedded UI.

### Added

- **`maggraph.toml` configuration** — `[storage]`, `[lakehouse]`, `[sync]` with validation and graph root initialization.
- **Markdown node model** — YAML frontmatter, wikilinks, in-memory `GraphIndex`, CRUD.
- **Graph traversal** — BFS/DFS to depth N, Markdown reports, directed adjacency from frontmatter `links` and body wikilinks.
- **Lakehouse mode** — `ContentResolver` for `file://`, `s3://`, `http(s)://` (stubs for remote), TTL/size cache.
- **Git sync** — leader/follower roles, `maggraph sync`, `.maggraph/lock.toml` write lock.
- **CLI** — `query`, `sync`, `scaffold`, `ui`, `init`, `complete`; global `--config` and `-v` tracing.
- **Python bindings** — PyO3 module with sync and asyncio APIs; maturin wheels in CI.
- **Agent artifacts** — schema introspection, `SKILL.md` generation, FastMCP server scaffold.
- **Embedded UI** — loopback-only Axum dashboard with REST API and markdown editor.
- **Security hardening** — path traversal checks, file allowlist, HTTP SSRF host blocking.
- **Release pipeline** — GitHub Actions release workflow for CLI binaries and Python wheels.

### Documentation

- Planning folder with architecture, implementation plan, and phase progress tracker.
- `planning/SECURITY.md`, `planning/BENCHMARKS.md`, and per-feature guides (CLI, Python, MCP, UI, sync, lakehouse).

[0.2.0]: https://github.com/AlexMercedCoder/MagGraph/releases/tag/v0.2.0
[0.1.0]: https://github.com/AlexMercedCoder/MagGraph/releases/tag/v0.1.0
