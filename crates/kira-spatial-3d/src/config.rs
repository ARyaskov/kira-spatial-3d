/// Runtime backend selection for deterministic compute kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputeBackend {
    /// Use best available backend for this binary/CPU.
    Auto,
    /// Force scalar reference implementation.
    Scalar,
}

/// Runtime compute configuration threaded through mapping/mesh pipelines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeConfig {
    pub backend: ComputeBackend,
}

impl Default for ComputeConfig {
    fn default() -> Self {
        Self {
            backend: ComputeBackend::Auto,
        }
    }
}
