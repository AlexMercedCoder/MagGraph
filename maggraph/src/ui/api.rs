//! JSON REST handlers for the embedded dashboard.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml::Value as YamlValue;

use crate::error::MagGraphError;
use crate::graph::GraphAdjacency;
use crate::index::NodeIndexEntry;
use crate::node::{NewNode, NodeMetadata};

use super::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct NodeSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub relative_path: String,
    pub source: Option<String>,
    pub links: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

impl From<&NodeIndexEntry> for NodeSummary {
    fn from(entry: &NodeIndexEntry) -> Self {
        let extra = entry
            .metadata
            .extra
            .iter()
            .filter_map(|(k, v)| yaml_value_to_json(v).ok().map(|j| (k.clone(), j)))
            .collect();

        Self {
            id: entry.metadata.id.clone(),
            node_type: entry.metadata.node_type.clone(),
            relative_path: entry.relative_path.to_string_lossy().into_owned(),
            source: entry.metadata.source.clone(),
            links: entry.metadata.links.clone(),
            extra,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeDetail {
    #[serde(flatten)]
    pub summary: NodeSummary,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeRecord {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgesResponse {
    pub edges: Vec<EdgeRecord>,
    pub unresolved: Vec<UnresolvedEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedEdge {
    pub from: String,
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub body: String,
    pub relative_path: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PatchNodeRequest {
    pub body: Option<String>,
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    pub source: Option<Option<String>>,
    pub links: Option<Vec<String>>,
    pub extra: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

pub async fn list_nodes(State(state): State<AppState>) -> Result<Json<Vec<NodeSummary>>, ApiError> {
    let index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;
    let mut nodes: Vec<NodeSummary> = index.iter().map(|(_, e)| NodeSummary::from(e)).collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(nodes))
}

pub async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<NodeDetail>, ApiError> {
    let index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;
    let entry = index
        .get(&id)
        .ok_or_else(|| ApiError::not_found(format!("node {id} not found")))?;
    let summary = NodeSummary::from(entry);
    let node = index.read_node(&id).map_err(ApiError::from)?;
    Ok(Json(NodeDetail {
        summary,
        body: node.body,
    }))
}

pub async fn create_node(
    State(state): State<AppState>,
    Json(req): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<NodeDetail>), ApiError> {
    let mut index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;

    let extra = json_map_to_yaml(req.extra)?;
    let node = index
        .create_node(NewNode {
            metadata: NodeMetadata {
                id: req.id.clone(),
                node_type: req.node_type,
                source: req.source,
                links: req.links,
                extra,
            },
            body: req.body.clone(),
            relative_path: req.relative_path.into(),
        })
        .map_err(ApiError::from)?;

    let summary = NodeSummary::from(index.get(&req.id).expect("just inserted"));
    Ok((
        StatusCode::CREATED,
        Json(NodeDetail {
            summary,
            body: node.body,
        }),
    ))
}

pub async fn patch_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchNodeRequest>,
) -> Result<Json<NodeDetail>, ApiError> {
    let mut index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;

    let mut node = index.read_node(&id).map_err(ApiError::from)?;

    if let Some(body) = req.body {
        node.body = body;
    }
    if let Some(node_type) = req.node_type {
        node.metadata.node_type = node_type;
    }
    if let Some(source) = req.source {
        node.metadata.source = source;
    }
    if let Some(links) = req.links {
        node.metadata.links = links;
    }
    if let Some(extra) = req.extra {
        node.metadata.extra = json_map_to_yaml(extra)?;
    }

    index.update_node(node.clone()).map_err(ApiError::from)?;

    let summary = NodeSummary::from(index.get(&id).expect("still indexed"));
    Ok(Json(NodeDetail {
        summary,
        body: node.body,
    }))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;
    index.delete_node(&id).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_edges(State(state): State<AppState>) -> Result<Json<EdgesResponse>, ApiError> {
    let index = state
        .index
        .lock()
        .map_err(|_| ApiError::internal("index lock poisoned"))?;
    let adjacency = index.adjacency().map_err(ApiError::from)?;

    let edges = collect_edges(&adjacency);
    let unresolved = collect_unresolved(&adjacency);

    Ok(Json(EdgesResponse { edges, unresolved }))
}

fn collect_edges(adjacency: &GraphAdjacency) -> Vec<EdgeRecord> {
    let mut edges: Vec<EdgeRecord> = adjacency
        .outgoing_edges()
        .map(|(from, to)| EdgeRecord {
            from: from.to_string(),
            to: to.to_string(),
        })
        .collect();
    edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    edges
}

fn collect_unresolved(adjacency: &GraphAdjacency) -> Vec<UnresolvedEdge> {
    let mut unresolved: Vec<UnresolvedEdge> = adjacency
        .unresolved_edges()
        .map(|(from, target)| UnresolvedEdge {
            from: from.to_string(),
            target: target.to_string(),
        })
        .collect();
    unresolved.sort_by(|a, b| (&a.from, &a.target).cmp(&(&b.from, &b.target)));
    unresolved
}

fn yaml_value_to_json(value: &YamlValue) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|e| ApiError::internal(format!("yaml to json: {e}")))
}

fn json_map_to_yaml(map: BTreeMap<String, Value>) -> Result<BTreeMap<String, YamlValue>, ApiError> {
    map.into_iter()
        .map(|(k, v)| {
            let yaml: YamlValue = serde_json::from_value(v)
                .map_err(|e| ApiError::bad_request(format!("invalid extra field {k}: {e}")))?;
            Ok((k, yaml))
        })
        .collect()
}

/// API error type mapped to HTTP responses.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<MagGraphError> for ApiError {
    fn from(err: MagGraphError) -> Self {
        let status = match &err {
            MagGraphError::NodeNotFound { .. } => StatusCode::NOT_FOUND,
            MagGraphError::NodeAlreadyExists { .. } | MagGraphError::DuplicateNodeId { .. } => {
                StatusCode::CONFLICT
            }
            MagGraphError::ConfigValidation(_)
            | MagGraphError::NodeParse { .. }
            | MagGraphError::Index(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::GraphIndex;
    use std::path::PathBuf;

    #[test]
    fn node_summary_from_entry() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest.join("../examples/basic/knowledge_graph");
        let index = GraphIndex::open(&root).expect("open");
        let entry = index.get("welcome").expect("welcome");
        let summary = NodeSummary::from(entry);
        assert_eq!(summary.id, "welcome");
        assert!(!summary.links.is_empty());
    }
}
