# MagGraph — Security (v0.1)

Threat model and mitigations for the local-first graph engine. This is an MVP review for v0.1; re-evaluate before enabling network fetches or multi-user deployments.

## Path traversal (node CRUD)

**Risk:** A malicious or buggy agent supplies `relative_path` values such as `../../etc/passwd` when creating nodes.

**Mitigations:**

- `validate_relative_node_path()` rejects absolute paths, `..` components, and paths under `.maggraph/`.
- Applied on node create and before every `Node::write_to()`.
- Graph index scans skip `.maggraph/` metadata.

**Tests:** `maggraph::security` unit tests; `GraphIndex::create_node_rejects_path_traversal`.

## Lakehouse `file://` reads

**Risk:** A node `source` pointing at `file:///etc/passwd` could exfiltrate host files.

**Mitigations:**

- `FileResolver` requires a non-empty allowlist derived from `[lakehouse].remote_sources` `file://` prefixes.
- Resolved paths are canonicalized and must stay under an allowed root.
- Empty allowlist denies all `file://` reads.

## Lakehouse HTTP(S) / SSRF

**Risk:** When HTTP fetching is enabled, `source` URIs could target internal services (`127.0.0.1`, RFC1918, link-local).

**Mitigations (v0.1):**

- HTTP/HTTPS resolvers are **stubs** — no outbound network I/O.
- `validate_http_uri_host()` blocks loopback, private, link-local, and `.local` hosts at URI resolution time (defense in depth for future fetch implementation).
- Only `file`, `s3`, `http`, and `https` schemes are allowed (`ALLOWED_SCHEMES`).

## Embedded UI

**Risk:** Exposing the dashboard on a public interface.

**Mitigations:**

- `maggraph ui` binds to loopback only (`127.0.0.1` / `::1`); public addresses are rejected.
- No authentication in v0.1 — intended for single-user local audit only.

## Git sync

**Risk:** Followers mutating shared graph state.

**Mitigations:**

- `[sync].role = follower` enforces read-only CRUD via `WritePolicy`.
- Leaders require an active `.maggraph/lock.toml` for writes.

## Test references

| Mitigation | Automated test |
|------------|------------------|
| Path traversal on node paths | `maggraph::security` unit tests; `GraphIndex::create_node_rejects_path_traversal` |
| file:// allowlist | Lakehouse `FileResolver` unit tests |
| HTTP host blocklist | `validate_http_uri_host` unit tests |
| UI loopback bind | UI startup validation tests |
| Follower read-only | `SyncEngine` / `WritePolicy` unit tests |

## Backlog (security-related)

| ID | Item |
|----|------|
| `T-H1` | Path traversal rejection via UI REST API (today: index-level only) |
| `T-F1` | SSRF integration tests when HTTP(S) fetch is enabled |
| MCP / UI | No auth in v0.1 — documented in [`MCP.md`](./MCP.md), [`UI.md`](./UI.md) |

See [`BACKLOG.md`](./BACKLOG.md) and [`TESTING.md`](./TESTING.md).

## Reporting

Open security issues privately with the repository maintainers before public disclosure.
