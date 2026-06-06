from typing import Awaitable, Literal, Optional, TypedDict

class SearchResult(TypedDict):
    id: str
    type: str
    relative_path: str
    score: int
    matched: list[str]
    summary: str
    modified_unix: Optional[int]

class GraphChange(TypedDict):
    id: str
    relative_path: str
    modified_unix: int

class RecallBundle(TypedDict):
    id: str
    type: str
    summary: str
    body_excerpt: str
    links: list[str]
    backlinks: list[str]
    metadata: dict[str, object]
    relevance_reason: str
    markdown: str

class MagGraphError(Exception): ...

class ResolvedConfig:
    @property
    def root_path(self) -> str: ...
    @property
    def config_path(self) -> str: ...
    @property
    def storage_mode(self) -> Literal["local", "lakehouse"]: ...
    def open_index(self) -> GraphIndex: ...
    def open_lakehouse_reader(self) -> LakehouseReader: ...

class GraphIndex:
    @classmethod
    def open(cls, root_path: str) -> GraphIndex: ...
    @property
    def root_path(self) -> str: ...
    def __len__(self) -> int: ...
    def list_nodes(self) -> list[str]: ...
    def read_node(self, node_id: str) -> Node: ...
    def search(
        self,
        query: str = "",
        node_type: str | None = None,
        tags: list[str] | None = None,
        include_suppressed: bool = False,
        limit: int = 50,
        modified_since_unix: int | None = None,
    ) -> list[SearchResult]: ...
    def backlinks(self, node_id: str) -> list[str]: ...
    def changed_since(self, since_unix: int) -> list[GraphChange]: ...
    def update_file(self, path: str) -> Optional[str]: ...
    def read_node_async(self, node_id: str) -> Awaitable[Node]: ...
    def create_node(
        self,
        node_id: str,
        node_type: str = "note",
        body: str = "",
        links: list[str] | None = None,
    ) -> Node: ...
    def create_memory_node(
        self,
        node_id: str,
        kind: Literal[
            "preference",
            "project_fact",
            "decision",
            "task",
            "session_summary",
            "bookmark",
            "tool_failure",
        ],
        body: str = "",
        links: list[str] | None = None,
    ) -> Node: ...
    def update_node(self, node_id: str, body: str) -> None: ...
    def delete_node(self, node_id: str) -> None: ...
    def suppress_node(self, node_id: str, reason: str | None = None) -> None: ...
    def unsuppress_node(self, node_id: str) -> None: ...
    def merge_nodes(self, target_id: str, source_id: str) -> None: ...
    def recall_bundle(
        self, node_id: str, reason: str = "", body_chars: int = 1200
    ) -> RecallBundle: ...
    def traverse(
        self,
        from_id: str,
        depth: int = 2,
        order: Literal["bfs", "dfs"] = "bfs",
    ) -> TraversalResult: ...
    def traverse_async(
        self,
        from_id: str,
        depth: int = 2,
        order: Literal["bfs", "dfs"] = "bfs",
    ) -> Awaitable[TraversalResult]: ...
    def read_node_with_content(
        self, reader: LakehouseReader, node_id: str
    ) -> NodeWithContent: ...

class Node:
    @property
    def id(self) -> str: ...
    @property
    def node_type(self) -> str: ...
    @property
    def source(self) -> Optional[str]: ...
    @property
    def links(self) -> list[str]: ...
    @property
    def body(self) -> str: ...
    @property
    def relative_path(self) -> str: ...
    def to_markdown(self) -> str: ...
    def to_dict(self) -> dict[str, object]: ...

class TraversalNode:
    @property
    def id(self) -> str: ...
    @property
    def depth(self) -> int: ...
    @property
    def path(self) -> list[str]: ...

class TraversalResult:
    @property
    def start(self) -> str: ...
    @property
    def max_depth(self) -> int: ...
    @property
    def order(self) -> str: ...
    @property
    def nodes(self) -> list[TraversalNode]: ...
    def to_markdown(self, index: GraphIndex) -> str: ...

class ResolvedContent:
    """Content resolved from a node source — local markdown or an external asset.

    Check ``.kind`` to determine how to interpret the other attributes:

    * ``"local"``          — ``.body`` is the markdown, ``.uri`` / ``.format`` are ``None``
    * ``"text"``           — ``.uri`` and ``.body`` are set, ``.format`` is ``None``
    * ``"external_asset"`` — ``.uri`` and ``.format`` are set, ``.body`` is ``None``
    """
    @property
    def kind(self) -> Literal["local", "text", "external_asset"]: ...
    @property
    def body(self) -> Optional[str]: ...
    """Markdown body (``"local"`` and ``"text"`` only; ``None`` for external assets)."""
    @property
    def uri(self) -> Optional[str]: ...
    """External URI (``"text"`` and ``"external_asset"`` only)."""
    @property
    def format(self) -> Optional[str]: ...
    """Detected format, e.g. ``"parquet"`` (``"external_asset"`` only)."""
    @property
    def size_bytes(self) -> Optional[int]: ...
    """Size in bytes of the external asset, if known."""
    @property
    def parquet_magic_valid(self) -> Optional[bool]: ...
    """Whether the PAR1 magic header is valid (Parquet assets only)."""
    @property
    def snippet(self) -> Optional[str]: ...
    """Optional text snippet for external assets."""
    def to_markdown(self) -> str: ...
    """Markdown-friendly summary for agent consumption."""

class NodeWithContent:
    """A graph node paired with its resolved external content."""
    @property
    def node(self) -> Node: ...
    @property
    def content(self) -> ResolvedContent: ...

class LakehouseReader:
    """Resolves node content according to the configured storage mode.

    Construct via :meth:`ResolvedConfig.open_lakehouse_reader`.

    Example::

        config = maggraph.load_config("maggraph.toml")
        index  = config.open_index()
        reader = config.open_lakehouse_reader()

        result = reader.read_node(index, "my_asset")
        print(result.content.to_markdown())
    """
    def read_node(self, index: GraphIndex, node_id: str) -> NodeWithContent: ...
    def read_node_async(
        self, index: GraphIndex, node_id: str
    ) -> Awaitable[NodeWithContent]: ...
    def cache_len(self) -> int: ...
    """Number of entries in the in-memory content cache."""
    def cache_bytes(self) -> int: ...
    """Total bytes currently held in the cache."""

def load_config(path: str) -> ResolvedConfig: ...
def open_index(root_path: str) -> GraphIndex: ...
