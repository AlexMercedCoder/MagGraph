"""Behavioral compatibility contract for MagAgent's MagGraph integration."""

from __future__ import annotations

from pathlib import Path

import maggraph
import pytest


MEMORY_KINDS = (
    "preference",
    "project_fact",
    "decision",
    "task",
    "session_summary",
    "bookmark",
    "tool_failure",
)


def _index(tmp_path: Path) -> tuple[maggraph.GraphIndex, Path]:
    root = tmp_path / "graph"
    root.mkdir()
    (root / "anchor.md").write_text(
        '---\nid: "anchor"\ntype: "project_fact"\ntags: ["contract"]\n'
        'links: ["related"]\n---\nAnchor body for MagAgent recall.\n',
        encoding="utf-8",
    )
    (root / "related.md").write_text(
        '---\nid: "related"\ntype: "decision"\nlinks: ["anchor"]\n'
        "---\nRelated decision.\n",
        encoding="utf-8",
    )
    return maggraph.open_index(str(root)), root


def test_magagent_retrieval_contract(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)

    results = index.search(
        "anchor",
        node_type="project_fact",
        tags=["contract"],
        include_suppressed=False,
        limit=10,
        modified_since_unix=None,
    )
    assert results
    assert set(results[0]) == {
        "id",
        "type",
        "relative_path",
        "score",
        "matched",
        "summary",
        "modified_unix",
    }
    assert isinstance(results[0]["matched"], list)

    bundle = index.recall_bundle("anchor", reason="contract test", body_chars=24)
    assert set(bundle) == {
        "id",
        "type",
        "summary",
        "body_excerpt",
        "links",
        "backlinks",
        "relevance_reason",
        "metadata",
        "markdown",
    }
    assert bundle["relevance_reason"] == "contract test"
    assert "related" in bundle["backlinks"]
    assert len(bundle["body_excerpt"]) <= 24


def test_magagent_hybrid_retrieval_contract(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)

    results = index.hybrid_search(
        "anchor",
        node_types=["project_fact", "decision"],
        seed_ids=["anchor"],
        semantic_scores={"related": 0.9},
        limit=10,
    )

    assert results
    assert set(results[0]) == {
        "id",
        "type",
        "relative_path",
        "score",
        "signals",
        "reasons",
        "summary",
        "modified_unix",
        "canonical_id",
    }
    related = next(item for item in results if item["id"] == "related")
    assert related["signals"]["semantic"] == pytest.approx(0.9)
    assert "semantic match" in related["reasons"]
    assert "graph relationship" in related["reasons"]


def test_magagent_incremental_change_contract(tmp_path: Path) -> None:
    index, root = _index(tmp_path)
    created = root / "fresh.md"
    created.write_text(
        '---\nid: "fresh"\ntype: "task"\n---\nFresh task.\n',
        encoding="utf-8",
    )

    assert index.update_file("fresh.md") == "fresh"
    changes = index.changed_since(0)
    assert changes
    assert set(changes[0]) == {"id", "relative_path", "modified_unix"}
    assert index.update_file(str(created)) == "fresh"

    created.unlink()
    assert index.update_file("fresh.md") is None
    assert "fresh" not in index.list_nodes()


@pytest.mark.parametrize("kind", MEMORY_KINDS)
def test_magagent_memory_schema_contract(tmp_path: Path, kind: str) -> None:
    index, _ = _index(tmp_path)
    node_id = f"memory_{kind}"
    created = index.create_memory_node(node_id, kind, f"Body for {kind}.", links=["anchor"])

    assert created.id == node_id
    assert created.node_type == kind
    assert created.links == ["anchor"]
    assert index.read_node(node_id).body == f"Body for {kind}.\n"


def test_magagent_memory_lifecycle_contract(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)
    index.create_memory_node("duplicate", "project_fact", "Duplicate body.")

    index.suppress_node("duplicate", reason="superseded")
    assert index.search("duplicate") == []
    assert index.search("duplicate", include_suppressed=True)[0]["id"] == "duplicate"

    index.unsuppress_node("duplicate")
    assert index.search("duplicate")[0]["id"] == "duplicate"
    index.merge_nodes("anchor", "duplicate")
    assert "duplicate" not in index.list_nodes()
    merged = index.read_node("anchor").to_dict()
    assert merged["merged_from"] == ["duplicate"]


def test_magagent_memory_provenance_and_temporal_contract(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)
    created = index.create_memory_node(
        "scoped_decision",
        "decision",
        "Use the durable runtime.",
        project="demo",
        source_task="task_123",
        source_session="session_456",
        source_tool="memory_promote",
        extraction_method="reviewed_inbox",
        confidence=1.5,
        valid_from="2025-01-01T00:00:00Z",
        canonical_id="runtime-decision",
    )

    metadata = created.to_dict()
    assert metadata["project"] == "demo"
    assert metadata["source_task"] == "task_123"
    assert metadata["source_session"] == "session_456"
    assert metadata["source_tool"] == "memory_promote"
    assert metadata["extraction_method"] == "reviewed_inbox"
    assert metadata["confidence"] == pytest.approx(1.0)
    assert metadata["canonical_id"] == "runtime-decision"


def test_magagent_reviewed_memory_batch_contract(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)
    operations = [
        {"op": "update", "id": "anchor", "body": "Reviewed anchor."},
        {"op": "suppress", "id": "related", "reason": "stale"},
    ]

    preview = index.apply_memory_batch(operations, preview=True)
    applied = index.apply_memory_batch(operations)

    assert preview == {"ok": True, "preview": True, "operations": 2}
    assert applied["applied"] == ["update:anchor", "suppress:related"]
    assert index.read_node("anchor").body == "Reviewed anchor.\n"
    assert all(item["id"] != "related" for item in index.search("related"))


def test_magagent_contract_errors_remain_typed(tmp_path: Path) -> None:
    index, _ = _index(tmp_path)
    with pytest.raises(maggraph.MagGraphError, match="not found"):
        index.recall_bundle("missing")
    with pytest.raises(maggraph.MagGraphError, match="cannot merge"):
        index.merge_nodes("anchor", "anchor")
