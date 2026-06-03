# Wikilink syntax (MagGraph)

Edges are **directed** and **outgoing** from the source node to resolved target node ids.

## Sources of edges

1. **Frontmatter** — `links: ["node_id", ...]` in YAML
2. **Body** — `[[wikilink]]` patterns in markdown (outside fenced code blocks)

Both are merged and deduplicated per source node.

## Supported wikilink forms

| Syntax | Resolved target |
|--------|-----------------|
| `[[getting_started]]` | `getting_started` |
| `[[welcome\|Display text]]` | `welcome` (Obsidian: target before `\|`) |
| `[[welcome#section]]` | `welcome` (heading fragment ignored) |

Fenced code blocks (` ``` `) are stripped before parsing so literals like `` `[[not_a_link]]` `` inside fences do not create edges.

## Target resolution

1. Exact match on node `id`
2. Else match on markdown file stem (e.g. `welcome` → `welcome.md`’s node id)

Unresolved targets are recorded separately (`GraphAdjacency::unresolved_targets`) and do not create edges.

## Traversal

- `GraphIndex::adjacency()` builds the edge map
- `traverse(adjacency, index, from, max_depth, order)` — BFS or DFS, includes start node at depth 0
- `TraversalResult::to_markdown(index)` — LLM-oriented report

See `maggraph::wikilink` and `maggraph::graph` for implementation details.
