pub mod height;
pub mod normalize;

pub use height::{HeightMapSpec, HeightMode, build_heights};
pub use normalize::{Normalization, NormalizeOptions, normalize, validate_normalization};

use crate::mesh::heightmap::{HeightmapOptions, build_heightmap_mesh};
use crate::{Error, Mesh, ScalarField, SpatialDomain};

/// Owned heights bound to a spatial domain.
///
/// This adapter allows reusing the canonical deterministic heightmap triangulation.
#[derive(Debug, Clone)]
pub struct HeightField {
    pub domain: SpatialDomain,
    pub heights: Vec<f32>,
}

impl HeightField {
    /// Returns a borrowed scalar field view over this owned height buffer.
    ///
    /// Panics if the internal buffer length does not match domain size.
    pub fn as_scalar_field(&self) -> ScalarField<'_> {
        assert_eq!(
            self.heights.len(),
            self.domain.len(),
            "height field length must match domain length"
        );
        ScalarField {
            domain: self.domain,
            values: &self.heights,
        }
    }
}

/// Convenience helper: map scalar values to deterministic heights and build a mesh.
pub fn build_heightmap_mesh_mapped(
    field: &ScalarField<'_>,
    spec: HeightMapSpec,
) -> Result<Mesh, Error> {
    let heights = build_heights(field, spec)?;
    let height_field = HeightField {
        domain: field.domain,
        heights,
    };
    build_heightmap_mesh(
        &height_field.as_scalar_field(),
        HeightmapOptions {
            z_scale: 1.0,
            z_offset: 0.0,
            compute: spec.compute,
        },
    )
}
