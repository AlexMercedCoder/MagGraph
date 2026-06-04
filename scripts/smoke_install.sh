#!/usr/bin/env bash
# Smoke test: build and exercise MagGraph from a clean checkout (v0.1 release gate).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo build (release)"
cargo build --release -p maggraph-cli

BIN="$ROOT/target/release/maggraph"

echo "==> maggraph --help"
"$BIN" --help | grep -q query

echo "==> maggraph query (basic example)"
"$BIN" query --from welcome --depth 1 --config examples/basic/maggraph.toml | grep -q "MagGraph Traversal Report"

echo "==> cargo test (all features)"
cargo test --all --features maggraph/ui

echo "==> traversal benchmark"
cargo bench -p maggraph --bench traversal 2>&1 | tee target/benchmark.txt

echo "OK: smoke install passed"
