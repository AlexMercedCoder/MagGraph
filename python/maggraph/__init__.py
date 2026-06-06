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

__version__ = "0.2.0"
