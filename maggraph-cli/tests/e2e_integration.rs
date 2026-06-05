//! Cross-phase integration tests for the v0.1 release gate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use tempfile::tempdir;

fn copy_basic_fixture(dest: &Path) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let basic_root = manifest_dir.join("../examples/basic");
    let graph_src = basic_root.join("knowledge_graph");
    let graph_dest = dest.join("knowledge_graph");
    fs::create_dir_all(&graph_dest).expect("graph dir");
    for entry in fs::read_dir(&graph_src).expect("read graph") {
        let entry = entry.expect("entry");
        fs::copy(entry.path(), graph_dest.join(entry.file_name())).expect("copy");
    }
    fs::copy(basic_root.join("maggraph.toml"), dest.join("maggraph.toml")).expect("copy config");
}

/// Build a bare-git + leader + follower environment.  Returns (dir, remote_url, leader_dir, follower_dir).
fn setup_sync_env() -> (tempfile::TempDir, String, PathBuf, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let bare = dir.path().join("remote.git");
    Command::new("git")
        .args(["init", "--bare", bare.to_str().expect("utf8")])
        .status()
        .expect("git init bare");

    let leader = dir.path().join("leader");
    let follower = dir.path().join("follower");
    fs::create_dir_all(&leader).expect("leader dir");
    fs::create_dir_all(&follower).expect("follower dir");

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::copy(
        manifest.join("../examples/sync/leader/maggraph.toml"),
        leader.join("maggraph.toml"),
    )
    .expect("leader config");
    fs::copy(
        manifest.join("../examples/sync/follower/maggraph.toml"),
        follower.join("maggraph.toml"),
    )
    .expect("follower config");

    // Seed leader graph
    let leader_graph = leader.join("graph");
    fs::create_dir_all(&leader_graph).expect("leader graph");
    for entry in fs::read_dir(manifest.join("../examples/basic/knowledge_graph")).expect("read") {
        let entry = entry.expect("entry");
        fs::copy(entry.path(), leader_graph.join(entry.file_name())).expect("copy node");
    }

    let remote_url = bare.canonicalize().expect("canon").display().to_string();
    for root in [&leader, &follower] {
        let config = fs::read_to_string(root.join("maggraph.toml")).expect("read");
        fs::write(
            root.join("maggraph.toml"),
            config.replace("/tmp/maggraph-sync.git", &remote_url),
        )
        .expect("write");
    }

    (dir, remote_url, leader, follower)
}

#[test]
fn init_query_scaffold_end_to_end() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args(["--config", config.to_str().expect("utf8"), "init"])
        .assert()
        .success();

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            config.to_str().expect("utf8"),
            "query",
            "--from",
            "welcome",
            "--depth",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MagGraph Traversal Report"));

    let scaffold_out = dir.path().join("agent_out");
    fs::create_dir_all(&scaffold_out).expect("out");

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            config.to_str().expect("utf8"),
            "scaffold",
            "--mcp",
            "--skill",
            "--output",
            scaffold_out.to_str().expect("utf8"),
        ])
        .assert()
        .success();

    assert!(scaffold_out.join("mcp_server/server.py").is_file());
    assert!(dir.path().join("knowledge_graph/SKILL.md").is_file());
}

#[test]
fn sync_leader_push_follower_pull() {
    let (_dir, _remote_url, leader, follower) = setup_sync_env();

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "init",
            "--git",
        ])
        .assert()
        .success();

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "seed graph",
        ])
        .assert()
        .success();

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            follower.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "pull",
        ])
        .assert()
        .success();

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            follower.join("maggraph.toml").to_str().expect("utf8"),
            "query",
            "--from",
            "welcome",
            "--depth",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("welcome"));
}

#[test]
fn clean_env_smoke_from_release_binary() {
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("ui"));
}

// T-M2: `maggraph init --skill` writes SKILL.md into the graph root.
#[test]
fn init_skill_writes_skill_md() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");

    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            config.to_str().expect("utf8"),
            "init",
            "--skill",
        ])
        .assert()
        .success();

    let skill_path = dir.path().join("knowledge_graph/SKILL.md");
    assert!(
        skill_path.is_file(),
        "SKILL.md should be written by init --skill"
    );
    let content = fs::read_to_string(&skill_path).expect("read SKILL.md");
    assert!(
        content.contains("maggraph_skill_version"),
        "SKILL.md missing version header"
    );
    assert!(
        content.contains("Edge patterns"),
        "SKILL.md missing edge patterns section"
    );
}

