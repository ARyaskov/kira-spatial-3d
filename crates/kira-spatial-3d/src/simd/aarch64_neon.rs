#![cfg(target_arch = "aarch64")]

use core::arch::aarch64::*;

use crate::mapping::HeightMode;

#[target_feature(enable = "neon")]
pub unsafe fn apply_mode_and_affine_neon(
    input: &[f32],
    mode: HeightMode,
    z_scale: f32,
    z_offset: f32,
    out: &mut [f32],
) {
    debug_assert_eq!(input.len(), out.len());

    let mut i = 0usize;
    let n = input.len();
    let lanes = 4usize;

    let zero = vdupq_n_f32(0.0);
    let scale = vdupq_n_f32(z_scale);
    let offset = vdupq_n_f32(z_offset);
    let max_finite = vdupq_n_f32(f32::MAX);

    while i + lanes <= n {
        let v = {
            // SAFETY: guarded by bounds check above, pointer is valid for 4 f32.
            unsafe { vld1q_f32(input.as_ptr().add(i)) }
        };
        let abs_v = vabsq_f32(v);
        let finite_mask = vcleq_f32(abs_v, max_finite);

        let work = if matches!(mode, HeightMode::Abs) {
            abs_v
        } else {
            v
        };
        let mapped = vfmaq_f32(offset, work, scale);
        let mut cleaned = vbslq_f32(finite_mask, mapped, zero);

        let zero_mask = vceqq_f32(cleaned, zero);
        cleaned = vbslq_f32(zero_mask, zero, cleaned);

        // SAFETY: guarded by bounds check above, pointer is valid for 4 f32.
        unsafe { vst1q_f32(out.as_mut_ptr().add(i), cleaned) };
        i += lanes;
    }

    if i < n {
        super::scalar::apply_mode_and_affine(&input[i..], mode, z_scale, z_offset, &mut out[i..]);
    }
}
