use crate::config::{ComputeBackend, ComputeConfig};
use crate::mapping::HeightMode;

pub mod scalar;

#[cfg(target_arch = "aarch64")]
mod aarch64_neon;
#[cfg(target_arch = "x86_64")]
mod x86_avx2;

/// Computes heightmap vertex normals with backend dispatch.
///
/// Determinism model:
/// - Normals currently use the scalar reference path for exact reproducibility.
/// - Results are deterministic run-to-run for a given binary/target.
pub fn compute_normals_heightmap(
    nx: usize,
    ny: usize,
    step_x: f32,
    step_y: f32,
    heights: &[f32],
    out_normals: &mut [[f32; 3]],
    _cfg: ComputeConfig,
) {
    scalar::compute_normals_heightmap(nx, ny, step_x, step_y, heights, out_normals);
}

/// Applies height mode and affine transform without allocations.
///
/// Non-finite inputs deterministically map to `0.0`.
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
            // SAFETY: AVX2 is checked at runtime and the function bounds-checks tails.
            unsafe { x86_avx2::apply_mode_and_affine_avx2(input, mode, z_scale, z_offset, out) };
            return;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: NEON is checked at runtime and the function bounds-checks tails.
            unsafe {
                aarch64_neon::apply_mode_and_affine_neon(input, mode, z_scale, z_offset, out)
            };
            return;
        }
    }

    scalar::apply_mode_and_affine(input, mode, z_scale, z_offset, out);
}
