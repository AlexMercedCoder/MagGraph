use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn maggraph_cmd() -> Command {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = manifest_dir.join("../examples/basic/maggraph.toml");
    let mut cmd = Command::cargo_bin("maggraph").expect("maggraph binary");
    cmd.arg("--config").arg(config);
    cmd
}

#[test]
fn query_traversal_matches_golden_markdown() {
    let golden = include_str!("fixtures/query_welcome_depth1.md");

    maggraph_cmd()
        .args([
            "query", "--from", "welcome", "--depth", "1", "--order", "bfs",
        ])
        .assert()
        .success()
        .stdout(predicate::eq(golden));
}

#[test]
fn query_unknown_node_exits_nonzero() {
    maggraph_cmd()
        .args(["query", "--from", "does_not_exist", "--depth", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NodeNotFound").or(predicate::str::contains("not found")));
}

#[test]
fn help_lists_query_and_scaffold() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("scaffold"));
}
