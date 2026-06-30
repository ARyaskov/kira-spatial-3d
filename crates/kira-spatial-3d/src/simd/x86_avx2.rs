#![cfg(target_arch = "x86_64")]
#![allow(unsafe_code)]

use core::arch::x86_64::*;

use crate::mapping::HeightMode;

#[target_feature(enable = "avx2")]
pub unsafe fn apply_mode_and_affine_avx2(
    input: &[f32],
    mode: HeightMode,
    z_scale: f32,
    z_offset: f32,
    out: &mut [f32],
) {
    debug_assert_eq!(input.len(), out.len());

    let mut i = 0usize;
    let n = input.len();
    let lanes = 8usize;

    let zero = _mm256_set1_ps(0.0);
    let scale = _mm256_set1_ps(z_scale);
    let offset = _mm256_set1_ps(z_offset);
    let max_finite = _mm256_set1_ps(f32::MAX);
    let sign_mask = _mm256_set1_ps(-0.0);

    while i + lanes <= n {
        let v = {
            // SAFETY: guarded by bounds check above, pointer is valid for 8 f32.
            unsafe { _mm256_loadu_ps(input.as_ptr().add(i)) }
        };
        let abs_v = _mm256_andnot_ps(sign_mask, v);
        let ordered = _mm256_cmp_ps(v, v, _CMP_ORD_Q);
        let finite = _mm256_and_ps(ordered, _mm256_cmp_ps(abs_v, max_finite, _CMP_LE_OQ));

        let mut work = v;
        if matches!(mode, HeightMode::Abs) {
            work = abs_v;
        }
        let mapped = _mm256_add_ps(offset, _mm256_mul_ps(scale, work));
        let cleaned = _mm256_blendv_ps(zero, mapped, finite);
        // Force ±0 → +0 to match the scalar `if mapped == 0.0` clause.
        let cleaned = _mm256_blendv_ps(cleaned, zero, _mm256_cmp_ps(cleaned, zero, _CMP_EQ_OQ));

        // SAFETY: guarded by bounds check above, pointer is valid for 8 f32.
        unsafe { _mm256_storeu_ps(out.as_mut_ptr().add(i), cleaned) };
        i += lanes;
    }

    if i < n {
        super::scalar::apply_mode_and_affine(&input[i..], mode, z_scale, z_offset, &mut out[i..]);
    }
}

/// AVX2 variant of `apply_sub_div_finite`. `inv` pre-computed so output is bitwise-equal to scalar.
#[target_feature(enable = "avx2")]
pub unsafe fn apply_sub_div_finite_avx2(input: &[f32], min: f32, inv: f32, out: &mut [f32]) {
    debug_assert_eq!(input.len(), out.len());

    let mut i = 0usize;
    let n = input.len();
    let lanes = 8usize;

    let zero = _mm256_set1_ps(0.0);
    let min_v = _mm256_set1_ps(min);
    let inv_v = _mm256_set1_ps(inv);
    let max_finite = _mm256_set1_ps(f32::MAX);
    let sign_mask = _mm256_set1_ps(-0.0);

    while i + lanes <= n {
        let v = {
            // SAFETY: guarded by bounds check above, pointer is valid for 8 f32.
            unsafe { _mm256_loadu_ps(input.as_ptr().add(i)) }
        };
        let abs_v = _mm256_andnot_ps(sign_mask, v);
        let ordered = _mm256_cmp_ps(v, v, _CMP_ORD_Q);
        let finite = _mm256_and_ps(ordered, _mm256_cmp_ps(abs_v, max_finite, _CMP_LE_OQ));

        let centered = _mm256_sub_ps(v, min_v);
        let scaled = _mm256_mul_ps(centered, inv_v);
        let cleaned = _mm256_blendv_ps(zero, scaled, finite);

        // SAFETY: guarded by bounds check above, pointer is valid for 8 f32.
        unsafe { _mm256_storeu_ps(out.as_mut_ptr().add(i), cleaned) };
        i += lanes;
    }

    if i < n {
        super::scalar::apply_sub_div_finite(&input[i..], min, inv, &mut out[i..]);
    }
}
