"""Smoke test for generated MCP server scaffold."""

from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
from pathlib import Path

import maggraph
import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
BASIC_CONFIG = REPO_ROOT / "examples" / "basic" / "maggraph.toml"
MAGGRAPH_BIN = REPO_ROOT / "target" / "debug" / "maggraph"


@pytest.fixture(scope="module")
def mcp_server_dir(tmp_path_factory: pytest.TempPathFactory) -> Path:
    out = tmp_path_factory.mktemp("scaffold")
    bin_path = MAGGRAPH_BIN
    subprocess.run(
        ["cargo", "build", "-p", "maggraph-cli"],
        cwd=REPO_ROOT,
        check=True,
    )
    subprocess.run(
        [
            str(bin_path),
            "scaffold",
            "--mcp",
            "--config",
            str(BASIC_CONFIG),
            "--output",
            str(out),
        ],
        cwd=REPO_ROOT,
        check=True,
    )
    return out / "mcp_server"


def _load_mcp_module(mcp_server_dir: Path, config_path: Path):
    """Import the generated server.py with MAGGRAPH_CONFIG pointing at config_path."""
    os.environ["MAGGRAPH_CONFIG"] = str(config_path)
    spec = importlib.util.spec_from_file_location(
        "mcp_server", mcp_server_dir / "server.py"
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_mcp_server_import_and_tools(mcp_server_dir: Path) -> None:
    module = _load_mcp_module(mcp_server_dir, BASIC_CONFIG)

    nodes = module.list_nodes()
    assert "welcome" in nodes

    md = module.get_node("welcome")
    assert "Welcome" in md

    report = module.traverse_graph("welcome", depth=1, order="bfs")
    assert "welcome" in report.lower() or "Welcome" in report


# T-H2: MCP CRUD tool tests — create_node, update_node, delete_node
def test_mcp_create_node(mcp_server_dir: Path, tmp_path: Path) -> None:
    """create_node tool writes a new node and list_nodes includes it."""
    graph = tmp_path / "graph"
    shutil.copytree(REPO_ROOT / "examples" / "basic" / "knowledge_graph", graph)
    config = tmp_path / "maggraph.toml"
    config.write_text(
        (BASIC_CONFIG).read_text().replace(
            "./knowledge_graph", str(graph)
        )
    )

    module = _load_mcp_module(mcp_server_dir, config)

    result = module.create_node(
        node_id="mcp_test",
        node_type="note",
        body="# MCP Created\n",
        links=[],
    )
    assert "mcp_test" in result or "created" in result.lower() or "mcp_test" in module.list_nodes()
    assert "mcp_test" in module.list_nodes()


def test_mcp_update_node(mcp_server_dir: Path, tmp_path: Path) -> None:
    """update_node tool modifies the body of an existing node."""
    graph = tmp_path / "graph"
    shutil.copytree(REPO_ROOT / "examples" / "basic" / "knowledge_graph", graph)
    config = tmp_path / "maggraph.toml"
    config.write_text(
        (BASIC_CONFIG).read_text().replace(
            "./knowledge_graph", str(graph)
        )
    )

    module = _load_mcp_module(mcp_server_dir, config)

    module.update_node("welcome", body="# Welcome Updated\n")
    detail = module.get_node("welcome")
    assert "Welcome Updated" in detail


def test_mcp_delete_node(mcp_server_dir: Path, tmp_path: Path) -> None:
    """delete_node tool removes a node from the graph."""
    graph = tmp_path / "graph"
    shutil.copytree(REPO_ROOT / "examples" / "basic" / "knowledge_graph", graph)
    config = tmp_path / "maggraph.toml"
    config.write_text(
        (BASIC_CONFIG).read_text().replace(
            "./knowledge_graph", str(graph)
        )
    )

    module = _load_mcp_module(mcp_server_dir, config)

    assert "getting_started" in module.list_nodes()
    module.delete_node("getting_started")
    assert "getting_started" not in module.list_nodes()
