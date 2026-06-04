use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn copy_basic_fixture(dest: &std::path::Path) {
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
fn scaffold_mcp_and_skill_writes_expected_files() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");
    let output = dir.path().join("out");
    fs::create_dir_all(&output).expect("output dir");

    Command::cargo_bin("maggraph")
        .expect("maggraph binary")
        .arg("--config")
        .arg(&config)
        .args([
            "scaffold",
            "--mcp",
            "--skill",
            "--output",
            output.to_str().expect("utf8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MCP scaffold"))
        .stdout(predicate::str::contains("SKILL.md"));

    let server = std::fs::read_to_string(output.join("mcp_server/server.py")).expect("read");
    assert!(server.contains("import maggraph"));
    assert!(server.contains("def traverse_graph"));
    assert!(!server.contains("stub"));

    let skill =
        std::fs::read_to_string(dir.path().join("knowledge_graph/SKILL.md")).expect("skill");
    assert!(skill.contains("maggraph_skill_version"));
    assert!(skill.contains("Edge patterns"));
}
