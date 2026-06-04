//! PyO3 bindings for MagGraph (Phase 7).

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
                let adj = index.adjacency()?;
                traverse(&adj, &index, &from_id, depth, traversal_order)
            })
            .await
            .map_err(|e| task_err(format!("task join error: {e}")))?
            .map(|inner| PyTraversalResult { inner })
            .map_err(map_err)
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "GraphIndex(root_path={:?}, nodes={})",
            self.root_path(),
            self.inner.len()
        )
    }
}

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

#[pymodule]
fn _maggraph(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("MagGraphError", m.py().get_type::<PyMagGraphError>())?;
    m.add_class::<PyResolvedConfig>()?;
    m.add_class::<PyGraphIndex>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyTraversalNode>()?;
    m.add_class::<PyTraversalResult>()?;
    m.add_function(wrap_pyfunction!(load_config, m)?)?;
    m.add_function(wrap_pyfunction!(open_index, m)?)?;
    Ok(())
}
