/// Deterministic float formatting policy for all exporters.
#[derive(Clone, Copy, Debug)]
pub struct FloatFmt {
    pub decimals: usize,
}

impl FloatFmt {
    pub const DEFAULT: FloatFmt = FloatFmt { decimals: 6 };
}

/// Formats `f32` using fixed decimals with deterministic normalization.
///
/// Rules:
/// - non-finite values serialize as `0.000000`-style zeros
/// - `-0.0` is normalized to `0.0`
/// - precision is clamped to `0..=9`, otherwise default precision is used
pub fn fmt_f32(v: f32, fmt: FloatFmt) -> String {
    let prec = sanitize_decimals(fmt.decimals);
    let value = if v.is_finite() { v } else { 0.0 };
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.prec$}", prec = prec)
}

#[inline]
pub(crate) fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() {
        if v == 0.0 { 0.0 } else { v }
    } else {
        0.0
    }
}

#[inline]
fn sanitize_decimals(decimals: usize) -> usize {
    if decimals <= 9 {
        decimals
    } else {
        FloatFmt::DEFAULT.decimals
    }
}
