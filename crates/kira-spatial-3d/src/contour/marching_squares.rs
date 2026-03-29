use crate::contour::types::{ContourSegment, ContourSet, MultiContour};
use crate::{Error, ScalarField};

/// Extracts iso-contour segments using deterministic Marching Squares.
///
/// Corner order per cell is:
/// - `v0 = (x, y)` (bottom-left)
/// - `v1 = (x + 1, y)` (bottom-right)
/// - `v2 = (x + 1, y + 1)` (top-right)
/// - `v3 = (x, y + 1)` (top-left)
///
/// Case index bits use `>= level`:
/// `bit0=v0`, `bit1=v1`, `bit2=v2`, `bit3=v3`.
///
/// Ambiguous cases `5` and `10` use center tie-break:
/// `center = (v0 + v1 + v2 + v3) * 0.25`.
/// - if `center >= level`: connect one diagonal pairing
/// - else: connect the alternate pairing.
pub fn extract_contours(field: &ScalarField<'_>, levels: &[f32]) -> Result<MultiContour, Error> {
    if levels.is_empty() {
        return Err(Error::InvalidContourSpec {
            message: "levels must not be empty",
        });
    }

    field.domain.validate()?;
    if field.values.len() != field.domain.len() {
        return Err(Error::LengthMismatch {
            expected: field.domain.len(),
            got: field.values.len(),
        });
    }

    for &level in levels {
        if !level.is_finite() {
            return Err(Error::InvalidContourSpec {
                message: "levels must be finite",
            });
        }
    }

    let d = field.domain;
    let mut contours = Vec::with_capacity(levels.len());
    for &level in levels {
        let mut segments = Vec::with_capacity(d.nx * d.ny);
        for y in 0..(d.ny - 1) {
            for x in 0..(d.nx - 1) {
                let v0 = field.get(x, y);
                let v1 = field.get(x + 1, y);
                let v2 = field.get(x + 1, y + 1);
                let v3 = field.get(x, y + 1);

                if !(v0.is_finite() && v1.is_finite() && v2.is_finite() && v3.is_finite()) {
                    continue;
                }

                let mut case_index = 0_u8;
                if v0 >= level {
                    case_index |= 1;
                }
                if v1 >= level {
                    case_index |= 1 << 1;
                }
                if v2 >= level {
                    case_index |= 1 << 2;
                }
                if v3 >= level {
                    case_index |= 1 << 3;
                }

                emit_cell_segments(
                    &mut segments,
                    x,
                    y,
                    [v0, v1, v2, v3],
                    case_index,
                    level,
                    field,
                );
            }
        }
        contours.push(ContourSet { level, segments });
    }
    Ok(MultiContour { contours })
}

/// Convenience helper for single-level ridge/boundary contour extraction.
pub fn extract_ridge_contours(
    field: &ScalarField<'_>,
    threshold: f32,
) -> Result<ContourSet, Error> {
    let multi = extract_contours(field, &[threshold])?;
    debug_assert_eq!(multi.contours.len(), 1);
    Ok(multi
        .contours
        .into_iter()
        .next()
        .expect("single contour set"))
}

fn emit_cell_segments(
    out: &mut Vec<ContourSegment>,
    x: usize,
    y: usize,
    values: [f32; 4],
    case_index: u8,
    level: f32,
    field: &ScalarField<'_>,
) {
    match case_index {
        0 | 15 => {}
        1 => push_segment(out, x, y, values, level, field, 3, 0),
        2 => push_segment(out, x, y, values, level, field, 0, 1),
        3 => push_segment(out, x, y, values, level, field, 3, 1),
        4 => push_segment(out, x, y, values, level, field, 1, 2),
        5 => {
            let center = (values[0] + values[1] + values[2] + values[3]) * 0.25;
            if center >= level {
                // High-valued center: connect around low corners.
                push_segment(out, x, y, values, level, field, 0, 1);
                push_segment(out, x, y, values, level, field, 2, 3);
            } else {
                // Low-valued center: connect around high corners.
                push_segment(out, x, y, values, level, field, 3, 0);
                push_segment(out, x, y, values, level, field, 1, 2);
            }
        }
        6 => push_segment(out, x, y, values, level, field, 0, 2),
        7 => push_segment(out, x, y, values, level, field, 2, 3),
        8 => push_segment(out, x, y, values, level, field, 2, 3),
        9 => push_segment(out, x, y, values, level, field, 0, 2),
        10 => {
            let center = (values[0] + values[1] + values[2] + values[3]) * 0.25;
            if center >= level {
                // High-valued center: connect around low corners.
                push_segment(out, x, y, values, level, field, 3, 0);
                push_segment(out, x, y, values, level, field, 1, 2);
            } else {
                // Low-valued center: connect around high corners.
                push_segment(out, x, y, values, level, field, 0, 1);
                push_segment(out, x, y, values, level, field, 2, 3);
            }
        }
        11 => push_segment(out, x, y, values, level, field, 1, 2),
        12 => push_segment(out, x, y, values, level, field, 1, 3),
        13 => push_segment(out, x, y, values, level, field, 0, 1),
        14 => push_segment(out, x, y, values, level, field, 3, 0),
        _ => unreachable!("case index must be in 0..=15"),
    }
}

#[inline]
fn push_segment(
    out: &mut Vec<ContourSegment>,
    x: usize,
    y: usize,
    values: [f32; 4],
    level: f32,
    field: &ScalarField<'_>,
    edge_a: u8,
    edge_b: u8,
) {
    let p0 = edge_intersection(x, y, values, level, field, edge_a);
    let p1 = edge_intersection(x, y, values, level, field, edge_b);
    out.push(ContourSegment { p0, p1 });
}

#[inline]
fn edge_intersection(
    x: usize,
    y: usize,
    values: [f32; 4],
    level: f32,
    field: &ScalarField<'_>,
    edge: u8,
) -> [f32; 3] {
    // Edges: 0=v0-v1, 1=v1-v2, 2=v2-v3, 3=v3-v0.
    let ((ax, ay), va, (bx, by), vb) = match edge {
        0 => ((x, y), values[0], (x + 1, y), values[1]),
        1 => ((x + 1, y), values[1], (x + 1, y + 1), values[2]),
        2 => ((x + 1, y + 1), values[2], (x, y + 1), values[3]),
        3 => ((x, y + 1), values[3], (x, y), values[0]),
        _ => unreachable!("edge index must be in 0..=3"),
    };

    let t = if vb == va {
        0.5
    } else {
        (level - va) / (vb - va)
    };
    let (axw, ayw) = field.domain.pos(ax, ay);
    let (bxw, byw) = field.domain.pos(bx, by);
    let xw = axw + t * (bxw - axw);
    let yw = ayw + t * (byw - ayw);
    [xw, yw, level]
}
