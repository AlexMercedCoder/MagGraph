#!/usr/bin/env python3
"""
MagGraph smoke-test script.
Exercises: config loading, index ops, traversal, CRUD, lakehouse reader,
async methods, type stubs (repr/dict), and error handling.
"""

import asyncio
import pathlib
import shutil
import tempfile

import maggraph

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
BASIC_CONFIG = REPO_ROOT / "examples" / "basic" / "maggraph.toml"
BASIC_GRAPH  = REPO_ROOT / "examples" / "basic" / "knowledge_graph"

PASS = "  ✅"
FAIL = "  ❌"

def check(label: str, value: bool) -> None:
    print(f"{'  ✅' if value else '  ❌'} {label}")
    if not value:
        raise AssertionError(f"FAILED: {label}")


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 1. Package metadata ═══")
# ─────────────────────────────────────────────────────────────────────────────
print(f"  maggraph version : {maggraph.__version__}")
check("version is 0.1.0", maggraph.__version__ == "0.1.0")

public_api = ["load_config", "open_index", "GraphIndex", "Node", "TraversalResult",
              "LakehouseReader", "NodeWithContent", "ResolvedContent", "MagGraphError"]
for name in public_api:
    check(f"{name} exported", hasattr(maggraph, name))


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 2. Config loading ═══")
# ─────────────────────────────────────────────────────────────────────────────
config = maggraph.load_config(str(BASIC_CONFIG))
print(f"  config repr      : {config!r}")
check("storage_mode is local", config.storage_mode == "local")
check("root_path resolves to graph dir", pathlib.Path(config.root_path) == BASIC_GRAPH.resolve())


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 3. Index — list & read ═══")
# ─────────────────────────────────────────────────────────────────────────────
index = config.open_index()
print(f"  index repr       : {index!r}")
nodes = index.list_nodes()
print(f"  nodes            : {nodes}")
check("at least 2 nodes", len(index) >= 2)
check("'welcome' present", "welcome" in nodes)
check("'getting_started' present", "getting_started" in nodes)

node = index.read_node("welcome")
print(f"  node repr        : {node!r}")
print(f"  body preview     : {node.body[:60]!r}…")
check("node.id == 'welcome'", node.id == "welcome")
check("node.node_type == 'note'", node.node_type == "note")
check("body contains 'Welcome'", "Welcome" in node.body)
check("to_markdown() starts with ---", node.to_markdown().startswith("---"))

d = node.to_dict()
check("to_dict() has 'id' key", "id" in d)
check("to_dict() id == welcome", d["id"] == "welcome")


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 4. BFS traversal ═══")
# ─────────────────────────────────────────────────────────────────────────────
result = index.traverse("welcome", depth=2, order="bfs")
print(f"  traversal repr   : {result!r}")
visited = {n.id for n in result.nodes}
print(f"  visited          : {sorted(visited)}")
check("start node correct", result.start == "welcome")
check("order is bfs", result.order == "bfs")
check("max_depth == 2", result.max_depth == 2)
check("getting_started reachable", "getting_started" in visited)

md = result.to_markdown(index)
check("markdown has Traversal Report header", "# MagGraph Traversal Report" in md)
check("markdown has welcome section", "### welcome" in md)


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 5. DFS traversal ═══")
# ─────────────────────────────────────────────────────────────────────────────
dfs_result = index.traverse("welcome", depth=2, order="dfs")
check("order is dfs", dfs_result.order == "dfs")
check("dfs also reaches getting_started", any(n.id == "getting_started" for n in dfs_result.nodes))


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 6. Error handling ═══")
# ─────────────────────────────────────────────────────────────────────────────
try:
    index.read_node("does-not-exist")
    check("MagGraphError raised for missing node", False)
except maggraph.MagGraphError as e:
    print(f"  caught expected error: {e}")
    check("MagGraphError is subclass of Exception", isinstance(e, Exception))

try:
    index.traverse("also-missing", depth=1)
    check("MagGraphError raised for missing start", False)
except maggraph.MagGraphError:
    check("traverse raises MagGraphError for unknown start", True)


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 7. CRUD round-trip ═══")
# ─────────────────────────────────────────────────────────────────────────────
with tempfile.TemporaryDirectory() as tmp:
    graph_copy = pathlib.Path(tmp) / "graph"
    shutil.copytree(BASIC_GRAPH, graph_copy)
    idx = maggraph.open_index(str(graph_copy))

    # Create
    new_node = idx.create_node(
        "smoke_test_node",
        node_type="note",
        body="# Smoke Test\nCreated by the smoke script.\n",
        links=["welcome"],
    )
    check("create returns Node", isinstance(new_node, maggraph.Node))
    check("created node has correct id", new_node.id == "smoke_test_node")
    check("created node has correct link", "welcome" in new_node.links)
    check("node appears in list_nodes()", "smoke_test_node" in idx.list_nodes())

    # Read back
    read_back = idx.read_node("smoke_test_node")
    check("read-back body matches", "Smoke Test" in read_back.body)

    # Update
    idx.update_node("smoke_test_node", "# Updated Body\n")
    updated = idx.read_node("smoke_test_node")
    check("update persists new body", "Updated Body" in updated.body)

    # Delete
    idx.delete_node("smoke_test_node")
    check("node removed from list after delete", "smoke_test_node" not in idx.list_nodes())
    print("  CRUD round-trip complete")


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 8. LakehouseReader — local mode ═══")
# ─────────────────────────────────────────────────────────────────────────────
reader = config.open_lakehouse_reader()
print(f"  reader repr      : {reader!r}")

