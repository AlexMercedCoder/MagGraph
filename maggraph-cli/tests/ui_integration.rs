use std::process::Command;
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;

#[test]
fn ui_dry_run_prints_loopback_url() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config = format!("{manifest_dir}/../examples/basic/maggraph.toml");

    let output = Command::new(cargo_bin("maggraph"))
        .args(["ui", "--dry-run", "--port", "9876", "--config", &config])
        .output()
        .expect("run ui dry-run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("http://127.0.0.1:9876"));
}

#[test]
fn ui_rejects_public_bind() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config = format!("{manifest_dir}/../examples/basic/maggraph.toml");

    let output = Command::new(cargo_bin("maggraph"))
        .args(["ui", "--host", "0.0.0.0", "--dry-run", "--config", &config])
        .output()
        .expect("run ui");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("loopback"));
}

#[test]
fn ui_serves_nodes_over_http() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config = format!("{manifest_dir}/../examples/basic/maggraph.toml");

    let mut child = Command::new(cargo_bin("maggraph"))
        .args(["ui", "--port", "18787", "--config", &config])
        .spawn()
        .expect("spawn ui server");

    std::thread::sleep(Duration::from_millis(500));

    let response = ureq::get("http://127.0.0.1:18787/api/nodes")
        .call()
        .expect("GET /api/nodes");

    assert_eq!(response.status(), 200);
    let nodes: Vec<serde_json::Value> = response.into_json().expect("json");
    assert!(nodes.iter().any(|n| n["id"] == "welcome"));

    let detail = ureq::get("http://127.0.0.1:18787/api/nodes/welcome")
        .call()
        .expect("GET node");
    let body: serde_json::Value = detail.into_json().expect("json");
    assert!(body["body"].as_str().unwrap_or("").contains("Welcome"));

    child.kill().ok();
    child.wait().ok();
}
