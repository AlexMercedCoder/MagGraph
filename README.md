# MagGraph

Rust-based in-process graph database for AI semantic layers. Markdown is the source of truth; edges come from `[[wikilinks]]`; Git backs sync and versioning.

**Current release:** v0.1.0 — see [CHANGELOG.md](./CHANGELOG.md).

## Install

### CLI (from source)

```bash
git clone https://github.com/AlexMercedCoder/MagGraph.git
cd MagGraph
cargo install --path maggraph-cli --features maggraph/ui
```

Pre-built binaries are attached to [GitHub Releases](https://github.com/AlexMercedCoder/MagGraph/releases) when tagged (`v*`).

### Python

```bash
pip install maturin pytest pytest-asyncio
cd python && maturin develop --release --features python-ext
```

Wheels are built in CI and published on release. See [planning/PYTHON.md](./planning/PYTHON.md).

## Quick start

```bash
cargo run -p maggraph-cli -- query --from welcome --depth 2 --config examples/basic/maggraph.toml
cargo run -p maggraph-cli -- ui --config examples/basic/maggraph.toml
```

Open http://127.0.0.1:8787 for the local dashboard. See [planning/UI.md](./planning/UI.md).

## Build & test

```bash
cargo build
cargo test --all --features maggraph/ui
bash scripts/smoke_install.sh   # release gate smoke test
cargo bench -p maggraph --bench traversal
```

## Project layout

```
MagGraph/
├── maggraph/           # Rust library (config, index, traversal, lakehouse, sync, ui, python)
├── maggraph-cli/       # `maggraph` binary and integration tests
├── python/             # PyO3 package (maturin)
├── examples/           # Sample maggraph.toml configs and graphs
├── planning/           # Architecture, implementation plan, progress, security
├── scripts/            # smoke_install.sh
└── .github/workflows/  # CI, release
```

## Features & flags

| Crate feature | Enables |
|---------------|---------|
| `maggraph/ui` | Axum embedded dashboard (`maggraph ui`) |
| `maggraph/python` | PyO3 module (used by maturin) |
| `maggraph/python-ext` | Python extension module for wheels |

CLI logging: `-v` / `-vv` / `-vvv`, or `RUST_LOG=maggraph=debug`.

## Contributing

1. Fork and branch from `main` (`cursor/<topic>` for cloud agents).
2. `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all --features maggraph/ui`
3. Update [planning/PROGRESS.md](./planning/PROGRESS.md) when completing planned tasks.
4. Dual-licensed under MIT OR Apache-2.0 ([LICENSE-MIT](./LICENSE-MIT), [LICENSE-APACHE](./LICENSE-APACHE)).

## Documentation

| Doc | Description |
|-----|-------------|
| [PRD.md](./PRD.md) | Product requirements and architecture |
| [CHANGELOG.md](./CHANGELOG.md) | Release history |
| [planning/README.md](./planning/README.md) | Planning index |
| [planning/PROGRESS.md](./planning/PROGRESS.md) | Phase completion tracker |
| [planning/BACKLOG.md](./planning/BACKLOG.md) | Post-v0.1 todos (testing, docs, PRD gaps) |
| [planning/TESTING.md](./planning/TESTING.md) | Test layout and coverage gaps |
| [planning/IMPLEMENTATION_STATUS.md](./planning/IMPLEMENTATION_STATUS.md) | PRD vs v0.1 shipped behavior |
| [planning/CLI.md](./planning/CLI.md) | CLI commands and flags |
| [planning/PYTHON.md](./planning/PYTHON.md) | Python bindings |
| [planning/MCP.md](./planning/MCP.md) | MCP server scaffold |
| [planning/UI.md](./planning/UI.md) | Embedded web dashboard |
| [planning/SYNC.md](./planning/SYNC.md) | Git sync and roles |
| [planning/LAKEHOUSE.md](./planning/LAKEHOUSE.md) | Lakehouse content resolution |
| [planning/SECURITY.md](./planning/SECURITY.md) | Threat model and mitigations |
| [planning/BENCHMARKS.md](./planning/BENCHMARKS.md) | Traversal latency benchmarks |

## Planning & progress

Start with [planning/README.md](./planning/README.md). Phases 0–10 are complete for v0.1; open work is in [planning/BACKLOG.md](./planning/BACKLOG.md).
