pub mod height;
pub mod normalize;

pub use height::{HeightMapSpec, HeightMode, build_heights};
pub use normalize::{
    Normalization, NormalizeOptions, normalize, normalize_with, validate_normalization,
};

use crate::mesh::heightmap::{HeightmapOptions, build_heightmap_mesh};
use crate::{Error, Mesh, ScalarField, SpatialDomain};

/// Owned heights bound to a spatial domain.
#[derive(Debug, Clone)]
pub struct HeightField {
    pub domain: SpatialDomain,
    pub heights: Vec<f32>,
}

impl HeightField {
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

/// Map scalar values to heights and build a heightmap mesh in one call.
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
