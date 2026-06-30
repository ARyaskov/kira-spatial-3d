use crate::mapping::normalize::{
    Normalization, NormalizeOptions, normalize_with, validate_normalization,
};
use crate::{ComputeConfig, Error, ScalarField, simd};

/// How scalar values are interpreted before normalization.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum HeightMode {
    Raw,
    Abs,
    Signed,
}

/// Mapping spec from field values to final mesh heights.
#[derive(Clone, Copy, Debug)]
pub struct HeightMapSpec {
    pub mode: HeightMode,
    pub normalization: Normalization,
    pub z_scale: f32,
    pub z_offset: f32,
    pub compute: ComputeConfig,
}

/// Build heights with `len == field.domain.len()`. Non-finite → 0, then `z = z_offset + z_scale * normed`.
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

    let normed = normalize_with(
        &tmp,
        NormalizeOptions {
            policy: spec.normalization,
        },
        spec.compute,
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
