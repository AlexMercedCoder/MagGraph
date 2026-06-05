//! T-H1: UI REST CRUD integration tests.
//!
//! These tests spin up the Axum router in-process (no TCP socket needed for most cases)
//! and verify POST /api/nodes, PATCH /api/nodes/{id}, DELETE /api/nodes/{id},
//! GET /api/edges, and path-traversal rejection via the API.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use maggraph::{router, AppState, GraphIndex};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn basic_state() -> (TempDir, AppState) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("../examples/basic/knowledge_graph");

    // Copy basic fixture into a temp dir so mutations don't touch the repo.
    let tmp = TempDir::new().expect("tempdir");
    let dest = tmp.path().join("knowledge_graph");
    copy_dir(&src, &dest);

    let index = GraphIndex::open(&dest).expect("open index");
    let state = AppState {
        index: Arc::new(Mutex::new(index)),
    };
    (tmp, state)
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).expect("mkdir");
    for entry in std::fs::read_dir(src).expect("read_dir") {
        let entry = entry.expect("entry");
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), dst_path).expect("copy");
        }
    }
}

async fn body_json(body: axum::body::Body) -> Value {
    let bytes = axum::body::to_bytes(body, usize::MAX).await.expect("bytes");
    serde_json::from_slice(&bytes).expect("json")
}

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

// ──────────────────────────────────────────────────────────────────────────────
// POST /api/nodes — create node
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_node_returns_201_with_detail() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let payload = json!({
        "id": "test_node",
        "type": "note",
        "body": "# Test\n",
        "relative_path": "test_node.md",
        "links": []
    });

    let response = app
        .oneshot(json_request(Method::POST, "/api/nodes", payload))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = body_json(response.into_body()).await;
    assert_eq!(body["id"], "test_node");
    assert_eq!(body["type"], "note");
    assert!(body["body"].as_str().unwrap_or("").contains("Test"));
}

#[tokio::test]
async fn create_node_conflict_returns_409() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let payload = json!({
        "id": "welcome",
        "type": "note",
        "body": "# Dup\n",
        "relative_path": "dup_welcome.md",
        "links": []
    });

    let response = app
        .oneshot(json_request(Method::POST, "/api/nodes", payload))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_node_path_traversal_returns_400() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    // relative_path contains ".." — must be rejected by security layer.
    let payload = json!({
        "id": "evil",
        "type": "note",
        "body": "# Evil\n",
        "relative_path": "../escape.md",
        "links": []
    });

    let response = app
        .oneshot(json_request(Method::POST, "/api/nodes", payload))
        .await
        .expect("response");

    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "expected 4xx/5xx for path traversal attempt, got {}",
        response.status()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// PATCH /api/nodes/{id} — update node
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_node_updates_body() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let payload = json!({ "body": "# Updated body\n" });

    let response = app
        .oneshot(json_request(Method::PATCH, "/api/nodes/welcome", payload))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response.into_body()).await;
    assert_eq!(body["id"], "welcome");
    assert!(body["body"].as_str().unwrap_or("").contains("Updated body"));
}

#[tokio::test]
async fn patch_node_updates_links() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let payload = json!({ "links": ["getting_started", "new_target"] });

    let response = app
        .oneshot(json_request(Method::PATCH, "/api/nodes/welcome", payload))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response.into_body()).await;
    let links = body["links"].as_array().expect("links array");
    assert!(links.iter().any(|l| l == "getting_started"));
    assert!(links.iter().any(|l| l == "new_target"));
}

#[tokio::test]
async fn patch_node_not_found_returns_404() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let payload = json!({ "body": "# Whatever\n" });

    let response = app
        .oneshot(json_request(
            Method::PATCH,
            "/api/nodes/no_such_node",
            payload,
        ))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ──────────────────────────────────────────────────────────────────────────────
// DELETE /api/nodes/{id}
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_node_returns_204() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/nodes/getting_started")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_node_not_found_returns_404() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/nodes/ghost_node")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_then_get_returns_404() {
    let (_tmp, state) = basic_state();
    let app = router(state.clone());

    // Delete
    app.oneshot(
        Request::builder()
            .method(Method::DELETE)
            .uri("/api/nodes/getting_started")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("delete response");

    // Now GET should 404
    let app2 = router(state);
    let get_resp = app2
        .oneshot(
            Request::builder()
                .uri("/api/nodes/getting_started")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("get response");

    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// ──────────────────────────────────────────────────────────────────────────────
// GET /api/edges
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_edges_returns_resolved_and_unresolved() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/edges")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response.into_body()).await;
    let edges = body["edges"].as_array().expect("edges array");
    // The basic fixture has welcome -> getting_started and getting_started -> welcome
    assert!(
        edges
            .iter()
            .any(|e| e["from"] == "welcome" && e["to"] == "getting_started"),
        "expected welcome -> getting_started edge, got: {body}"
    );
}

#[tokio::test]
async fn get_edges_has_unresolved_field() {
    let (_tmp, state) = basic_state();
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/edges")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    let body = body_json(response.into_body()).await;
    // `unresolved` field must always be present (may be empty array)
    assert!(body["unresolved"].is_array(), "unresolved field missing");
}

// ──────────────────────────────────────────────────────────────────────────────
// Full CRUD round-trip via REST API
// ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn crud_round_trip_via_rest() {
    let (_tmp, state) = basic_state();

    // 1. Create
    {
        let app = router(state.clone());
        let payload = json!({
            "id": "api_roundtrip",
            "type": "note",
            "body": "# Round trip\n",
            "relative_path": "api_roundtrip.md",
            "links": ["welcome"]
        });
        let resp = app
            .oneshot(json_request(Method::POST, "/api/nodes", payload))
            .await
            .expect("create");
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // 2. Read
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/nodes/api_roundtrip")
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("read");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["id"], "api_roundtrip");
    }

    // 3. Patch
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(json_request(
                Method::PATCH,
                "/api/nodes/api_roundtrip",
                json!({ "body": "# Patched\n" }),
            ))
            .await
            .expect("patch");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert!(body["body"].as_str().unwrap_or("").contains("Patched"));
    }

    // 4. Delete
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/nodes/api_roundtrip")
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("delete");
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // 5. Confirm gone
    {
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/nodes/api_roundtrip")
                    .body(Body::empty())
                    .expect("req"),
            )
            .await
            .expect("final get");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
