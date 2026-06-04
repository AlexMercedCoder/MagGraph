"""Smoke test for generated MCP server scaffold."""

from __future__ import annotations

import importlib.util
import os
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


def test_mcp_server_import_and_tools(mcp_server_dir: Path) -> None:
    os.environ["MAGGRAPH_CONFIG"] = str(BASIC_CONFIG)
    spec = importlib.util.spec_from_file_location(
        "mcp_server", mcp_server_dir / "server.py"
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    nodes = module.list_nodes()
    assert "welcome" in nodes

    md = module.get_node("welcome")
    assert "Welcome" in md

    report = module.traverse_graph("welcome", depth=1, order="bfs")
    assert "welcome" in report.lower() or "Welcome" in report
