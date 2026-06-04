#!/usr/bin/env python3
"""Minimal MagGraph Python agent example (Phase 7).

Run from the repository root after installing the extension:

    cd python && maturin develop --features python-ext
    python examples/python_agent.py
"""

from __future__ import annotations

import asyncio
from pathlib import Path

import maggraph

REPO_ROOT = Path(__file__).resolve().parents[1]


def sync_demo() -> None:
    config_path = REPO_ROOT / "examples" / "basic" / "maggraph.toml"
    config = maggraph.load_config(str(config_path))
    index = config.open_index()

    print(f"Graph root: {config.root_path}")
    print(f"Nodes ({len(index)}): {', '.join(index.list_nodes())}")

    welcome = index.read_node("welcome")
    print(f"\n--- {welcome.id} ({welcome.node_type}) ---")
    print(welcome.body.strip())

    result = index.traverse("welcome", depth=2, order="bfs")
    print("\n--- Traversal (sync) ---")
    print(result.to_markdown(index))


async def async_demo() -> None:
    config_path = REPO_ROOT / "examples" / "basic" / "maggraph.toml"
    index = maggraph.load_config(str(config_path)).open_index()

    welcome, result = await asyncio.gather(
        index.read_node_async("welcome"),
        index.traverse_async("welcome", depth=2, order="bfs"),
    )
    print(f"\n--- Async read: {welcome.id} ---")
    print(f"Reached {len(result.nodes)} nodes from {result.start}")


def main() -> None:
    sync_demo()
    asyncio.run(async_demo())


if __name__ == "__main__":
    main()
