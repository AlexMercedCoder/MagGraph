# Sync example (Phase 5)

This example shows leader/follower Git sync with a local bare remote.

```bash
# Create bare remote
git init --bare /tmp/maggraph-sync.git

# Leader (from repo root)
cp -r examples/sync/leader /tmp/maggraph-leader
cd /tmp/maggraph-leader
maggraph init --git --config maggraph.toml
maggraph sync push --message "seed graph" --config maggraph.toml

# Follower
cp -r examples/sync/follower /tmp/maggraph-follower
cd /tmp/maggraph-follower
# Edit maggraph.toml remote_url if needed
maggraph sync init --config maggraph.toml   # clones when root is empty
maggraph sync pull --config maggraph.toml
```

Leader writes require the write lock via `SyncEngine::with_leader_write` in Rust, or leader role in the API.
