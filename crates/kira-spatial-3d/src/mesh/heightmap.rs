use crate::types::{Error, Mesh, ScalarField};
use crate::{ComputeConfig, simd};

/// Options controlling scalar-to-height projection.
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

/// Builds a deterministic heightmap mesh from a regular scalar grid.
///
/// # Determinism guarantees
/// - Vertex `i` is always the row-major `(x, y)` sample from the input domain.
/// - Triangles are emitted in fixed scanline order with fixed winding:
///   `(a, b, c)` and `(b, d, c)` per cell.
/// - Normals use fixed finite-difference rules (central interior, one-sided edges).
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

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut heights = Vec::with_capacity(vertex_count);

    for idx in 0..vertex_count {
        let (x, y) = domain.xy(idx);
        let zw = opts.z_offset + opts.z_scale * field.get(x, y);
        heights.push(zw);
    }

    let mut normals = vec![[0.0_f32; 3]; vertex_count];
    simd::compute_normals_heightmap(
        domain.nx,
        domain.ny,
        domain.step_x,
        domain.step_y,
        &heights,
        &mut normals,
        opts.compute,
    );

    for (idx, &zw) in heights.iter().enumerate() {
        let (x, y) = domain.xy(idx);
        let (xw, yw) = domain.pos(x, y);
        vertices.push([xw, yw, zw]);
    }

    let cell_count = (domain.nx - 1) * (domain.ny - 1);
    let mut indices = Vec::with_capacity(cell_count * 6);
    for y in 0..(domain.ny - 1) {
        for x in 0..(domain.nx - 1) {
            let a = domain.idx(x, y) as u32;
            let b = domain.idx(x + 1, y) as u32;
            let c = domain.idx(x, y + 1) as u32;
            let d = domain.idx(x + 1, y + 1) as u32;

            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    Mesh::new(vertices, normals, indices)
}
