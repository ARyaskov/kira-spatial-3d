use core::fmt;

/// Crate-level errors for deterministic mesh projection.
#[derive(Debug)]
pub enum Error {
    /// Regular grid parameters are invalid.
    InvalidDomain {
        nx: usize,
        ny: usize,
        step_x: f32,
        step_y: f32,
    },
    /// Scalar field values length does not match domain cell count.
    LengthMismatch { expected: usize, got: usize },
    /// The domain cannot be indexed with `u32`.
    IndexOverflow { vertex_count: usize },
    /// Normalization options are invalid.
    InvalidNormalization { message: &'static str },
    /// Height mapping specification is invalid.
    InvalidHeightSpec { message: &'static str },
    /// Contour extraction specification is invalid.
    InvalidContourSpec { message: &'static str },
    /// Export specification is invalid.
    InvalidExportSpec { message: &'static str },
    /// IO layer error.
    Io(std::io::Error),
    /// JSON serialization error.
    SerdeJson(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomain {
                nx,
                ny,
                step_x,
                step_y,
            } => write!(
                f,
                "invalid domain: nx={nx}, ny={ny}, step_x={step_x}, step_y={step_y}"
            ),
            Self::LengthMismatch { expected, got } => {
                write!(f, "length mismatch: expected {expected}, got {got}")
            }
            Self::IndexOverflow { vertex_count } => {
                write!(
                    f,
                    "index overflow: vertex count {vertex_count} exceeds u32::MAX"
                )
            }
            Self::InvalidNormalization { message } => {
                write!(f, "invalid normalization: {message}")
            }
            Self::InvalidHeightSpec { message } => {
                write!(f, "invalid height spec: {message}")
            }
            Self::InvalidContourSpec { message } => {
                write!(f, "invalid contour spec: {message}")
            }
            Self::InvalidExportSpec { message } => {
                write!(f, "invalid export spec: {message}")
            }
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::SerdeJson(err) => write!(f, "serde json error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}

/// Regular 2D spatial domain used as the base for deterministic projection.
///
/// # Determinism
/// Indexing is row-major with `y` as the outer scanline and `x` as the inner axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialDomain {
    pub nx: usize,
    pub ny: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub step_x: f32,
    pub step_y: f32,
}

impl SpatialDomain {
    /// Creates and validates a regular grid domain.
    pub fn new(
        nx: usize,
        ny: usize,
        origin_x: f32,
        origin_y: f32,
        step_x: f32,
        step_y: f32,
    ) -> Result<Self, Error> {
        let domain = Self {
            nx,
            ny,
            origin_x,
            origin_y,
            step_x,
            step_y,
        };
        domain.validate()?;
        Ok(domain)
    }

    /// Validates domain invariants.
    pub fn validate(&self) -> Result<(), Error> {
        if self.nx < 2 || self.ny < 2 || self.step_x <= 0.0 || self.step_y <= 0.0 {
            return Err(Error::InvalidDomain {
                nx: self.nx,
                ny: self.ny,
                step_x: self.step_x,
                step_y: self.step_y,
            });
        }
        Ok(())
    }

    /// Number of scalar samples in row-major order.
    #[inline]
    pub fn len(&self) -> usize {
        self.nx * self.ny
    }

    /// Returns `true` if the domain has zero samples.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts `(x, y)` to row-major linear index (`y * nx + x`).
    ///
    /// Panics if `(x, y)` is out of bounds.
    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        assert!(x < self.nx, "x out of bounds: {x} >= {}", self.nx);
        assert!(y < self.ny, "y out of bounds: {y} >= {}", self.ny);
        y * self.nx + x
    }

    /// Converts row-major linear index back to `(x, y)`.
    ///
    /// Panics if `idx` is out of bounds.
    #[inline]
    pub fn xy(&self, idx: usize) -> (usize, usize) {
        assert!(
            idx < self.len(),
            "index out of bounds: {idx} >= {}",
            self.len()
        );
        (idx % self.nx, idx / self.nx)
    }

    /// Returns world-space `(x, y)` for grid coordinate `(x, y)`.
    ///
    /// Panics if `(x, y)` is out of bounds.
    #[inline]
    pub fn pos(&self, x: usize, y: usize) -> (f32, f32) {
        let _ = self.idx(x, y);
        (
            self.origin_x + x as f32 * self.step_x,
            self.origin_y + y as f32 * self.step_y,
        )
    }
}

/// Borrowed scalar field over a validated regular domain.
#[derive(Debug, Clone, Copy)]
pub struct ScalarField<'a> {
    pub domain: SpatialDomain,
    pub values: &'a [f32],
}

impl<'a> ScalarField<'a> {
    /// Creates a borrowed scalar field and validates length/domain invariants.
    pub fn new(domain: SpatialDomain, values: &'a [f32]) -> Result<Self, Error> {
        domain.validate()?;
        if values.len() != domain.len() {
            return Err(Error::LengthMismatch {
                expected: domain.len(),
                got: values.len(),
            });
        }
        Ok(Self { domain, values })
    }

    /// Returns scalar value at `(x, y)` with bounds checks.
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        let idx = self.domain.idx(x, y);
        self.values[idx]
    }

    /// Returns scalar value at `(x, y)` without bounds checks.
    ///
    /// # Safety
    /// Caller must guarantee `x < nx` and `y < ny`.
    #[inline]
    pub unsafe fn get_unchecked(&self, x: usize, y: usize) -> f32 {
        let idx = y * self.domain.nx + x;
        unsafe { *self.values.get_unchecked(idx) }
    }
}

/// GPU-friendly triangle mesh.
///
/// Invariant: `vertices.len() == normals.len()` and `indices.len() % 3 == 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Builds and validates a mesh container.
    pub fn new(
        vertices: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        indices: Vec<u32>,
    ) -> Result<Self, Error> {
        if vertices.len() != normals.len() {
            return Err(Error::LengthMismatch {
                expected: vertices.len(),
                got: normals.len(),
            });
        }
        if !indices.len().is_multiple_of(3) {
            return Err(Error::LengthMismatch {
                expected: indices.len() / 3 * 3,
                got: indices.len(),
            });
        }
        Ok(Self {
            vertices,
            normals,
            indices,
        })
    }
}
