use crate::contour::types::{ContourSegment, ContourSet, MultiContour};
use crate::{Error, ScalarField};

/// Per-call diagnostics. `skipped_cells` counts cells dropped for non-finite corners.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContourStats {
    pub skipped_cells: usize,
}

/// Extract iso-contour segments via deterministic Marching Squares.
pub fn extract_contours(field: &ScalarField<'_>, levels: &[f32]) -> Result<MultiContour, Error> {
    let (multi, _) = extract_contours_with_stats(field, levels)?;
    Ok(multi)
}

/// Like [`extract_contours`] but also returns [`ContourStats`].
pub fn extract_contours_with_stats(
    field: &ScalarField<'_>,
    levels: &[f32],
) -> Result<(MultiContour, ContourStats), Error> {
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
    let mut stats = ContourStats::default();
    let cap_hint = (d.nx + d.ny).saturating_mul(2);
    let nx = d.nx;
    let values = field.values;
    for &level in levels {
        let mut segments = Vec::with_capacity(cap_hint);
        for y in 0..(d.ny - 1) {
            let row0 = y * nx;
            let row1 = row0 + nx;
            for x in 0..(d.nx - 1) {
                let v0 = values[row0 + x];
                let v1 = values[row0 + x + 1];
                let v2 = values[row1 + x + 1];
                let v3 = values[row1 + x];

                if !(v0.is_finite() && v1.is_finite() && v2.is_finite() && v3.is_finite()) {
                    stats.skipped_cells += 1;
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

                let mut push = |seg: ContourSegment| segments.push(seg);
                emit_cell_segments(&mut push, x, y, [v0, v1, v2, v3], case_index, level, field);
            }
        }
        contours.push(ContourSet { level, segments });
    }
    Ok((MultiContour { contours }, stats))
}

/// Streaming variant of [`extract_contours`] — emits each segment via `sink`.
pub fn for_each_contour_segment<S>(
    field: &ScalarField<'_>,
    levels: &[f32],
    mut sink: S,
) -> Result<ContourStats, Error>
where
    S: FnMut(f32, ContourSegment),
{
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
    let mut stats = ContourStats::default();
    let nx = d.nx;
    let values = field.values;
    for &level in levels {
        for y in 0..(d.ny - 1) {
            let row0 = y * nx;
            let row1 = row0 + nx;
            for x in 0..(d.nx - 1) {
                let v0 = values[row0 + x];
                let v1 = values[row0 + x + 1];
                let v2 = values[row1 + x + 1];
                let v3 = values[row1 + x];

                if !(v0.is_finite() && v1.is_finite() && v2.is_finite() && v3.is_finite()) {
                    stats.skipped_cells += 1;
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

                let mut adapter = |seg: ContourSegment| sink(level, seg);
                emit_cell_segments(
                    &mut adapter,
                    x,
                    y,
                    [v0, v1, v2, v3],
                    case_index,
                    level,
                    field,
                );
            }
        }
    }
    Ok(stats)
}

/// Single-level ridge/boundary contour extraction.
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

fn emit_cell_segments<F>(
    sink: &mut F,
    x: usize,
    y: usize,
    values: [f32; 4],
    case_index: u8,
    level: f32,
    field: &ScalarField<'_>,
) where
    F: FnMut(ContourSegment),
{
    let mut emit = |edge_a: u8, edge_b: u8| {
        let p0 = edge_intersection(x, y, values, level, field, edge_a);
        let p1 = edge_intersection(x, y, values, level, field, edge_b);
        sink(ContourSegment { p0, p1 });
    };
    match case_index {
        0 | 15 => {}
        1 => emit(3, 0),
        2 => emit(0, 1),
        3 => emit(3, 1),
        4 => emit(1, 2),
        5 => {
            let center = (values[0] + values[1] + values[2] + values[3]) * 0.25;
            if center >= level {
                emit(0, 1);
                emit(2, 3);
            } else {
                emit(3, 0);
                emit(1, 2);
            }
        }
        6 => emit(0, 2),
        7 => emit(2, 3),
        8 => emit(2, 3),
        9 => emit(0, 2),
        10 => {
            let center = (values[0] + values[1] + values[2] + values[3]) * 0.25;
            if center >= level {
                emit(3, 0);
                emit(1, 2);
            } else {
                emit(0, 1);
                emit(2, 3);
            }
        }
        11 => emit(1, 2),
        12 => emit(1, 3),
        13 => emit(0, 1),
        14 => emit(3, 0),
        _ => unreachable!("case index must be in 0..=15"),
    }
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
    let ((ax, ay), va, (bx, by), vb) = match edge {
        0 => ((x, y), values[0], (x + 1, y), values[1]),
        1 => ((x + 1, y), values[1], (x + 1, y + 1), values[2]),
        2 => ((x + 1, y + 1), values[2], (x, y + 1), values[3]),
        3 => ((x, y + 1), values[3], (x, y), values[0]),
        _ => unreachable!("edge index must be in 0..=3"),
    };

    // Degenerate-edge tie-break: midpoint. Zero-length segments are dropped downstream.
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
