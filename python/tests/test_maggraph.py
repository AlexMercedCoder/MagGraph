"""Pytest suite for MagGraph Python bindings."""

from __future__ import annotations

import asyncio
from pathlib import Path

import maggraph
import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
BASIC_CONFIG = REPO_ROOT / "examples" / "basic" / "maggraph.toml"
BASIC_GRAPH = REPO_ROOT / "examples" / "basic" / "knowledge_graph"


def test_load_config_resolves_root_path() -> None:
    config = maggraph.load_config(str(BASIC_CONFIG))
    assert config.storage_mode == "local"
    assert Path(config.root_path) == BASIC_GRAPH.resolve()


def test_open_index_lists_nodes() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    nodes = index.list_nodes()
    assert "welcome" in nodes
    assert "getting_started" in nodes
    assert len(index) >= 2


def test_read_node_returns_body() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    node = index.read_node("welcome")
    assert node.id == "welcome"
    assert node.node_type == "note"
    assert "Welcome" in node.body
    assert node.to_markdown().startswith("---")


def test_traverse_bfs_reaches_neighbors() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    result = index.traverse("welcome", depth=1, order="bfs")
    visited = {n.id for n in result.nodes}
    assert result.start == "welcome"
    assert "getting_started" in visited
    markdown = result.to_markdown(index)
    assert "# MagGraph Traversal Report" in markdown
    assert "### welcome" in markdown


def test_traverse_unknown_start_raises() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    with pytest.raises(maggraph.MagGraphError, match="not found"):
        index.traverse("missing-node", depth=1)


@pytest.mark.asyncio
async def test_traverse_async_does_not_block_event_loop() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    tick = asyncio.Event()

    async def waiter() -> None:
        await tick.wait()

    task = asyncio.create_task(waiter())
    await asyncio.sleep(0)
    assert not task.done()

    result = await index.traverse_async("welcome", depth=2, order="bfs")
    tick.set()
    await task

    assert any(n.id == "getting_started" for n in result.nodes)


@pytest.mark.asyncio
async def test_read_node_async() -> None:
    index = maggraph.open_index(str(BASIC_GRAPH))
    node = await index.read_node_async("welcome")
    assert node.id == "welcome"


def test_config_open_index_helper() -> None:
    config = maggraph.load_config(str(BASIC_CONFIG))
    index = config.open_index()
    assert len(index) >= 2


def test_crud_round_trip(tmp_path: Path) -> None:
    import shutil

    graph = tmp_path / "graph"
    shutil.copytree(BASIC_GRAPH, graph)
    index = maggraph.open_index(str(graph))

    index.create_node("agent_test", node_type="note", body="# Test\n", links=[])
    assert "agent_test" in index.list_nodes()

    index.update_node("agent_test", "# Updated\n")
    assert "Updated" in index.read_node("agent_test").body

    index.delete_node("agent_test")
    assert "agent_test" not in index.list_nodes()
