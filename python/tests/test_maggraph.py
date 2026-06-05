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


# ─────────────────────────────────────────────────────────────────────────────
# T-M4: LakehouseReader Python API tests
# ─────────────────────────────────────────────────────────────────────────────

def _write_local_graph(tmp_path: Path, *, mode: str = "local", remote_sources: str = "") -> tuple[Path, Path]:
    """Write a minimal graph + config to tmp_path. Returns (config_path, graph_dir)."""
    graph = tmp_path / "graph"
    graph.mkdir()
    (graph / "note.md").write_text(
        '---\nid: "note"\ntype: "note"\n---\n# Hello\n', encoding="utf-8"
    )
    remote_block = f"\n[lakehouse]\nremote_sources = [{remote_sources}]\n" if remote_sources else ""
    config = tmp_path / "maggraph.toml"
    config.write_text(
        f'[storage]\nmode = "{mode}"\nroot_path = "./graph"\n{remote_block}',
        encoding="utf-8",
    )
    return config, graph


def _write_lakehouse_node(graph: Path, *, source: str) -> None:
    """Write a node with an external source URI into graph."""
    (graph / "asset.md").write_text(
        f'---\nid: "asset"\ntype: "external_asset"\nsource: "{source}"\n---\n# Asset\n',
        encoding="utf-8",
    )


def test_lakehouse_reader_local_mode_returns_markdown(tmp_path: Path) -> None:
    """In local mode, read_node_with_content returns kind='local' with the markdown body."""
    config_path, _ = _write_local_graph(tmp_path)
    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    result = reader.read_node(index, "note")

    assert result.node.id == "note"
    assert result.content.kind == "local"
    assert result.content.body is not None
    assert "Hello" in result.content.body
    assert "Hello" in result.content.to_markdown()
    assert repr(result).startswith("NodeWithContent")


def test_lakehouse_reader_via_index_method(tmp_path: Path) -> None:
    """GraphIndex.read_node_with_content delegates to the reader correctly."""
    config_path, _ = _write_local_graph(tmp_path)
    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    result = index.read_node_with_content(reader, "note")

    assert result.node.id == "note"
    assert result.content.kind == "local"


def test_lakehouse_reader_resolved_content_repr(tmp_path: Path) -> None:
    """ResolvedContent.__repr__ includes kind and body length for local content."""
    config_path, _ = _write_local_graph(tmp_path)
    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    content = reader.read_node(index, "note").content
    r = repr(content)
    assert "local" in r
    assert "body_len" in r


def test_lakehouse_reader_resolves_s3_stub(tmp_path: Path) -> None:
    """In lakehouse mode with a mocked S3 remote, source nodes resolve as external_asset."""
    remote_sources = '{ uri = "s3://corp-data/lake", format = "parquet" }'
    config_path, graph = _write_local_graph(
        tmp_path, mode="lakehouse", remote_sources=remote_sources
    )
    _write_lakehouse_node(graph, source="s3://corp-data/lake/churn.parquet")

    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    result = reader.read_node(index, "asset")

    assert result.node.id == "asset"
    assert result.content.kind == "external_asset"
    assert result.content.uri is not None
    assert "s3://" in result.content.uri
    assert result.content.format == "parquet"
    assert result.content.body is None  # external assets have no inline body
    md = result.content.to_markdown()
    assert "External asset" in md or "s3://" in md


def test_lakehouse_reader_caches_result(tmp_path: Path) -> None:
    """Reading the same node twice should populate the cache; cache_len() == 1."""
    remote_sources = '{ uri = "s3://corp-data/lake", format = "parquet" }'
    config_path, graph = _write_local_graph(
        tmp_path, mode="lakehouse", remote_sources=remote_sources
    )
    _write_lakehouse_node(graph, source="s3://corp-data/lake/churn.parquet")

    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    # First read populates cache
    reader.read_node(index, "asset")
    assert reader.cache_len() == 1
    assert reader.cache_bytes() > 0

    # Second read hits cache — len stays at 1
    reader.read_node(index, "asset")
    assert reader.cache_len() == 1


def test_lakehouse_reader_file_uri(tmp_path: Path) -> None:
    """file:// sources resolve to ExternalAsset with Parquet metadata when magic header present."""
    data_dir = tmp_path / "data"
    data_dir.mkdir()
    parquet_file = data_dir / "metrics.parquet"
    parquet_file.write_bytes(b"PAR1" + b"\x00" * 20 + b"PAR1")  # minimal PAR1 magic

    graph = tmp_path / "graph"
    graph.mkdir()
    file_uri = parquet_file.as_uri()  # file:///…
    (graph / "metrics.md").write_text(
        f'---\nid: "metrics"\ntype: "external_asset"\nsource: "{file_uri}"\n---\nPointer\n',
        encoding="utf-8",
    )
    remote_uri = data_dir.as_uri()
    config = tmp_path / "maggraph.toml"
    config.write_text(
        f'[storage]\nmode = "lakehouse"\nroot_path = "./graph"\n\n'
        f'[lakehouse]\nremote_sources = [{{ uri = "{remote_uri}", format = "parquet" }}]\n',
        encoding="utf-8",
    )

    cfg = maggraph.load_config(str(config))
    index = cfg.open_index()
    reader = cfg.open_lakehouse_reader()

    result = reader.read_node(index, "metrics")

    assert result.content.kind == "external_asset"
    assert result.content.format == "parquet"
    assert result.content.parquet_magic_valid is True


@pytest.mark.asyncio
async def test_lakehouse_reader_async(tmp_path: Path) -> None:
    """read_node_async on LakehouseReader returns the same result as the sync version."""
    config_path, _ = _write_local_graph(tmp_path)
    config = maggraph.load_config(str(config_path))
    index = config.open_index()
    reader = config.open_lakehouse_reader()

    result = await reader.read_node_async(index, "note")

    assert result.node.id == "note"
    assert result.content.kind == "local"


