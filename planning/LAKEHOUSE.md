# Lakehouse mode

Nodes in **lakehouse** storage mode are semantic pointers: markdown frontmatter holds metadata and an optional `source` / `source_uri`, while bulk data lives externally.

## Configuration

```toml
[storage]
mode = "lakehouse"
root_path = "./knowledge_graph"

[lakehouse]
remote_sources = [{ uri = "s3://corp-data/lake", format = "parquet" }]

[lakehouse.cache]
ttl_secs = 300      # 0 = no TTL expiry
max_bytes = 10485760
```

## URI resolution

| Node `source` | Result |
|---------------|--------|
| Absolute URI (`file://`, `s3://`, `http://`, `https://`) | Used as-is (scheme allowlisted) |
| Relative path (`churn_data.parquet`) | Joined to first `[lakehouse].remote_sources` URI |

## Reading content

```rust
use maggraph::{GraphIndex, LakehouseReader, MagGraphConfig};

let resolved = MagGraphConfig::load("maggraph.toml")?;
let index = GraphIndex::open(&resolved.root_path)?;
let mut reader = LakehouseReader::from_config(&resolved);
let node = reader.read_node(&index, "customer_churn_q2")?;
println!("{}", node.content.to_markdown());
```

- **Local mode:** returns the markdown body (`ResolvedContent::LocalMarkdown`).
- **Lakehouse mode:** resolves `source` via `ContentResolver` implementations; results are cached per URI.

## Resolvers (MVP)

| Scheme | Behavior |
|--------|----------|
| `file://` | Reads text or Parquet metadata (PAR1 magic, size) from disk |
| `s3://` | Stub: metadata + snippet (no AWS SDK yet) |
| `http(s)://` | Stub: metadata only (no network I/O) |

Register custom resolvers with `ResolverRegistry` for tests or future S3 integration.