nwc = reader.read_node(index, "welcome")
print(f"  NodeWithContent  : {nwc!r}")
print(f"  ResolvedContent  : {nwc.content!r}")
check("nwc.node.id == welcome", nwc.node.id == "welcome")
check("content.kind == local", nwc.content.kind == "local")
check("content.body contains Welcome", nwc.content.body is not None and "Welcome" in nwc.content.body)
check("content.uri is None (local)", nwc.content.uri is None)
check("content.format is None (local)", nwc.content.format is None)
check("content.to_markdown() has body", "Welcome" in nwc.content.to_markdown())

# Same via index convenience method
nwc2 = index.read_node_with_content(reader, "welcome")
check("index.read_node_with_content returns same kind", nwc2.content.kind == "local")


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 9. LakehouseReader — lakehouse / S3-stub mode ═══")
# ─────────────────────────────────────────────────────────────────────────────
with tempfile.TemporaryDirectory() as tmp:
    graph = pathlib.Path(tmp) / "graph"
    graph.mkdir()
    (graph / "asset.md").write_text(
        '---\nid: "asset"\ntype: "external_asset"\nsource: "s3://corp-data/lake/churn.parquet"\n---\n# Asset\n',
        encoding="utf-8",
    )
    cfg_path = pathlib.Path(tmp) / "maggraph.toml"
    cfg_path.write_text(
        '[storage]\nmode = "lakehouse"\nroot_path = "./graph"\n\n'
        '[lakehouse]\nremote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]\n',
        encoding="utf-8",
    )
    lh_config = maggraph.load_config(str(cfg_path))
    lh_index  = lh_config.open_index()
    lh_reader = lh_config.open_lakehouse_reader()

    lh_result = lh_reader.read_node(lh_index, "asset")
    print(f"  external content : {lh_result.content!r}")
    check("kind == external_asset", lh_result.content.kind == "external_asset")
    check("uri contains s3://", "s3://" in (lh_result.content.uri or ""))
    check("format == parquet", lh_result.content.format == "parquet")
    check("body is None for external", lh_result.content.body is None)

    # Second read hits cache
    lh_reader.read_node(lh_index, "asset")
    check("cache_len == 1 after two reads", lh_reader.cache_len() == 1)
    check("cache_bytes > 0", lh_reader.cache_bytes() > 0)
    print(f"  cache_len={lh_reader.cache_len()}  cache_bytes={lh_reader.cache_bytes()}")

    lh_md = lh_result.content.to_markdown()
    check("external asset markdown has URI", "s3://" in lh_md)
    print(f"  to_markdown preview:\n    {lh_md[:120]!r}")


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 10. Async methods ═══")
# ─────────────────────────────────────────────────────────────────────────────
async def run_async_checks() -> None:
    idx = maggraph.open_index(str(BASIC_GRAPH))

    # read_node_async
    node = await idx.read_node_async("welcome")
    check("async read_node returns correct id", node.id == "welcome")

    # traverse_async bfs
    result = await idx.traverse_async("welcome", depth=2, order="bfs")
    check("async traverse order=bfs", result.order == "bfs")
    check("async traverse reaches getting_started",
          any(n.id == "getting_started" for n in result.nodes))

    # traverse_async dfs
    dfs = await idx.traverse_async("welcome", depth=2, order="dfs")
    check("async traverse order=dfs", dfs.order == "dfs")

    # LakehouseReader async
    cfg  = maggraph.load_config(str(BASIC_CONFIG))
    aidx = cfg.open_index()
    rdr  = cfg.open_lakehouse_reader()
    anwc = await rdr.read_node_async(aidx, "welcome")
    check("async reader returns local kind", anwc.content.kind == "local")

asyncio.run(run_async_checks())


# ─────────────────────────────────────────────────────────────────────────────
print("\n═══ 11. TraversalNode properties ═══")
# ─────────────────────────────────────────────────────────────────────────────
result = index.traverse("welcome", depth=2, order="bfs")
for tnode in result.nodes:
    check(f"  node {tnode.id!r} has id", isinstance(tnode.id, str))
    check(f"  node {tnode.id!r} has depth", isinstance(tnode.depth, int))
    check(f"  node {tnode.id!r} has path list", isinstance(tnode.path, list))

# depth 0 = start node
start = result.nodes[0]
check("start node is at depth 0", start.depth == 0)
check("start path contains only itself", start.path == ["welcome"])


# ─────────────────────────────────────────────────────────────────────────────
print("\n")
print("═" * 60)
print("  🎉  ALL CHECKS PASSED — maggraph is working correctly")
print("═" * 60)
