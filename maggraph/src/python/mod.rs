//! PyO3 bindings for MagGraph (Phase 7 + T-F4 lakehouse extension).

use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};
use pyo3::BoundObject;
use pyo3_async_runtimes::tokio::future_into_py;
use serde_yaml::Value;

use crate::config::StorageMode;
use crate::error::MagGraphError as CoreError;
use crate::graph::{traverse, TraversalOrder, TraversalResult as CoreTraversalResult};
use crate::index::GraphIndex;
use crate::lakehouse::{
    LakehouseReader, NodeWithContent as CoreNodeWithContent, ResolvedContent as CoreResolvedContent,
};
use crate::node::{NewNode, Node as CoreNode, NodeMetadata};
use crate::MagGraphConfig;

pyo3::create_exception!(_maggraph, PyMagGraphError, pyo3::exceptions::PyException);

fn map_err(err: CoreError) -> PyErr {
    PyMagGraphError::new_err(err.to_string())
}

fn task_err(message: impl Into<String>) -> PyErr {
    PyMagGraphError::new_err(message.into())
}

fn parse_order(order: &str) -> PyResult<TraversalOrder> {
    match order.to_ascii_lowercase().as_str() {
        "bfs" => Ok(TraversalOrder::Bfs),
        "dfs" => Ok(TraversalOrder::Dfs),
        other => Err(PyValueError::new_err(format!(
            "order must be 'bfs' or 'dfs', got {other:?}"
        ))),
    }
}

fn storage_mode_label(mode: &StorageMode) -> &'static str {
    match mode {
        StorageMode::Local => "local",
        StorageMode::Lakehouse => "lakehouse",
    }
}

fn yaml_key_to_string(key: &Value) -> Option<String> {
    match key {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn into_py_any<'py, T>(py: Python<'py>, value: T) -> PyResult<Py<PyAny>>
where
    T: IntoPyObject<'py>,
    PyErr: From<T::Error>,
{
    Ok(value.into_pyobject(py)?.into_any().unbind())
}

fn yaml_value_to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(v) => into_py_any(py, *v),
        Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                into_py_any(py, i)
            } else if let Some(u) = v.as_u64() {
                into_py_any(py, u)
            } else if let Some(f) = v.as_f64() {
                into_py_any(py, f)
            } else {
                into_py_any(py, v.to_string())
            }
        }
        Value::String(v) => into_py_any(py, v.as_str()),
        Value::Sequence(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(yaml_value_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        Value::Mapping(map) => {
            let dict = PyDict::new(py);
            for (key, val) in map {
                if let Some(key_str) = yaml_key_to_string(key) {
                    dict.set_item(key_str, yaml_value_to_py(py, val)?)?;
                }
            }
            Ok(dict.into())
        }
        Value::Tagged(tagged) => yaml_value_to_py(py, &tagged.value),
    }
}

fn metadata_to_dict<'py>(py: Python<'py>, metadata: &NodeMetadata) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("id", &metadata.id)?;
    dict.set_item("type", &metadata.node_type)?;
    if let Some(source) = &metadata.source {
        dict.set_item("source", source)?;
    }
    dict.set_item("links", &metadata.links)?;
    for (key, value) in &metadata.extra {
        dict.set_item(key, yaml_value_to_py(py, value)?)?;
    }
    Ok(dict)
}

// ─────────────────────────────────────────────────────────────────────────────
// ResolvedContent
// ─────────────────────────────────────────────────────────────────────────────

/// Content returned when reading a node in Python.
///
/// `kind` is one of ``"local"``, ``"text"``, or ``"external_asset"``.
/// Use `.to_markdown()` to get a human/agent-friendly string regardless of kind.
#[pyclass(name = "ResolvedContent")]
#[derive(Clone)]
pub struct PyResolvedContent {
    inner: CoreResolvedContent,
}

