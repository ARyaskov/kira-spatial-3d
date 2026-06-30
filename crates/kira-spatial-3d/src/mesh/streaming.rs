use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::export::buffer::{BufferOptions, K3dMeshMeta};
use crate::mesh::heightmap::HeightmapOptions;
use crate::{Error, ScalarField};

/// Stream a heightmap mesh to `<prefix>.k3d.bin` + `<prefix>.k3d.json`.
/// Bitwise-identical to building the full [`crate::Mesh`] and writing it; only the memory profile differs.
pub fn build_heightmap_mesh_to_k3d<P: AsRef<Path>>(
    field: &ScalarField<'_>,
    opts: HeightmapOptions,
    out_prefix: P,
    buf_opts: BufferOptions,
) -> Result<(PathBuf, PathBuf), Error> {
    field.domain.validate()?;
    if field.values.len() != field.domain.len() {
        return Err(Error::LengthMismatch {
            expected: field.domain.len(),
            got: field.values.len(),
        });
    }

    let domain = field.domain;
    let nx = domain.nx;
    let ny = domain.ny;
    let vertex_count = nx * ny;
    if vertex_count > u32::MAX as usize {
        return Err(Error::IndexOverflow { vertex_count });
    }

    let cell_count = (nx - 1) * (ny - 1);
    let index_count = cell_count * 6;
    let vertex_count_u32 = u32::try_from(vertex_count).map_err(|_| Error::InvalidExportSpec {
        message: "vertex_count exceeds u32",
    })?;
    let index_count_u32 = u32::try_from(index_count).map_err(|_| Error::InvalidExportSpec {
        message: "index_count exceeds u32",
    })?;

    let positions_bytes = vertex_count as u64 * 3 * 4;
    let normals_bytes = if buf_opts.write_normals {
        vertex_count as u64 * 3 * 4
    } else {
        0
    };
    let indices_bytes = index_count as u64 * 4;

    let positions_offset = 0_u64;
    let normals_offset = positions_offset + positions_bytes;
    let indices_offset = normals_offset + normals_bytes;

    let meta = K3dMeshMeta {
        version: "k3d-mesh-buffer/v1".to_string(),
        vertex_count: vertex_count_u32,
        index_count: index_count_u32,
        positions_offset,
        normals_offset,
        indices_offset,
        positions_bytes,
        normals_bytes,
        indices_bytes,
        index_format: "u32".to_string(),
    };

    let out_prefix = out_prefix.as_ref();
    if let Some(parent) = out_prefix.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::IoContext {
            path: parent.to_path_buf(),
            operation: "creating parent dir for",
            source,
        })?;
    }
    let bin_path = PathBuf::from(format!("{}.k3d.bin", out_prefix.display()));
    let json_path = PathBuf::from(format!("{}.k3d.json", out_prefix.display()));

    let mut heights = vec![0.0_f32; vertex_count];
    for y in 0..ny {
        let row = y * nx;
        for x in 0..nx {
            let v = field.values[row + x];
            heights[row + x] = opts.z_offset + opts.z_scale * v;
        }
    }

    {
        let file = File::create(&bin_path).map_err(|source| Error::IoContext {
            path: bin_path.clone(),
            operation: "creating",
            source,
        })?;
        let mut w = BufWriter::new(file);

        let mut row_buf = vec![[0.0_f32; 3]; nx];
        for y in 0..ny {
            let yw = domain.origin_y + y as f32 * domain.step_y;
            let row = y * nx;
            for x in 0..nx {
                let xw = domain.origin_x + x as f32 * domain.step_x;
                row_buf[x] = [xw, yw, heights[row + x]];
            }
            write_f32_block(&mut w, &row_buf)?;
        }

        if buf_opts.write_normals {
            for y in 0..ny {
                fill_normals_row(
                    nx,
                    ny,
                    domain.step_x,
                    domain.step_y,
                    &heights,
                    y,
                    &mut row_buf,
                );
                write_f32_block(&mut w, &row_buf)?;
            }
        }

        if nx >= 2 && ny >= 2 {
            let cells_per_row = nx - 1;
            let mut idx_buf = vec![0_u32; cells_per_row * 6];
            let nx_u32 = nx as u32;
            for y in 0..(ny - 1) {
                let row = y as u32 * nx_u32;
                let next_row = row + nx_u32;
                for x in 0..cells_per_row {
                    let xu = x as u32;
                    let a = row + xu;
                    let b = a + 1;
                    let c = next_row + xu;
                    let d = c + 1;
                    let base = x * 6;
                    idx_buf[base] = a;
                    idx_buf[base + 1] = b;
                    idx_buf[base + 2] = c;
                    idx_buf[base + 3] = b;
                    idx_buf[base + 4] = d;
                    idx_buf[base + 5] = c;
                }
                write_u32_block(&mut w, &idx_buf)?;
            }
        }
        w.flush().map_err(|source| Error::IoContext {
            path: bin_path.clone(),
            operation: "flushing",
            source,
        })?;
    }

    {
        let file = File::create(&json_path).map_err(|source| Error::IoContext {
            path: json_path.clone(),
            operation: "creating",
            source,
        })?;
        let w = BufWriter::new(file);
        serde_json::to_writer(w, &meta)?;
    }

    Ok((bin_path, json_path))
}

