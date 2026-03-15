use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Python wrapper around `synapse_client::Client`.
#[pyclass]
struct SynapseClient {
    client: synapse_client::Client,
    rt: tokio::runtime::Runtime,
}

#[pymethods]
impl SynapseClient {
    #[new]
    #[pyo3(signature = (base_url = "http://localhost:8080", timeout_secs = None))]
    fn new(base_url: &str, timeout_secs: Option<u64>) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {e}")))?;

        let client = match timeout_secs {
            Some(secs) => {
                synapse_client::Client::with_timeout(base_url, std::time::Duration::from_secs(secs))
            }
            None => synapse_client::Client::new(base_url),
        };

        Ok(Self { client, rt })
    }

    /// Emit an event to trigger a handler.
    ///
    /// Args:
    ///     event: Event name (e.g. "save", "user_message")
    ///     payload: Dict of event data
    ///
    /// Returns:
    ///     Dict with the handler result
    #[pyo3(signature = (event, payload = None))]
    fn emit(
        &self,
        py: Python<'_>,
        event: &str,
        payload: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let payload_json = python_to_json(py, payload)?;

        let result = self
            .rt
            .block_on(self.client.emit(event, payload_json))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        json_to_python(py, &result)
    }

    /// Execute a named query.
    ///
    /// Args:
    ///     query_name: Name of the query to execute
    ///     params: Dict of query parameters
    ///
    /// Returns:
    ///     Query results as a list of dicts
    #[pyo3(signature = (query_name, params = None))]
    fn query(
        &self,
        py: Python<'_>,
        query_name: &str,
        params: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let params_json = python_to_json(py, params)?;

        let result = self
            .rt
            .block_on(self.client.query(query_name, params_json))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        json_to_python(py, &result)
    }

    /// Check if the runtime is reachable.
    fn ping(&self) -> PyResult<bool> {
        Ok(self.rt.block_on(self.client.ping()))
    }

    /// Get runtime health info.
    fn health(&self, py: Python<'_>) -> PyResult<PyObject> {
        let resp = self
            .rt
            .block_on(self.client.health())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("status", &resp.status)?;
        dict.set_item("uptime_secs", resp.uptime_secs)?;
        Ok(dict.into_any().unbind())
    }

    /// Get runtime status info.
    fn status(&self, py: Python<'_>) -> PyResult<PyObject> {
        let resp = self
            .rt
            .block_on(self.client.status())
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("status", &resp.status)?;
        dict.set_item("uptime_secs", resp.uptime_secs)?;
        dict.set_item("handlers", resp.handlers.clone())?;
        dict.set_item("queries", resp.queries.clone())?;
        dict.set_item("memories", resp.memories.clone())?;
        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        "SynapseClient(...)".to_string()
    }
}

/// Convert a Python object (dict/list/scalar/None) to serde_json::Value.
fn python_to_json(_py: Python<'_>, obj: Option<&Bound<'_, PyAny>>) -> PyResult<serde_json::Value> {
    let Some(obj) = obj else {
        return Ok(serde_json::json!({}));
    };

    // If it's a string, try parsing as JSON first
    if let Ok(s) = obj.extract::<String>() {
        return serde_json::from_str(&s)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON string: {e}")));
    }

    // Convert via Python's json module
    let json_mod = obj.py().import("json")?;
    let json_str: String = json_mod.call_method1("dumps", (obj,))?.extract()?;

    serde_json::from_str(&json_str)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON conversion failed: {e}")))
}

/// Convert serde_json::Value to a Python object.
fn json_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    let json_str = serde_json::to_string(value)
        .map_err(|e| PyRuntimeError::new_err(format!("JSON serialization failed: {e}")))?;

    let json_mod = py.import("json")?;
    let result = json_mod.call_method1("loads", (json_str,))?;
    Ok(result.unbind())
}

/// The Python module.
#[pymodule]
fn synapse(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SynapseClient>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
