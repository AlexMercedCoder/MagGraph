//! Embedded local web dashboard (Phase 9).
//!
//! Serves a minimal HTML UI and JSON REST API bound to loopback by default.

mod api;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::config::ResolvedConfig;
use crate::error::{MagGraphError, Result};
use crate::index::GraphIndex;

pub use api::{
    create_node, delete_node, get_node, list_edges, list_nodes, patch_node, CreateNodeRequest,
    PatchNodeRequest,
};

/// Shared application state for HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub index: Arc<Mutex<GraphIndex>>,
}

/// Options for starting the UI server.
#[derive(Debug, Clone)]
pub struct UiServerOptions {
    pub bind: SocketAddr,
    pub resolved: ResolvedConfig,
}

impl UiServerOptions {
    /// Build options with loopback bind address validation.
    pub fn new(host: &str, port: u16, resolved: ResolvedConfig) -> Result<Self> {
        let bind = parse_loopback_addr(host, port)?;
        Ok(Self { bind, resolved })
    }
}

/// Build the Axum router (API + embedded static assets).
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/nodes", get(api::list_nodes).post(api::create_node))
        .route(
            "/nodes/{id}",
            get(api::get_node)
                .patch(api::patch_node)
                .delete(api::delete_node),
        )
        .route("/edges", get(api::list_edges));

    Router::new()
        .nest("/api", api)
        .route("/", get(serve_index))
        .route("/app.js", get(serve_app_js))
        .route("/style.css", get(serve_style))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

/// Run the UI server until interrupted (Ctrl+C).
pub async fn run(options: UiServerOptions) -> Result<()> {
    let index = GraphIndex::open(&options.resolved.root_path)?;
    let state = AppState {
        index: Arc::new(Mutex::new(index)),
    };

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(options.bind)
        .await
        .map_err(|source| {
            MagGraphError::Index(format!("failed to bind {}: {source}", options.bind))
        })?;

    tracing::info!(
        url = %format!("http://{}", options.bind),
        root = %options.resolved.root_path.display(),
        "MagGraph UI listening (localhost only)"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| MagGraphError::Index(format!("UI server error: {e}")))?;

    Ok(())
}

async fn serve_index() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("assets/index.html"),
    )
}

async fn serve_app_js() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("assets/app.js"),
    )
}

async fn serve_style() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("assets/style.css"),
    )
}

/// Parse and validate that the bind address is loopback-only (MVP security).
pub fn parse_loopback_addr(host: &str, port: u16) -> Result<SocketAddr> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| MagGraphError::ConfigValidation(format!("invalid bind address: {e}")))?;

    if !is_loopback(&addr) {
        return Err(MagGraphError::ConfigValidation(format!(
            "UI server must bind to loopback (127.0.0.1 or ::1); got {}",
            addr.ip()
        )));
    }

    Ok(addr)
}

fn is_loopback(addr: &SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(v4) => v4.is_loopback(),
        std::net::IpAddr::V6(v6) => v6.is_loopback(),
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::index::GraphIndex;
    use std::path::PathBuf;

    fn example_state() -> AppState {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest.join("../examples/basic/knowledge_graph");
        AppState {
            index: Arc::new(Mutex::new(GraphIndex::open(&root).expect("open"))),
        }
    }

    #[test]
    fn rejects_non_loopback_bind() {
        let err = parse_loopback_addr("0.0.0.0", 8787).expect_err("non-loopback");
        assert!(matches!(err, MagGraphError::ConfigValidation(_)));
    }

    #[test]
    fn accepts_ipv4_loopback() {
        let addr = parse_loopback_addr("127.0.0.1", 0).expect("loopback");
        assert!(addr.ip().is_loopback());
    }

    #[tokio::test]
    async fn api_lists_example_nodes() {
        let app = router(example_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nodes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let nodes: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(nodes.len() >= 2);
        assert!(nodes.iter().any(|n| n["id"] == "welcome"));
    }

    #[tokio::test]
    async fn api_gets_node_body() {
        let app = router(example_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nodes/welcome")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(detail["id"], "welcome");
        assert!(detail["body"].as_str().unwrap_or("").contains("Welcome"));
    }

    #[tokio::test]
    async fn dashboard_html_served() {
        let app = router(example_state());
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("MagGraph"));
    }
}