#[inline]
fn write_f32_block<W: Write>(w: &mut W, row: &[[f32; 3]]) -> Result<(), Error> {
    #[cfg(target_endian = "little")]
    {
        w.write_all(bytemuck::cast_slice::<[f32; 3], u8>(row))?;
    }
    #[cfg(target_endian = "big")]
    {
        for v in row {
            for c in v {
                w.write_all(&c.to_le_bytes())?;
            }
        }
    }
    Ok(())
}

#[inline]
fn write_u32_block<W: Write>(w: &mut W, row: &[u32]) -> Result<(), Error> {
    #[cfg(target_endian = "little")]
    {
        w.write_all(bytemuck::cast_slice::<u32, u8>(row))?;
    }
    #[cfg(target_endian = "big")]
    {
        for &idx in row {
            w.write_all(&idx.to_le_bytes())?;
        }
    }
    Ok(())
}

fn fill_normals_row(
    nx: usize,
    ny: usize,
    step_x: f32,
    step_y: f32,
    heights: &[f32],
    y: usize,
    out: &mut [[f32; 3]],
) {
    debug_assert!(nx >= 2 && ny >= 2);
    debug_assert!(y < ny);
    debug_assert!(out.len() == nx);

    let inv_2sx = 1.0 / (2.0 * step_x);
    let inv_2sy = 1.0 / (2.0 * step_y);
    let inv_sx = 1.0 / step_x;
    let inv_sy = 1.0 / step_y;
    let row = y * nx;

    for x in 0..nx {
        let idx = row + x;

        let dzdx = if x > 0 && x + 1 < nx {
            (heights[row + (x + 1)] - heights[row + (x - 1)]) * inv_2sx
        } else if x + 1 < nx {
            (heights[row + (x + 1)] - heights[idx]) * inv_sx
        } else {
            (heights[idx] - heights[row + (x - 1)]) * inv_sx
        };

        let dzdy = if y > 0 && y + 1 < ny {
            (heights[(y + 1) * nx + x] - heights[(y - 1) * nx + x]) * inv_2sy
        } else if y + 1 < ny {
            (heights[(y + 1) * nx + x] - heights[idx]) * inv_sy
        } else {
            (heights[idx] - heights[(y - 1) * nx + x]) * inv_sy
        };

        out[x] = normalize_vec3([-dzdx, -dzdy, 1.0]);
    }
}

#[inline]
fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if len2 <= 0.0 || !len2.is_finite() {
        return [0.0, 0.0, 1.0];
    }
    let inv_len = len2.sqrt().recip();
    if !inv_len.is_finite() {
        return [0.0, 0.0, 1.0];
    }
    [v[0] * inv_len, v[1] * inv_len, v[2] * inv_len]
}
