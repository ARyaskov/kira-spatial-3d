use std::io::{self, Write};

/// Float formatting policy for exporters.
#[derive(Clone, Copy, Debug)]
pub struct FloatFmt {
    pub decimals: usize,
}

impl FloatFmt {
    pub const DEFAULT: FloatFmt = FloatFmt { decimals: 6 };
}

/// Format `f32` with fixed decimals. Non-finite → `0`, `-0.0` → `0.0`, decimals clamped to 0..=9.
pub fn fmt_f32(v: f32, fmt: FloatFmt) -> String {
    let prec = sanitize_decimals(fmt.decimals);
    let value = sanitize_f32(v);
    format!("{value:.prec$}")
}

/// Streaming variant of [`fmt_f32`]; avoids the per-value allocation.
#[inline]
pub fn write_fmt_f32<W: Write + ?Sized>(w: &mut W, v: f32, fmt: FloatFmt) -> io::Result<()> {
    let prec = sanitize_decimals(fmt.decimals);
    let value = sanitize_f32(v);
    write!(w, "{value:.prec$}")
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
