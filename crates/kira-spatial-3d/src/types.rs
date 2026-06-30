use core::fmt;

/// Crate-level errors.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidDomain {
        nx: usize,
        ny: usize,
        step_x: f32,
        step_y: f32,
    },
    LengthMismatch {
        expected: usize,
        got: usize,
    },
    IndexOverflow {
        vertex_count: usize,
    },
    InvalidNormalization {
        message: &'static str,
    },
    InvalidHeightSpec {
        message: &'static str,
    },
    InvalidContourSpec {
        message: &'static str,
    },
    InvalidExportSpec {
        message: &'static str,
    },
    InvalidMeshTopology {
        message: &'static str,
    },
    Io(std::io::Error),
    IoContext {
        path: std::path::PathBuf,
        operation: &'static str,
        source: std::io::Error,
    },
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
            Self::InvalidMeshTopology { message } => {
                write!(f, "invalid mesh topology: {message}")
            }
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::IoContext {
                path,
                operation,
                source,
            } => {
                write!(f, "io error while {operation} {}: {source}", path.display())
            }
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

/// Regular 2D spatial domain. Row-major: `y` outer, `x` inner.
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

    #[inline]
    pub fn len(&self) -> usize {
        self.nx * self.ny
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        assert!(x < self.nx, "x out of bounds: {x} >= {}", self.nx);
        assert!(y < self.ny, "y out of bounds: {y} >= {}", self.ny);
        y * self.nx + x
    }

    #[inline]
    pub fn xy(&self, idx: usize) -> (usize, usize) {
        assert!(
            idx < self.len(),
            "index out of bounds: {idx} >= {}",
            self.len()
        );
        (idx % self.nx, idx / self.nx)
    }

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

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        let idx = self.domain.idx(x, y);
        self.values[idx]
    }

    /// # Safety
    /// Caller must guarantee `x < nx` and `y < ny`.
    #[inline]
    #[allow(unsafe_code)]
    pub unsafe fn get_unchecked(&self, x: usize, y: usize) -> f32 {
        let idx = y * self.domain.nx + x;
        unsafe { *self.values.get_unchecked(idx) }
    }
}

/// Owned scalar field. Use [`OwnedScalarField::as_view`] for algorithms.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedScalarField {
    pub domain: SpatialDomain,
    pub values: Vec<f32>,
}

impl OwnedScalarField {
    pub fn new(domain: SpatialDomain, values: Vec<f32>) -> Result<Self, Error> {
        domain.validate()?;
        if values.len() != domain.len() {
            return Err(Error::LengthMismatch {
                expected: domain.len(),
                got: values.len(),
            });
        }
        Ok(Self { domain, values })
    }

    #[inline]
    pub fn as_view(&self) -> ScalarField<'_> {
        ScalarField {
            domain: self.domain,
            values: &self.values,
        }
    }

    pub fn into_parts(self) -> (SpatialDomain, Vec<f32>) {
        (self.domain, self.values)
    }
}

impl<'a> From<&'a OwnedScalarField> for ScalarField<'a> {
    fn from(value: &'a OwnedScalarField) -> Self {
        value.as_view()
    }
}

#[cfg(feature = "with-field")]
pub fn from_kira_field<'a>(
    domain: SpatialDomain,
    field: &'a kira_spatial_field::Field,
) -> Result<ScalarField<'a>, Error> {
    ScalarField::new(domain, field.values())
}

/// AABB of a mesh's vertex positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl MeshBounds {
    pub fn center(&self) -> [f32; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    pub fn radius(&self) -> f32 {
        let c = self.center();
        let dx = self.max[0] - c[0];
        let dy = self.max[1] - c[1];
        let dz = self.max[2] - c[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// GPU-friendly triangle mesh. Invariant: `vertices.len() == normals.len()`, `indices.len() % 3 == 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl Mesh {
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
            return Err(Error::InvalidMeshTopology {
                message: "indices length must be a multiple of 3",
            });
        }
        Ok(Self {
            vertices,
            normals,
            indices,
        })
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    #[inline]
    pub fn face_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn bounds(&self) -> Option<MeshBounds> {
        let first = *self.vertices.first()?;
        let mut min = first;
        let mut max = first;
        for &p in self.vertices.iter().skip(1) {
            for i in 0..3 {
                if p[i] < min[i] {
                    min[i] = p[i];
                }
                if p[i] > max[i] {
                    max[i] = p[i];
                }
            }
        }
        Some(MeshBounds { min, max })
    }
}
