"""MagGraph — Python bindings for the Rust graph engine."""

from maggraph._maggraph import (
    GraphIndex,
    LakehouseReader,
    MagGraphError,
    Node,
    NodeWithContent,
    ResolvedConfig,
    ResolvedContent,
    TraversalNode,
    TraversalResult,
    load_config,
    open_index,
)

async def _graph_read_node_async(self: GraphIndex, node_id: str) -> Node:
    return self.read_node(node_id)


async def _graph_traverse_async(
    self: GraphIndex, from_id: str, depth: int = 2, order: str = "bfs"
) -> TraversalResult:
    return self.traverse(from_id, depth, order)


async def _lakehouse_read_node_async(
    self: LakehouseReader, index: GraphIndex, node_id: str
) -> NodeWithContent:
    return self.read_node(index, node_id)


# Keep async convenience methods on the Python side so they follow Python 3.14
# asyncio semantics while reusing the Rust sync methods as the source of truth.
GraphIndex.read_node_async = _graph_read_node_async
GraphIndex.traverse_async = _graph_traverse_async
LakehouseReader.read_node_async = _lakehouse_read_node_async

__all__ = [
    "GraphIndex",
    "LakehouseReader",
    "MagGraphError",
    "Node",
    "NodeWithContent",
    "ResolvedConfig",
    "ResolvedContent",
    "TraversalNode",
    "TraversalResult",
    "load_config",
    "open_index",
]

__version__ = "0.2.1"
