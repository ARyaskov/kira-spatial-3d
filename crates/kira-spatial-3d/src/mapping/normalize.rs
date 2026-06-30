use crate::{ComputeConfig, Error, simd};

/// Normalization policy for scalar-to-height mapping.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Normalization {
    None,
    MinMax {
        clip: Option<(f32, f32)>,
    },
    RobustZ {
        clip_z: Option<(f32, f32)>,
    },
    /// Percentile window mapped to `[0, 1]`. Endpoints use linear interpolation (NumPy default).
    Percentile {
        lo: f32,
        hi: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct NormalizeOptions {
    pub policy: Normalization,
}

pub fn validate_normalization(policy: &Normalization) -> Result<(), Error> {
    match *policy {
        Normalization::None => {}
        Normalization::MinMax { clip } => {
            if let Some((lo, hi)) = clip {
                if !lo.is_finite() || !hi.is_finite() {
                    return Err(Error::InvalidNormalization {
                        message: "minmax clip endpoints must be finite",
                    });
                }
                if lo > hi {
                    return Err(Error::InvalidNormalization {
                        message: "minmax clip lo must be <= hi",
                    });
                }
            }
        }
        Normalization::RobustZ { clip_z } => {
            if let Some((lo, hi)) = clip_z {
                if !lo.is_finite() || !hi.is_finite() {
                    return Err(Error::InvalidNormalization {
                        message: "robust_z clip_z endpoints must be finite",
                    });
                }
                if lo > hi {
                    return Err(Error::InvalidNormalization {
                        message: "robust_z clip_z lo must be <= hi",
                    });
                }
            }
        }
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

/// Normalize values. Non-finite → `0.0`. Stats use finite values only.
pub fn normalize(values: &[f32], opts: NormalizeOptions) -> Vec<f32> {
    normalize_with(values, opts, ComputeConfig::default())
}

/// [`normalize`] with explicit backend selection.
pub fn normalize_with(values: &[f32], opts: NormalizeOptions, cfg: ComputeConfig) -> Vec<f32> {
    match opts.policy {
        Normalization::None => values
            .iter()
            .map(|&v| if v.is_finite() { v } else { 0.0 })
            .collect(),
        Normalization::MinMax { clip } => normalize_minmax(values, clip, cfg),
        Normalization::RobustZ { clip_z } => normalize_robust_z(values, clip_z),
        Normalization::Percentile { lo, hi } => normalize_percentile(values, lo, hi),
    }
}

fn normalize_minmax(values: &[f32], clip: Option<(f32, f32)>, cfg: ComputeConfig) -> Vec<f32> {
    let (min_v, max_v) = match finite_min_max(values) {
        Some(mm) => mm,
        None => return vec![0.0; values.len()],
    };

    if max_v == min_v {
        return vec![0.0; values.len()];
    }

    let denom = max_v - min_v;

    if clip.is_none() {
        let mut out = vec![0.0_f32; values.len()];
        simd::apply_sub_div_finite(values, min_v, denom, &mut out, cfg);
        return out;
    }

    let scale = denom.recip();
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

    let median = partition_median(&mut finite);

    for v in finite.iter_mut() {
        *v = (*v - median).abs();
    }
    let mad = partition_median(&mut finite);
    if mad == 0.0 {
        return vec![0.0; values.len()];
    }

    // 1.4826 = MAD-to-sigma multiplier under a Gaussian model.
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

    let plo = partition_percentile(&mut finite, lo);
    let phi = partition_percentile(&mut finite, hi);

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

/// Median via O(n) partition. Even-length → mean of two middles (NumPy/R convention).
fn partition_median(buf: &mut [f32]) -> f32 {
    let n = buf.len();
    assert!(n > 0, "median requires a non-empty buffer");

    if n % 2 == 1 {
        let mid = n / 2;
        let (_, m, _) = buf.select_nth_unstable_by(mid, f32::total_cmp);
        return *m;
    }

    let hi = n / 2;
    let (lo_part, m_hi, _) = buf.select_nth_unstable_by(hi, f32::total_cmp);
    let high = *m_hi;
    let low = lo_part
        .iter()
        .copied()
        .reduce(|a, b| if a > b { a } else { b })
        .expect("lo_part is non-empty since n >= 2");
    0.5 * (low + high)
}

/// Linear-interpolation percentile (NumPy `interpolation='linear'`). `p` in `[0, 100]`.
fn partition_percentile(buf: &mut [f32], p: f32) -> f32 {
    let n = buf.len();
    assert!(n > 0, "percentile requires a non-empty buffer");
    if n == 1 {
        return buf[0];
    }

    let rank = (p / 100.0) * (n - 1) as f32;
    let k_lo = rank.floor() as usize;
    let k_hi = (k_lo + 1).min(n - 1);
    let frac = rank - k_lo as f32;

    let (_, m_lo, hi_part) = buf.select_nth_unstable_by(k_lo, f32::total_cmp);
    let low = *m_lo;
    if k_lo == k_hi {
        return low;
    }
    let high = hi_part
        .iter()
        .copied()
        .reduce(|a, b| if a < b { a } else { b })
        .expect("hi_part is non-empty since k_lo < n - 1");

    low + frac * (high - low)
}

#[inline]
fn clamp_opt(v: f32, clip: Option<(f32, f32)>) -> f32 {
    match clip {
        Some((lo, hi)) => v.clamp(lo, hi),
        None => v,
    }
}
