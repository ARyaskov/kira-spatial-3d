use crate::mapping::HeightMode;

pub fn compute_normals_heightmap(
    nx: usize,
    ny: usize,
    step_x: f32,
    step_y: f32,
    heights: &[f32],
    out_normals: &mut [[f32; 3]],
) {
    assert_eq!(heights.len(), nx * ny, "heights length must match nx*ny");
    assert_eq!(
        out_normals.len(),
        nx * ny,
        "normals length must match nx*ny"
    );

    let inv_2sx = 1.0 / (2.0 * step_x);
    let inv_2sy = 1.0 / (2.0 * step_y);
    let inv_sx = 1.0 / step_x;
    let inv_sy = 1.0 / step_y;

    for y in 0..ny {
        for x in 0..nx {
            let idx = y * nx + x;

            let dzdx = if x > 0 && x + 1 < nx {
                (heights[y * nx + (x + 1)] - heights[y * nx + (x - 1)]) * inv_2sx
            } else if x + 1 < nx {
                (heights[y * nx + (x + 1)] - heights[idx]) * inv_sx
            } else {
                (heights[idx] - heights[y * nx + (x - 1)]) * inv_sx
            };

            let dzdy = if y > 0 && y + 1 < ny {
                (heights[(y + 1) * nx + x] - heights[(y - 1) * nx + x]) * inv_2sy
            } else if y + 1 < ny {
                (heights[(y + 1) * nx + x] - heights[idx]) * inv_sy
            } else {
                (heights[idx] - heights[(y - 1) * nx + x]) * inv_sy
            };

            out_normals[idx] = normalize([-dzdx, -dzdy, 1.0]);
        }
    }
}

pub fn apply_mode_and_affine(
    input: &[f32],
    mode: HeightMode,
    z_scale: f32,
    z_offset: f32,
    out: &mut [f32],
) {
    assert_eq!(
        input.len(),
        out.len(),
        "input and output lengths must match"
    );

    for (src, dst) in input.iter().copied().zip(out.iter_mut()) {
        let mut v = if src.is_finite() { src } else { 0.0 };
        if matches!(mode, HeightMode::Abs) {
            v = v.abs();
        }
        let mapped = z_offset + z_scale * v;
        *dst = if mapped == 0.0 { 0.0 } else { mapped };
    }
}

#[inline]
fn normalize(v: [f32; 3]) -> [f32; 3] {
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
