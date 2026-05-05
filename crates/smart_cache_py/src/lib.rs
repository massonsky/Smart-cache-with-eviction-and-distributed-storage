use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use smart_cache_core::{Cache, CacheConfig, EvictionPolicy};
use std::sync::{Arc, Mutex, MutexGuard};

#[pyclass(name = "EvictionPolicy", from_py_object)]
#[derive(Clone, Copy)]
pub enum PyEvictionPolicy {
    LRU,
    LFU,
    FIFO,
}

impl From<PyEvictionPolicy> for EvictionPolicy {
    fn from(value: PyEvictionPolicy) -> Self {
        match value {
            PyEvictionPolicy::LRU => Self::Lru,
            PyEvictionPolicy::LFU => Self::Lfu,
            PyEvictionPolicy::FIFO => Self::Fifo,
        }
    }
}

#[pyclass(name = "Cache")]
pub struct PyCache {
    inner: Arc<Mutex<Cache>>,
}

#[pymethods]
impl PyCache {
    #[new]
    #[pyo3(signature = (capacity, policy, ttl_ms = None))]
    fn new(capacity: usize, policy: PyEvictionPolicy, ttl_ms: Option<u64>) -> PyResult<Self> {
        let cache = Cache::try_new(CacheConfig {
            capacity,
            policy: policy.into(),
            default_ttl_ms: ttl_ms,
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(cache)),
        })
    }

    fn put(&self, key: String, value: &[u8]) -> PyResult<()> {
        self.lock()?.put(key, value.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> PyResult<Option<Vec<u8>>> {
        Ok(self.lock()?.get(key))
    }

    fn remove(&self, key: &str) -> PyResult<bool> {
        Ok(self.lock()?.remove(key))
    }

    fn clear(&self) -> PyResult<()> {
        self.lock()?.clear();
        Ok(())
    }

    fn stats(&self) -> PyResult<PyCacheStats> {
        let stats = self.lock()?.stats();
        Ok(PyCacheStats {
            hits: stats.hits,
            misses: stats.misses,
            puts: stats.puts,
            updates: stats.updates,
            removes: stats.removes,
            evictions: stats.evictions,
            expirations: stats.expirations,
        })
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.lock()?.len())
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        Ok(self.lock()?.contains_key(key))
    }
}

impl PyCache {
    fn lock(&self) -> PyResult<MutexGuard<'_, Cache>> {
        self.inner
            .lock()
            .map_err(|_| PyValueError::new_err("cache mutex is poisoned"))
    }
}

#[pyclass(name = "CacheStats", frozen)]
pub struct PyCacheStats {
    #[pyo3(get)]
    hits: u64,
    #[pyo3(get)]
    misses: u64,
    #[pyo3(get)]
    puts: u64,
    #[pyo3(get)]
    updates: u64,
    #[pyo3(get)]
    removes: u64,
    #[pyo3(get)]
    evictions: u64,
    #[pyo3(get)]
    expirations: u64,
}

#[pymodule]
fn smart_cache(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEvictionPolicy>()?;
    m.add_class::<PyCache>()?;
    m.add_class::<PyCacheStats>()?;
    Ok(())
}
