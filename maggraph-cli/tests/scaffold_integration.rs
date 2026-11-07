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

    let skill = std::fs::read_to_string(output.join("maggraph-skill/SKILL.md")).expect("skill");
    assert!(skill.contains("maggraph_skill_version"));
    assert!(skill.contains("Edge patterns"));
    assert!(
        !dir.path().join("knowledge_graph/SKILL.md").exists(),
        "SKILL.md must not be written into the graph root"
    );
}

fn maggraph(config: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("maggraph").expect("maggraph binary");
    cmd.arg("--config").arg(config);
    cmd
}

// G1: scaffolding the skill must leave the graph openable, so the generated server's
// `open_index()` / `list_nodes()` and every CLI command that opens the index keep working.
#[test]
fn scaffold_skill_keeps_index_openable() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");
    let output = dir.path().join("agent");

    maggraph(&config)
        .args(["scaffold", "--mcp", "--skill", "--output"])
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("maggraph-skill/SKILL.md"));

    // Opening the index (search lists all nodes) succeeds after scaffolding.
    maggraph(&config)
        .args(["search", "--limit", "50"])
        .assert()
        .success()
        .stdout(predicate::str::contains("welcome"));

    // Re-running the scaffold (which opens the index again) succeeds too.
    maggraph(&config)
        .args(["scaffold", "--mcp", "--skill", "--output"])
        .arg(&output)
        .assert()
        .success();
}

// G1: a SKILL.md left in the graph root by MagGraph 0.4.1 is skipped and reported,
// not parsed as a node that fails the whole index.
#[test]
fn legacy_skill_md_in_graph_root_is_skipped() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");
    fs::write(
        dir.path().join("knowledge_graph/SKILL.md"),
        "---\nmaggraph_skill_version: \"0.4.1\"\nnode_count: 2\n---\n\n# MagGraph Agent Skill\n",
    )
    .expect("write legacy skill");

    maggraph(&config)
        .args(["query", "--from", "welcome", "--depth", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MagGraph Traversal Report"));

    maggraph(&config)
        .args(["scaffold", "--skill", "--output"])
        .arg(dir.path().join("agent"))
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped non-node file SKILL.md"));
}

// G2: the generated requirements point at the published `maggraph` bindings.
#[test]
fn scaffold_requirements_reference_published_bindings() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");
    let output = dir.path().join("agent");

    maggraph(&config)
        .args(["scaffold", "--mcp", "--output"])
        .arg(&output)
        .assert()
        .success();

    let requirements =
        fs::read_to_string(output.join("mcp_server/requirements.txt")).expect("requirements");
    let lines: Vec<&str> = requirements
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .collect();
    assert!(lines.contains(&"fastmcp>=2.0"), "{requirements}");
    assert!(
        lines.iter().any(|line| line.starts_with("maggraph>=")),
        "requirements must install the maggraph bindings: {requirements}"
    );
    assert!(!requirements.contains("install from repo"));
}

// G4: `--read-only` generates a server without create/update/delete tools.
#[test]
fn scaffold_read_only_omits_write_tools() {
    let dir = tempdir().expect("tempdir");
    copy_basic_fixture(dir.path());
    let config = dir.path().join("maggraph.toml");
    let output = dir.path().join("agent");

    maggraph(&config)
        .args(["scaffold", "--mcp", "--skill", "--read-only", "--output"])
        .arg(&output)
        .assert()
        .success();

    let server = fs::read_to_string(output.join("mcp_server/server.py")).expect("server");
    assert!(server.contains("def list_nodes"));
    assert!(server.contains("def get_node"));
    assert!(server.contains("def traverse_graph"));
    assert!(server.contains("READ_ONLY = True"));
    for tool in ["create_node", "update_node", "delete_node"] {
        assert!(
            !server.contains(tool),
            "read-only server must not define {tool}"
        );
    }

    let readme = fs::read_to_string(output.join("mcp_server/README.md")).expect("readme");
    let skill = fs::read_to_string(output.join("maggraph-skill/SKILL.md")).expect("skill");
    for doc in [&readme, &skill] {
        assert!(!doc.contains("`create_node`"));
        assert!(doc.contains("--read-only"));
    }
}
