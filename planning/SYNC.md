# Git sync & leader/follower roles

MagGraph replicates graph state through **Git**. The `[sync]` section in `maggraph.toml` configures role and remote URL.

## Topology

| Role | Writes | Reads | Git push | Git pull |
|------|--------|-------|----------|----------|
| **leader** | Yes (with lock) | Yes | Yes | Yes |
| **follower** | No | Yes | No | Yes |

```toml
[sync]
role = "leader"   # or "follower"
remote_url = "/path/to/remote.git"
```

`remote_url` may be a local bare repository path (`/tmp/graph.git`), `file://` URL, or a hosted Git remote.

## Write lock (`lock.toml`)

The **leader** serializes concurrent writes with a local lock file:

```
{root_path}/.maggraph/lock.toml
```

Example:

```toml
holder = "leader-host"
acquired_at = "2026-06-04T12:00:00Z"
token = "550e8400-e29b-41d4-a716-446655440000"
```

- Acquired automatically when using `SyncEngine::with_leader_write` or leader CRUD with `WritePolicy::leader_with_lock`.
- **Not synced** — `.maggraph/` is listed in the graph root `.gitignore`.
- Followers reject writes based on `[sync].role`, not the lock file.

## CLI

Initialize graph root and Git (when `[sync]` is present):

```bash
maggraph init --git --config maggraph.toml
maggraph sync init --config maggraph.toml
```

Status, pull, push:

```bash
maggraph sync status --config maggraph.toml
maggraph sync pull --config maggraph.toml
maggraph sync push --message "Add nodes" --config maggraph.toml
```

## Rust API

```rust
use maggraph::{GraphIndex, MagGraphConfig, NewNode, SyncEngine, WritePolicy};

let resolved = MagGraphConfig::load("maggraph.toml")?;
let mut sync = SyncEngine::init(&resolved)?;

sync.with_leader_write("my-leader", |policy| {
    let mut index = GraphIndex::open(sync.root_path())?;
    index.create_node_with_policy(new_node, policy)
})?;

sync.commit_and_push("Add node")?;
```

Followers clone and pull:

```rust
let mut sync = SyncEngine::clone_follower(&resolved)?;
sync.pull()?;
let index = GraphIndex::open(sync.root_path())?; // read-only
```

## Conflict resolution

MagGraph uses **Git-native** merges:

1. **Fast-forward** when the follower/leader has no divergent commits.
2. **Three-way merge** when both sides changed; libgit2 performs the merge and reports conflict paths.
3. Conflicts must be resolved with standard Git tooling (`git status`, edit files, `git add`, commit).

There is no custom merge driver for Markdown frontmatter in v0.1 — treat node files like any other text and resolve manually or with your Git workflow.

## Acceptance workflow (local bare remote)

```bash
# 1. Bare remote
git init --bare /tmp/maggraph-sync.git

# 2. Leader: write + push
maggraph init --git --config leader/maggraph.toml
maggraph sync push --message "seed" --config leader/maggraph.toml

# 3. Follower: clone + pull (read-only)
maggraph sync pull --config follower/maggraph.toml
# follower CRUD / push returns read-only error
```

Integration tests in `maggraph::sync::engine` cover leader push, follower pull, and follower write rejection.
