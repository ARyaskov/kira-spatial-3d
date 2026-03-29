use crate::Error;

/// Deterministic normalization policy for scalar-to-height mapping.
#[derive(Clone, Copy, Debug)]
pub enum Normalization {
    /// Keep finite values unchanged, map missing to `0.0`.
    None,
    /// Linear normalization to `[0, 1]`, optional post-clamp.
    MinMax { clip: Option<(f32, f32)> },
    /// Robust z-score based on median and MAD, optional post-clamp in z-space.
    RobustZ { clip_z: Option<(f32, f32)> },
    /// Percentile window mapped to `[0, 1]`.
    Percentile { lo: f32, hi: f32 },
}

/// Options wrapper for normalization.
#[derive(Clone, Copy, Debug)]
pub struct NormalizeOptions {
    pub policy: Normalization,
}

/// Validates normalization parameters.
pub fn validate_normalization(policy: &Normalization) -> Result<(), Error> {
    match *policy {
        Normalization::None | Normalization::RobustZ { .. } | Normalization::MinMax { .. } => {}
        Normalization::Percentile { lo, hi } => {
            if !(0.0..=100.0).contains(&lo) || !(0.0..=100.0).contains(&hi) {
                return Err(Error::InvalidNormalization {
                    message: "percentiles must be in [0, 100]",
                });
            }
            if lo >= hi {
                return Err(Error::InvalidNormalization {
                    message: "percentile lo must be < hi",
                });
            }
        }
    }
    Ok(())
}

/// Deterministically normalizes values to a derived height buffer.
///
/// Non-finite values are treated as missing and mapped to exactly `0.0`.
/// Statistics are computed over finite values only.
pub fn normalize(values: &[f32], opts: NormalizeOptions) -> Vec<f32> {
    match opts.policy {
        Normalization::None => values
            .iter()
            .map(|&v| if v.is_finite() { v } else { 0.0 })
            .collect(),
        Normalization::MinMax { clip } => normalize_minmax(values, clip),
        Normalization::RobustZ { clip_z } => normalize_robust_z(values, clip_z),
        Normalization::Percentile { lo, hi } => normalize_percentile(values, lo, hi),
    }
}

fn normalize_minmax(values: &[f32], clip: Option<(f32, f32)>) -> Vec<f32> {
    let (min_v, max_v) = match finite_min_max(values) {
        Some(mm) => mm,
        None => return vec![0.0; values.len()],
    };

    if max_v == min_v {
        return vec![0.0; values.len()];
    }

    let scale = (max_v - min_v).recip();
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return 0.0;
            }
            let n = (v - min_v) * scale;
            clamp_opt(n, clip)
        })
        .collect()
}

fn normalize_robust_z(values: &[f32], clip_z: Option<(f32, f32)>) -> Vec<f32> {
    let mut finite = collect_finite(values);
    if finite.is_empty() {
        return vec![0.0; values.len()];
    }

    finite.sort_by(f32::total_cmp);
    let median = median_sorted(&finite);

    let mut abs_dev = finite
        .iter()
        .map(|v| (v - median).abs())
        .collect::<Vec<f32>>();
    abs_dev.sort_by(f32::total_cmp);
    let mad = median_sorted(&abs_dev);
    if mad == 0.0 {
        return vec![0.0; values.len()];
    }

    let denom = 1.4826_f32 * mad;
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return 0.0;
            }
            let z = (v - median) / denom;
            clamp_opt(z, clip_z)
        })
        .collect()
}

fn normalize_percentile(values: &[f32], lo: f32, hi: f32) -> Vec<f32> {
    let mut finite = collect_finite(values);
    if finite.is_empty() {
        return vec![0.0; values.len()];
    }

    finite.sort_by(f32::total_cmp);
    let n = finite.len();

    let klo = percentile_index(lo, n);
    let khi = percentile_index(hi, n);
    let plo = finite[klo];
    let phi = finite[khi];

    if phi == plo {
        return vec![0.0; values.len()];
    }

    let inv = (phi - plo).recip();
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return 0.0;
            }
            let c = v.clamp(plo, phi);
            (c - plo) * inv
        })
        .collect()
}

#[inline]
fn finite_min_max(values: &[f32]) -> Option<(f32, f32)> {
    let mut it = values.iter().copied().filter(|v| v.is_finite());
    let first = it.next()?;
    let mut min_v = first;
    let mut max_v = first;
    for v in it {
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    Some((min_v, max_v))
}

#[inline]
fn collect_finite(values: &[f32]) -> Vec<f32> {
    values.iter().copied().filter(|v| v.is_finite()).collect()
}

#[inline]
fn median_sorted(sorted: &[f32]) -> f32 {
    sorted[sorted.len() / 2]
}

#[inline]
fn percentile_index(p: f32, n: usize) -> usize {
    ((p / 100.0) * (n.saturating_sub(1) as f32)).floor() as usize
}

#[inline]
fn clamp_opt(v: f32, clip: Option<(f32, f32)>) -> f32 {
    match clip {
        Some((lo, hi)) => v.clamp(lo, hi),
        None => v,
    }
}