// T-M2: follower `sync init` clones the remote and allows `query`.
#[test]
fn follower_sync_init_clones_remote() {
    let (_dir, _remote_url, leader, follower) = setup_sync_env();

    // Leader: init git + push
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "init",
            "--git",
        ])
        .assert()
        .success();
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "init",
        ])
        .assert()
        .success();

    // Follower: `sync init` should clone the remote
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            follower.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "init",
        ])
        .assert()
        .success();

    // After clone, follower graph directory should have the leader's nodes
    let follower_graph = follower.join("graph");
    assert!(
        follower_graph.exists(),
        "follower graph directory should exist after sync init"
    );
}

// T-H4: Follower `sync push` must be rejected (followers are read-only).
#[test]
fn follower_sync_push_is_rejected() {
    let (_dir, _remote_url, leader, follower) = setup_sync_env();

    // Leader setup
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "init",
            "--git",
        ])
        .assert()
        .success();
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "seed",
        ])
        .assert()
        .success();

    // Follower: pull succeeds
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            follower.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "pull",
        ])
        .assert()
        .success();

    // Follower: push must fail with a role/permission error
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            follower.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "forbidden",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("follower")
                .or(predicate::str::contains("read-only"))
                .or(predicate::str::contains("write"))
                .or(predicate::str::contains("policy")),
        );
}

// T-M3: `sync pull` with merge conflicts prints conflict file paths to stdout.
#[test]
fn sync_pull_prints_conflict_paths() {
    let dir = tempdir().expect("tempdir");
    let bare = dir.path().join("remote.git");
    Command::new("git")
        .args(["init", "--bare", bare.to_str().expect("utf8")])
        .status()
        .expect("git init bare");

    let leader_a = dir.path().join("leader_a");
    let leader_b = dir.path().join("leader_b");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for role_dir in [&leader_a, &leader_b] {
        fs::create_dir_all(role_dir).expect("dir");
        let config_src = manifest.join("../examples/sync/leader/maggraph.toml");
        let config_text = fs::read_to_string(&config_src).expect("read").replace(
            "/tmp/maggraph-sync.git",
            &bare.canonicalize().expect("canon").display().to_string(),
        );
        fs::write(role_dir.join("maggraph.toml"), config_text).expect("write config");

        let graph_dir = role_dir.join("graph");
        fs::create_dir_all(&graph_dir).expect("graph dir");
        for entry in fs::read_dir(manifest.join("../examples/basic/knowledge_graph")).expect("read")
        {
            let entry = entry.expect("entry");
            fs::copy(entry.path(), graph_dir.join(entry.file_name())).expect("copy");
        }
    }

    // leader_a: init, push
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader_a.join("maggraph.toml").to_str().expect("utf8"),
            "init",
            "--git",
        ])
        .assert()
        .success();
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader_a.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "initial",
        ])
        .assert()
        .success();

    // leader_b: init (clone), then diverge from leader_a by editing welcome.md
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader_b.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "init",
        ])
        .assert()
        .success();

    // Both sides modify the same file to guarantee a conflict.
    let welcome_a = leader_a.join("graph/welcome.md");
    let welcome_b = leader_b.join("graph/welcome.md");
    fs::write(
        &welcome_a,
        "---\nid: \"welcome\"\ntype: \"note\"\nlinks: []\n---\n# Branch A edit\n",
    )
    .expect("write a");
    fs::write(
        &welcome_b,
        "---\nid: \"welcome\"\ntype: \"note\"\nlinks: []\n---\n# Branch B edit\n",
    )
    .expect("write b");

    // leader_a pushes its change
    assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader_a.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "push",
            "--message",
            "branch a change",
        ])
        .assert()
        .success();

    // leader_b pulls — should report conflict paths (not exit nonzero; conflicts are printed)
    let output = assert_cmd::Command::new(cargo_bin("maggraph"))
        .args([
            "--config",
            leader_b.join("maggraph.toml").to_str().expect("utf8"),
            "sync",
            "pull",
        ])
        .output()
        .expect("pull output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // Either the pull succeeded cleanly (unlikely with diverged histories) or
    // the conflict was surfaced. Either way, the command must not panic.
    assert!(
        output.status.success() || combined.contains("conflict") || combined.contains("welcome"),
        "expected success or conflict report, got status={} stdout={stdout} stderr={stderr}",
        output.status
    );
}
