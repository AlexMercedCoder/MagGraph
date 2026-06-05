# Contributing to MagGraph

Thank you for considering a contribution! This document explains how to set up your development environment, run the full test suite, and submit a pull request.

## Prerequisites

| Tool | Minimum version | How to install |
|------|-----------------|---------------|
| Rust | 1.75 (MSRV) | `rustup update stable` |
| Python | 3.11 | [python.org](https://python.org) |
| maturin | 1.x | `pip install maturin` |
| Git | 2.39 | OS package manager |

Optional: **cargo-llvm-cov** (coverage), **cargo-nextest** (faster test runner).

---

## Quick start

```bash
git clone https://github.com/AlexMercedCoder/MagGraph
cd MagGraph

# Build everything (Rust + Python extension in dev mode)
cargo build --all --features maggraph/ui
cd python && maturin develop --features python-ext && cd ..
```

---

## Running the full test suite

### Rust

```bash
# All unit + integration tests (including UI REST tests)
cargo test --all --features maggraph/ui

# Just the library tests
cargo test -p maggraph

# Just CLI integration tests
cargo test -p maggraph-cli --test '*'

# Benchmarks (optional)
cargo bench -p maggraph --bench traversal
```

### Python

```bash
cd python
python -m venv .venv
.venv/bin/pip install maturin pytest pytest-asyncio fastmcp
.venv/bin/maturin develop --features python-ext
.venv/bin/pytest -v
```

### Shell completion smoke test

```bash
cargo build -p maggraph-cli
./target/debug/maggraph complete bash > /dev/null && echo "bash: OK"
./target/debug/maggraph complete zsh  > /dev/null && echo "zsh: OK"
```

---

## Code quality

All CI checks must pass before a PR can be merged.

```bash
# Format (Rust)
cargo fmt --all

# Clippy (must use ui feature to match CI)
cargo clippy --all-targets --features maggraph/ui -- -D warnings

# Build docs (must be warning-free)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features maggraph/ui
```

---

## Project layout

```
MagGraph/
├── maggraph/          # Core library crate
│   ├── src/
│   │   ├── agent/     # Schema introspection, SKILL.md, MCP scaffold
│   │   ├── config.rs  # TOML config loader
│   │   ├── graph.rs   # Traversal (BFS/DFS)
│   │   ├── index.rs   # GraphIndex — CRUD & scan
│   │   ├── lakehouse/ # External content resolution
│   │   ├── node.rs    # Node + frontmatter parsing
│   │   ├── security.rs
│   │   ├── sync/      # Git-backed sync (leader/follower)
│   │   └── ui/        # Axum REST API (feature = "ui")
│   └── benches/
├── maggraph-cli/      # CLI wrapper (assert_cmd integration tests)
│   └── tests/
│       ├── e2e_integration.rs
│       ├── query_integration.rs
│       └── ui_crud_integration.rs
├── python/            # PyO3 bindings (maturin)
│   └── tests/
├── examples/
│   ├── basic/         # Minimal local graph
│   └── sync/          # Leader + follower maggraph.toml examples
├── planning/          # Architecture & design docs (read-only for contributors)
└── .github/workflows/ # CI (rust, python, smoke, benchmark, docs)
```

---

## Submitting a pull request

1. Fork the repository and create a feature branch: `git checkout -b feat/my-feature`
2. Make your changes and add tests. New public APIs must include `///` doc examples.
3. Run the full test suite locally (`cargo test --all --features maggraph/ui`).
4. Ensure `cargo fmt` and `cargo clippy` pass.
5. Open a PR against `main`. The CI pipeline will run automatically.

### PR guidelines

- **One concern per PR.** If you fix a bug and add a feature, split them.
- **Tests are required.** PRs that reduce test coverage will be asked to add tests.
- **Doc examples for public APIs.** New `pub fn` items should include `/// # Example` blocks.
- **No breaking changes to v0.1 public API** without a tracking issue and discussion.

---

## Reporting issues

Use the GitHub issue tracker. Include:
- MagGraph version (`maggraph --version`)
- OS and Rust toolchain version (`rustup show`)
- Minimal reproduction (ideally a `maggraph.toml` + `*.md` files)

---

## License

MagGraph is licensed under the [Apache 2.0 License](../LICENSE).
