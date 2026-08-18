//! Python bindings via PyO3.
//!
//! This module provides Python bindings for embeddenator core types,
//! enabling seamless interop with Python data science tools.
//!
//! ## Example (Python)
//!
//! ```python
//! from embeddenator_interop import SparseVec, VSAConfig
//!
//! # Create vectors
//! vec1 = SparseVec.new()
//! vec2 = SparseVec.new()
//!
//! # Perform operations
//! bundled = vec1.bundle(vec2)
//! similarity = vec1.cosine(vec2)
//!
//! # Encode data
//! config = VSAConfig.new()
//! data = b"Hello from Python!"
//! encoded = config.encode(data)
//! decoded = encoded.decode(config, len(data))
//! ```

#[cfg(feature = "python")]
use embeddenator_vsa::{ReversibleVSAConfig, SparseVec};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::PyBytes;

#[cfg(feature = "python")]
#[pyclass(name = "SparseVec")]
pub struct PySparseVec {
    inner: SparseVec,
}

#[cfg(feature = "python")]
impl Default for PySparseVec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl PySparseVec {
    /// Create a new empty SparseVec
    #[new]
    pub fn new() -> Self {
        Self {
            inner: SparseVec::new(),
        }
    }

    /// Create from positive and negative indices
    #[staticmethod]
    pub fn from_indices(pos: Vec<usize>, neg: Vec<usize>) -> Self {
        Self {
            inner: SparseVec { pos, neg },
        }
    }

    /// Get positive indices
    #[getter]
    pub fn pos(&self) -> Vec<usize> {
        self.inner.pos.clone()
    }

    /// Get negative indices
    #[getter]
    pub fn neg(&self) -> Vec<usize> {
        self.inner.neg.clone()
    }

    /// Bundle this vector with another
    pub fn bundle(&self, other: &PySparseVec) -> PySparseVec {
        PySparseVec {
            inner: self.inner.bundle(&other.inner),
        }
    }

    /// Bind this vector with another
    pub fn bind(&self, other: &PySparseVec) -> PySparseVec {
        PySparseVec {
            inner: self.inner.bind(&other.inner),
        }
    }

    /// Compute cosine similarity with another vector
    pub fn cosine(&self, other: &PySparseVec) -> f64 {
        self.inner.cosine(&other.inner)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Deserialize from JSON
    #[staticmethod]
    pub fn from_json(json: &str) -> PyResult<PySparseVec> {
        let inner: SparseVec = serde_json::from_str(json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PySparseVec { inner })
    }

    /// Serialize to bytes (bincode)
    pub fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = bincode::serialize(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Deserialize from bytes (bincode)
    #[staticmethod]
    pub fn from_bytes(bytes: &[u8]) -> PyResult<PySparseVec> {
        let inner: SparseVec = bincode::deserialize(bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PySparseVec { inner })
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "SparseVec(pos={:?}, neg={:?})",
            &self.inner.pos[..self.inner.pos.len().min(5)],
            &self.inner.neg[..self.inner.neg.len().min(5)]
        )
    }

    /// Length (number of non-zero elements)
    pub fn __len__(&self) -> usize {
        self.inner.pos.len() + self.inner.neg.len()
    }
}

#[cfg(feature = "python")]
#[pyclass(name = "VSAConfig")]
pub struct PyVSAConfig {
    inner: ReversibleVSAConfig,
}

#[cfg(feature = "python")]
impl Default for PyVSAConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl PyVSAConfig {
    /// Create a new default VSAConfig
    #[new]
    pub fn new() -> Self {
        Self {
            inner: ReversibleVSAConfig::default(),
        }
    }

    /// Create a custom VSAConfig
    #[staticmethod]
    pub fn custom(
        block_size: usize,
        max_path_depth: usize,
        base_shift: usize,
        target_sparsity: usize,
    ) -> Self {
        Self {
            inner: ReversibleVSAConfig {
                block_size,
                max_path_depth,
                base_shift,
                target_sparsity,
            },
        }
    }

    /// Get block size
    #[getter]
    pub fn block_size(&self) -> usize {
        self.inner.block_size
    }

    /// Get max path depth
    #[getter]
    pub fn max_path_depth(&self) -> usize {
        self.inner.max_path_depth
    }

    /// Get base shift
    #[getter]
    pub fn base_shift(&self) -> usize {
        self.inner.base_shift
    }

    /// Get target sparsity
    #[getter]
    pub fn target_sparsity(&self) -> usize {
        self.inner.target_sparsity
    }

    /// Encode data into a SparseVec
    pub fn encode(&self, data: &[u8], path: Option<&str>) -> PySparseVec {
        PySparseVec {
            inner: SparseVec::encode_data(data, &self.inner, path),
        }
    }

    /// Decode a SparseVec back to data
    pub fn decode<'py>(
        &self,
        py: Python<'py>,
        vec: &PySparseVec,
        expected_size: usize,
        path: Option<&str>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let decoded = vec.inner.decode_data(&self.inner, path, expected_size);
        Ok(PyBytes::new(py, &decoded))
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Deserialize from JSON
    #[staticmethod]
    pub fn from_json(json: &str) -> PyResult<PyVSAConfig> {
        let inner: ReversibleVSAConfig = serde_json::from_str(json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyVSAConfig { inner })
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "VSAConfig(block_size={}, max_path_depth={}, base_shift={}, target_sparsity={})",
            self.inner.block_size,
            self.inner.max_path_depth,
            self.inner.base_shift,
            self.inner.target_sparsity
        )
    }
}

/// Python module initialization
#[cfg(feature = "python")]
#[pymodule]
fn embeddenator_interop(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySparseVec>()?;
    m.add_class::<PyVSAConfig>()?;
    Ok(())
}

#[cfg(all(test, feature = "python"))]
mod tests {
    use super::*;

    #[test]
    fn test_py_sparse_vec_creation() {
        let vec = PySparseVec::new();
        assert_eq!(vec.pos().len(), 0);
        assert_eq!(vec.neg().len(), 0);
    }

    #[test]
    fn test_py_sparse_vec_operations() {
        let vec1 = PySparseVec::from_indices(vec![1, 2, 3], vec![4, 5]);
        let vec2 = PySparseVec::from_indices(vec![2, 3, 4], vec![5, 6]);

        let bundled = vec1.bundle(&vec2);
        assert!(bundled.pos().len() > 0 || bundled.neg().len() > 0);

        let bound = vec1.bind(&vec2);
        assert!(bound.pos().len() > 0 || bound.neg().len() > 0);

        let similarity = vec1.cosine(&vec2);
        assert!(similarity.is_finite());
    }

    #[test]
    fn test_py_vsa_config() {
        let config = PyVSAConfig::new();
        assert!(config.block_size() > 0);
        assert!(config.max_path_depth() > 0);

        let custom = PyVSAConfig::custom(256, 10, 1000, 200);
        assert_eq!(custom.block_size(), 256);
        assert_eq!(custom.max_path_depth(), 10);
        assert_eq!(custom.base_shift(), 1000);
        assert_eq!(custom.target_sparsity(), 200);
    }

    #[test]
    fn test_py_encode_decode() {
        use pyo3::Python;

        Python::attach(|py| {
            let config = PyVSAConfig::new();
            let data = b"Hello from Python!";

            let encoded = config.encode(data, None);
            assert!(encoded.pos().len() > 0 || encoded.neg().len() > 0);

            let decoded = config.decode(py, &encoded, data.len(), None).unwrap();
            let decoded_bytes = decoded.as_bytes();
            assert_eq!(decoded_bytes, data);
        });
    }
}
