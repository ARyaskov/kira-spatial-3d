pub mod marching_squares;
pub mod stitch;
pub mod types;

pub use marching_squares::{
    ContourStats, extract_contours, extract_contours_with_stats, extract_ridge_contours,
    for_each_contour_segment,
};
pub use stitch::{Polyline, PolylineSet, QKey, Quantize, StitchOptions, qkey, stitch_contours};
pub use types::{ContourSegment, ContourSet, MultiContour};

use crate::{Error, ScalarField};

/// Extract a single iso-level and stitch its segments into polylines.
pub fn extract_ridge_polylines(
    field: &ScalarField<'_>,
    threshold: f32,
    opts: StitchOptions,
) -> Result<PolylineSet, Error> {
    let contours = extract_ridge_contours(field, threshold)?;
    stitch_contours(&contours, opts)
}
