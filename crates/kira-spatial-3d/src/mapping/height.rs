use crate::mapping::normalize::{
    Normalization, NormalizeOptions, normalize, validate_normalization,
};
use crate::{ComputeConfig, Error, ScalarField, simd};

/// Controls how scalar values are interpreted before normalization.
#[derive(Clone, Copy, Debug)]
pub enum HeightMode {
    /// Use finite values as-is.
    Raw,
    /// Use `abs(v)` for finite values.
    Abs,
    /// Same numerical behavior as `Raw`, kept explicit for signed fields (for example, Δf).
    Signed,
}

/// Full deterministic mapping spec from field values to final mesh heights.
#[derive(Clone, Copy, Debug)]
pub struct HeightMapSpec {
    pub mode: HeightMode,
    pub normalization: Normalization,
    pub z_scale: f32,
    pub z_offset: f32,
    pub compute: ComputeConfig,
}

/// Builds a mapped height buffer with length equal to `field.domain.len()`.
///
/// Non-finite inputs are treated as missing and produce `0.0` after normalization.
/// Final output is always affine-mapped: `z = z_offset + z_scale * normed`.
pub fn build_heights(field: &ScalarField<'_>, spec: HeightMapSpec) -> Result<Vec<f32>, Error> {
    validate_normalization(&spec.normalization)?;
    if !spec.z_scale.is_finite() {
        return Err(Error::InvalidHeightSpec {
            message: "z_scale must be finite",
        });
    }
    if !spec.z_offset.is_finite() {
        return Err(Error::InvalidHeightSpec {
            message: "z_offset must be finite",
        });
    }

    field.domain.validate()?;
    if field.values.len() != field.domain.len() {
        return Err(Error::LengthMismatch {
            expected: field.domain.len(),
            got: field.values.len(),
        });
    }

    let mut tmp = Vec::with_capacity(field.values.len());
    for &v in field.values {
        if !v.is_finite() {
            tmp.push(f32::NAN);
            continue;
        }

        let mapped = match spec.mode {
            HeightMode::Raw | HeightMode::Signed => v,
            HeightMode::Abs => v.abs(),
        };
        tmp.push(mapped);
    }

    let normed = normalize(
        &tmp,
        NormalizeOptions {
            policy: spec.normalization,
        },
    );
    let mut out = vec![0.0_f32; normed.len()];
    simd::apply_mode_and_affine(
        &normed,
        HeightMode::Raw,
        spec.z_scale,
        spec.z_offset,
        &mut out,
        spec.compute,
    );
    Ok(out)
}
