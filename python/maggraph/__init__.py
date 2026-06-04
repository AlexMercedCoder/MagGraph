"""MagGraph — Python bindings for the Rust graph engine."""

from maggraph._maggraph import (
    GraphIndex,
    MagGraphError,
    Node,
    ResolvedConfig,
    TraversalNode,
    TraversalResult,
    load_config,
    open_index,
)

__all__ = [
    "GraphIndex",
    "MagGraphError",
    "Node",
    "ResolvedConfig",
    "TraversalNode",
    "TraversalResult",
    "load_config",
    "open_index",
]

__version__ = "0.1.0"