#[pymethods]
impl PyResolvedContent {
    /// One of ``"local"``, ``"text"``, or ``"external_asset"``.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            CoreResolvedContent::LocalMarkdown { .. } => "local",
            CoreResolvedContent::Text { .. } => "text",
            CoreResolvedContent::ExternalAsset { .. } => "external_asset",
        }
    }

    /// The markdown body for ``"local"`` and ``"text"`` content; ``None`` for external assets.
    #[getter]
    fn body(&self) -> Option<&str> {
        match &self.inner {
            CoreResolvedContent::LocalMarkdown { body } => Some(body.as_str()),
            CoreResolvedContent::Text { body, .. } => Some(body.as_str()),
            CoreResolvedContent::ExternalAsset { .. } => None,
        }
    }

    /// The external URI for ``"text"`` and ``"external_asset"`` content; ``None`` for local.
    #[getter]
    fn uri(&self) -> Option<&str> {
        match &self.inner {
            CoreResolvedContent::LocalMarkdown { .. } => None,
            CoreResolvedContent::Text { uri, .. } => Some(uri.as_str()),
            CoreResolvedContent::ExternalAsset { uri, .. } => Some(uri.as_str()),
        }
    }

    /// The detected format for ``"external_asset"`` content (e.g. ``"parquet"``); ``None`` otherwise.
    #[getter]
    fn format(&self) -> Option<&str> {
        match &self.inner {
            CoreResolvedContent::ExternalAsset { format, .. } => Some(format.as_str()),
            _ => None,
        }
    }

    /// Size in bytes of the external asset, if known.
    #[getter]
    fn size_bytes(&self) -> Option<u64> {
        match &self.inner {
            CoreResolvedContent::ExternalAsset { metadata, .. } => metadata.size_bytes,
            _ => None,
        }
    }

    /// Whether the Parquet magic header is valid (external Parquet assets only).
    #[getter]
    fn parquet_magic_valid(&self) -> Option<bool> {
        match &self.inner {
            CoreResolvedContent::ExternalAsset { metadata, .. } => {
                metadata.parquet.as_ref().map(|p| p.magic_valid)
            }
            _ => None,
        }
    }

    /// Optional text snippet for external assets.
    #[getter]
    fn snippet(&self) -> Option<&str> {
        match &self.inner {
            CoreResolvedContent::ExternalAsset { snippet, .. } => snippet.as_deref(),
            _ => None,
        }
    }

    /// Markdown-friendly summary suitable for agent consumption.
    fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            CoreResolvedContent::LocalMarkdown { body } => {
                format!("ResolvedContent(kind='local', body_len={})", body.len())
            }
            CoreResolvedContent::Text { uri, body } => {
                format!(
                    "ResolvedContent(kind='text', uri={uri:?}, body_len={})",
                    body.len()
                )
            }
            CoreResolvedContent::ExternalAsset { uri, format, .. } => {
                format!("ResolvedContent(kind='external_asset', uri={uri:?}, format={format:?})")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NodeWithContent
// ─────────────────────────────────────────────────────────────────────────────

/// A graph node paired with its resolved external content.
///
/// Access the raw node via `.node` and the resolved content via `.content`.
#[pyclass(name = "NodeWithContent")]
pub struct PyNodeWithContent {
    inner: CoreNodeWithContent,
}

#[pymethods]
impl PyNodeWithContent {
    /// The graph node (metadata + markdown body).
    #[getter]
    fn node(&self) -> PyNode {
        PyNode {
            inner: self.inner.node.clone(),
        }
    }

    /// The resolved content (local markdown or external asset).
    #[getter]
    fn content(&self) -> PyResolvedContent {
        PyResolvedContent {
            inner: self.inner.content.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeWithContent(id={:?}, kind={:?})",
            self.inner.node.id(),
            match &self.inner.content {
                CoreResolvedContent::LocalMarkdown { .. } => "local",
                CoreResolvedContent::Text { .. } => "text",
                CoreResolvedContent::ExternalAsset { .. } => "external_asset",
            }
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LakehouseReader
// ─────────────────────────────────────────────────────────────────────────────

/// Reads node content according to the configured storage mode.
///
/// In ``"local"`` mode, returns the markdown body from disk.
/// In ``"lakehouse"`` mode, resolves the ``source`` / ``source_uri`` field
/// against the configured remote sources and returns the external content.
///
/// Construct via :meth:`ResolvedConfig.open_lakehouse_reader`.
///
/// Example::
///
///     config = maggraph.load_config("maggraph.toml")
///     index  = config.open_index()
///     reader = config.open_lakehouse_reader()
///
///     result = reader.read_node(index, "my_asset")
///     print(result.content.to_markdown())
#[pyclass(name = "LakehouseReader")]
pub struct PyLakehouseReader {
    // Arc<Mutex<…>> lets us share across Python threads and async tasks.
    inner: Arc<Mutex<LakehouseReader>>,
}

#[pymethods]
impl PyLakehouseReader {
    /// Read a node and resolve its external content synchronously.
    ///
    /// Returns a :class:`NodeWithContent` with both the node metadata/body
    /// and the resolved content (local or external).
    fn read_node(&self, index: &PyGraphIndex, node_id: &str) -> PyResult<PyNodeWithContent> {
        let mut reader = self
            .inner
            .lock()
            .map_err(|e| task_err(format!("reader lock poisoned: {e}")))?;
        let result = reader.read_node(&index.inner, node_id).map_err(map_err)?;
        Ok(PyNodeWithContent { inner: result })
    }

    /// Read a node and resolve its external content asynchronously.
    ///
    /// Returns an awaitable :class:`NodeWithContent`. Blocking Rust I/O runs
    /// on a Tokio thread pool so the Python event loop stays responsive.
    fn read_node_async<'py>(
        &self,
        py: Python<'py>,
        index: &PyGraphIndex,
        node_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let reader_arc = Arc::clone(&self.inner);
        let index_clone = index.inner.clone();
        future_into_py(py, async move {
            tokio::task::spawn_blocking(move || {
                let mut reader = reader_arc
                    .lock()
                    .map_err(|e| task_err(format!("reader lock poisoned: {e}")))?;
                reader
                    .read_node(&index_clone, &node_id)
                    .map_err(map_err)
                    .map(|inner| PyNodeWithContent { inner })
            })
            .await
            .map_err(|e| task_err(format!("task join error: {e}")))?
        })
    }

    /// Number of entries currently in the in-memory content cache.
    fn cache_len(&self) -> PyResult<usize> {
        let reader = self
            .inner
            .lock()
            .map_err(|e| task_err(format!("reader lock poisoned: {e}")))?;
        Ok(reader.cache().len())
    }

    /// Total bytes currently held in the in-memory content cache.
    fn cache_bytes(&self) -> PyResult<usize> {
        let reader = self
            .inner
            .lock()
            .map_err(|e| task_err(format!("reader lock poisoned: {e}")))?;
        Ok(reader.cache().current_bytes())
    }

    fn __repr__(&self) -> String {
        "LakehouseReader()".to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ResolvedConfig
// ─────────────────────────────────────────────────────────────────────────────

#[pyclass(name = "ResolvedConfig")]
#[derive(Clone)]
struct PyResolvedConfig {
    inner: crate::ResolvedConfig,
}

#[pymethods]
impl PyResolvedConfig {
    #[getter]
    fn root_path(&self) -> String {
        self.inner.root_path.display().to_string()
    }

    #[getter]
    fn config_path(&self) -> String {
        self.inner.config_path.display().to_string()
    }

    #[getter]
    fn storage_mode(&self) -> &'static str {
        storage_mode_label(&self.inner.config.storage.mode)
    }

    fn open_index(&self) -> PyResult<PyGraphIndex> {
        Ok(PyGraphIndex {
            inner: GraphIndex::open(&self.inner.root_path).map_err(map_err)?,
        })
    }

    /// Open a :class:`LakehouseReader` configured from this config.
    ///
    /// In ``"local"`` mode the reader simply returns markdown bodies.
    /// In ``"lakehouse"`` mode it resolves ``source`` URIs against remote sources.
    fn open_lakehouse_reader(&self) -> PyResult<PyLakehouseReader> {
        let reader = LakehouseReader::from_config(&self.inner);
        Ok(PyLakehouseReader {
            inner: Arc::new(Mutex::new(reader)),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ResolvedConfig(root_path={:?}, storage_mode={:?})",
            self.root_path(),
            self.storage_mode()
        )
    }
}

#[pyfunction]
fn load_config(path: &str) -> PyResult<PyResolvedConfig> {
    Ok(PyResolvedConfig {
        inner: MagGraphConfig::load(path).map_err(map_err)?,
    })
}

#[pyfunction]
fn open_index(root_path: &str) -> PyResult<PyGraphIndex> {
    Ok(PyGraphIndex {
        inner: GraphIndex::open(root_path).map_err(map_err)?,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// GraphIndex
// ─────────────────────────────────────────────────────────────────────────────

#[pyclass(name = "GraphIndex")]
struct PyGraphIndex {
    inner: GraphIndex,
}

#[pymethods]
impl PyGraphIndex {
    #[classmethod]
    fn open(_cls: &Bound<'_, PyType>, root_path: &str) -> PyResult<Self> {
        Ok(Self {
            inner: GraphIndex::open(root_path).map_err(map_err)?,
        })
    }

    #[getter]
    fn root_path(&self) -> String {
        self.inner.root_path().display().to_string()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn list_nodes(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.inner.iter().map(|(id, _)| id.to_string()).collect();
        ids.sort();
        ids
    }

    fn read_node(&self, node_id: &str) -> PyResult<PyNode> {
        Ok(PyNode {
            inner: self.inner.read_node(node_id).map_err(map_err)?,
        })
    }

    #[pyo3(signature = (node_id, node_type="note", body="", links=None))]
    fn create_node(
        &mut self,
        node_id: &str,
        node_type: &str,
        body: &str,
        links: Option<Vec<String>>,
    ) -> PyResult<PyNode> {
        let relative_path = format!("{node_id}.md").into();
        let new_node = NewNode {
            metadata: NodeMetadata {
                id: node_id.to_string(),
                node_type: node_type.to_string(),
                source: None,
                links: links.unwrap_or_default(),
                extra: Default::default(),
            },
            body: body.to_string(),
            relative_path,
        };
        Ok(PyNode {
            inner: self.inner.create_node(new_node).map_err(map_err)?,
        })
    }

    fn update_node(&mut self, node_id: &str, body: &str) -> PyResult<()> {
        let mut node = self.inner.read_node(node_id).map_err(map_err)?;
        node.body = body.to_string();
        self.inner.update_node(node).map_err(map_err)?;
        Ok(())
    }

    fn delete_node(&mut self, node_id: &str) -> PyResult<()> {
        self.inner.delete_node(node_id).map_err(map_err)?;
        Ok(())
    }

    fn read_node_async<'py>(
        &self,
        py: Python<'py>,
        node_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let index = self.inner.clone();
        future_into_py(py, async move {
            tokio::task::spawn_blocking(move || index.read_node(&node_id))
                .await
                .map_err(|e| task_err(format!("task join error: {e}")))?
                .map(|inner| PyNode { inner })
                .map_err(map_err)
        })
    }

    #[pyo3(signature = (from_id, depth=2, order="bfs"))]
    fn traverse(&self, from_id: &str, depth: u32, order: &str) -> PyResult<PyTraversalResult> {
        let order = parse_order(order)?;
        let adj = self.inner.adjacency().map_err(map_err)?;
        let result = traverse(&adj, &self.inner, from_id, depth, order).map_err(map_err)?;
        Ok(PyTraversalResult { inner: result })
    }

    #[pyo3(signature = (from_id, depth=2, order="bfs"))]
    fn traverse_async<'py>(
        &self,
        py: Python<'py>,
        from_id: String,
        depth: u32,
        order: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let traversal_order = parse_order(order)?;
        let index = self.inner.clone();
        future_into_py(py, async move {
            tokio::task::spawn_blocking(move || {
                let adj = index.adjacency().map_err(map_err)?;
                traverse(&adj, &index, &from_id, depth, traversal_order)
                    .map_err(map_err)
                    .map(|inner| PyTraversalResult { inner })
            })
            .await
            .map_err(|e| task_err(format!("task join error: {e}")))?
        })
    }

    /// Read a node with external content resolution via a :class:`LakehouseReader`.
    ///
    /// Equivalent to ``reader.read_node(index, node_id)`` but callable directly
    /// on the index. Useful when you already have a reader open.
    fn read_node_with_content(
        &self,
        reader: &PyLakehouseReader,
        node_id: &str,
    ) -> PyResult<PyNodeWithContent> {
        reader.read_node(self, node_id)
    }

    fn __repr__(&self) -> String {
        format!(
            "GraphIndex(root_path={:?}, nodes={})",
            self.root_path(),
            self.inner.len()
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node
// ─────────────────────────────────────────────────────────────────────────────

#[pyclass(name = "Node")]
struct PyNode {
    inner: CoreNode,
}

#[pymethods]
impl PyNode {
    #[getter]
    fn id(&self) -> &str {
        self.inner.id()
    }

    #[getter]
    fn node_type(&self) -> &str {
        &self.inner.metadata.node_type
    }

    #[getter]
    fn source(&self) -> Option<&str> {
        self.inner.metadata.source.as_deref()
    }

    #[getter]
    fn links(&self) -> Vec<String> {
        self.inner.metadata.links.clone()
    }

    #[getter]
    fn body(&self) -> &str {
        &self.inner.body
    }

    #[getter]
    fn relative_path(&self) -> String {
        self.inner.relative_path.display().to_string()
    }

    fn to_markdown(&self) -> PyResult<String> {
        self.inner.to_markdown().map_err(map_err)
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = metadata_to_dict(py, &self.inner.metadata)?;
        dict.set_item("body", &self.inner.body)?;
        dict.set_item("relative_path", self.relative_path())?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("Node(id={:?}, type={:?})", self.id(), self.node_type())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TraversalNode / TraversalResult
// ─────────────────────────────────────────────────────────────────────────────

#[pyclass(name = "TraversalNode")]
struct PyTraversalNode {
    inner: crate::graph::TraversalNode,
}

#[pymethods]
impl PyTraversalNode {
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn depth(&self) -> u32 {
        self.inner.depth
    }

    #[getter]
    fn path(&self) -> Vec<String> {
        self.inner.path.clone()
    }
}

#[pyclass(name = "TraversalResult")]
struct PyTraversalResult {
    inner: CoreTraversalResult,
}

#[pymethods]
impl PyTraversalResult {
    #[getter]
    fn start(&self) -> &str {
        &self.inner.start
    }

    #[getter]
    fn max_depth(&self) -> u32 {
        self.inner.max_depth
    }

    #[getter]
    fn order(&self) -> &'static str {
        match self.inner.order {
            TraversalOrder::Bfs => "bfs",
            TraversalOrder::Dfs => "dfs",
        }
    }

    #[getter]
    fn nodes(&self) -> Vec<PyTraversalNode> {
        self.inner
            .nodes
            .iter()
            .map(|node| PyTraversalNode {
                inner: node.clone(),
            })
            .collect()
    }

    fn to_markdown(&self, index: &PyGraphIndex) -> String {
        self.inner.to_markdown(&index.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "TraversalResult(start={:?}, nodes={})",
            self.start(),
            self.inner.nodes.len()
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Module registration
// ─────────────────────────────────────────────────────────────────────────────

#[pymodule]
fn _maggraph(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("MagGraphError", m.py().get_type::<PyMagGraphError>())?;
    m.add_class::<PyResolvedConfig>()?;
    m.add_class::<PyGraphIndex>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyTraversalNode>()?;
    m.add_class::<PyTraversalResult>()?;
    // T-F4: lakehouse bindings
    m.add_class::<PyResolvedContent>()?;
    m.add_class::<PyNodeWithContent>()?;
    m.add_class::<PyLakehouseReader>()?;
    m.add_function(wrap_pyfunction!(load_config, m)?)?;
    m.add_function(wrap_pyfunction!(open_index, m)?)?;
    Ok(())
}
