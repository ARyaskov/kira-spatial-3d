use crate::types::{Error, Mesh, ScalarField};
use crate::{ComputeConfig, simd};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightmapOptions {
    pub z_scale: f32,
    pub z_offset: f32,
    pub compute: ComputeConfig,
}

impl Default for HeightmapOptions {
    fn default() -> Self {
        Self {
            z_scale: 1.0,
            z_offset: 0.0,
            compute: ComputeConfig::default(),
        }
    }
}

/// Build a heightmap mesh from a regular scalar grid in row-major scanline order.
pub fn build_heightmap_mesh(
    field: &ScalarField<'_>,
    opts: HeightmapOptions,
) -> Result<Mesh, Error> {
    field.domain.validate()?;
    if field.values.len() != field.domain.len() {
        return Err(Error::LengthMismatch {
            expected: field.domain.len(),
            got: field.values.len(),
        });
    }

    let domain = field.domain;
    let vertex_count = domain.len();
    if vertex_count > u32::MAX as usize {
        return Err(Error::IndexOverflow { vertex_count });
    }

    let mut heights = Vec::with_capacity(vertex_count);
    let mut vertices = Vec::with_capacity(vertex_count);
    for y in 0..domain.ny {
        let yw = domain.origin_y + y as f32 * domain.step_y;
        let row_start = y * domain.nx;
        for x in 0..domain.nx {
            let v = field.values[row_start + x];
            let zw = opts.z_offset + opts.z_scale * v;
            let xw = domain.origin_x + x as f32 * domain.step_x;
            heights.push(zw);
            vertices.push([xw, yw, zw]);
        }
    }

    let mut normals = vec![[0.0_f32; 3]; vertex_count];
    simd::compute_normals_heightmap(
        domain.nx,
        domain.ny,
        domain.step_x,
        domain.step_y,
        &heights,
        &mut normals,
    );

    let cell_count = (domain.nx - 1) * (domain.ny - 1);
    let mut indices = Vec::with_capacity(cell_count * 6);
    let nx_u32 = domain.nx as u32;
    for y in 0..(domain.ny - 1) {
        let row = y as u32 * nx_u32;
        let next_row = row + nx_u32;
        for x in 0..(domain.nx - 1) {
            let xu = x as u32;
            let a = row + xu;
            let b = a + 1;
            let c = next_row + xu;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    Mesh::new(vertices, normals, indices)
}
