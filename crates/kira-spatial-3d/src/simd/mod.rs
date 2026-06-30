use crate::config::{ComputeBackend, ComputeConfig};
use crate::mapping::HeightMode;

pub mod scalar;

#[cfg(target_arch = "aarch64")]
mod aarch64_neon;
#[cfg(target_arch = "x86_64")]
mod x86_avx2;

/// Heightmap vertex normals. Always scalar — gradient stencil too cheap to amortize SIMD overhead.
pub fn compute_normals_heightmap(
    nx: usize,
    ny: usize,
    step_x: f32,
    step_y: f32,
    heights: &[f32],
    out_normals: &mut [[f32; 3]],
) {
    scalar::compute_normals_heightmap(nx, ny, step_x, step_y, heights, out_normals);
}

/// Apply height mode + affine transform in place. Non-finite → `0.0`.
pub fn apply_mode_and_affine(
    input: &[f32],
    mode: HeightMode,
    z_scale: f32,
    z_offset: f32,
    out: &mut [f32],
    cfg: ComputeConfig,
) {
    assert_eq!(
        input.len(),
        out.len(),
        "input and output lengths must match"
    );

    if matches!(cfg.backend, ComputeBackend::Scalar) {
        scalar::apply_mode_and_affine(input, mode, z_scale, z_offset, out);
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 checked at runtime; callee handles tail.
            #[allow(unsafe_code)]
            unsafe {
                x86_avx2::apply_mode_and_affine_avx2(input, mode, z_scale, z_offset, out)
            };
            return;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: NEON checked at runtime; callee handles tail.
            #[allow(unsafe_code)]
            unsafe {
                aarch64_neon::apply_mode_and_affine_neon(input, mode, z_scale, z_offset, out)
            };
            return;
        }
    }

    scalar::apply_mode_and_affine(input, mode, z_scale, z_offset, out);
}

/// Min-max normalize: `out[i] = (in[i] - min) / denom` when finite, else `0.0`.
/// Caller pre-validates `denom != 0` and finite. Bitwise-equal to the scalar path.
pub fn apply_sub_div_finite(
    input: &[f32],
    min: f32,
    denom: f32,
    out: &mut [f32],
    cfg: ComputeConfig,
) {
    assert_eq!(
        input.len(),
        out.len(),
        "input and output lengths must match"
    );

    let inv = denom.recip();

    if matches!(cfg.backend, ComputeBackend::Scalar) {
        scalar::apply_sub_div_finite(input, min, inv, out);
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 checked at runtime; callee handles tail.
            #[allow(unsafe_code)]
            unsafe {
                x86_avx2::apply_sub_div_finite_avx2(input, min, inv, out)
            };
            return;
        }
    }

    scalar::apply_sub_div_finite(input, min, inv, out);
}
