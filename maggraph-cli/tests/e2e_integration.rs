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

    // Seed leader with the basic example graph so query has nodes after pull.
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
