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
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("recall"))
        .stdout(predicate::str::contains("scaffold"));
}

#[test]
fn search_finds_node_body_and_json_output() {
    maggraph_cmd()
        .args(["search", "Welcome", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"welcome\""));
}

#[test]
fn recall_prints_agent_bundle() {
    maggraph_cmd()
        .args(["recall", "welcome", "--reason", "test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reason"))
        .stdout(predicate::str::contains("Backlinks"));
}

// T-M1: DFS golden snapshot
#[test]
fn query_dfs_traversal_matches_golden_markdown() {
    let golden = include_str!("fixtures/query_welcome_depth1_dfs.md");

    maggraph_cmd()
        .args([
            "query", "--from", "welcome", "--depth", "1", "--order", "dfs",
        ])
        .assert()
        .success()
        .stdout(predicate::eq(golden));
}

// T-H3: shell completion smoke — assert non-empty output for every supported shell.
// The `complete` subcommand does not require a valid config on disk; it only reads
// the CLI structure at compile time, so we pass the default path and rely on the
// fact that a missing config is handled after arg parsing.
#[test]
fn complete_bash_emits_non_empty_script() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .args(["complete", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?s).{20,}").expect("regex"));
}

#[test]
fn complete_zsh_emits_non_empty_script() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .args(["complete", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?s).{20,}").expect("regex"));
}

#[test]
fn complete_fish_emits_non_empty_script() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .args(["complete", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?s).{20,}").expect("regex"));
}

#[test]
fn complete_elvish_emits_non_empty_script() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .args(["complete", "elvish"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?s).{20,}").expect("regex"));
}

#[test]
fn complete_powershell_emits_non_empty_script() {
    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .args(["complete", "power-shell"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?s).{20,}").expect("regex"));
}
