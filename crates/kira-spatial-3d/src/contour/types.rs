/// A single 2.5D contour line segment in world coordinates.
///
/// `z` coordinates are flat at the contour level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContourSegment {
    pub p0: [f32; 3],
    pub p1: [f32; 3],
}

/// Contour result for one iso-level.
///
/// # Determinism
/// Segments are appended in strict scanline order of source cells (`y` outer, `x` inner),
/// and deterministic within-cell edge order.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourSet {
    pub level: f32,
    pub segments: Vec<ContourSegment>,
}

/// Multi-level contour collection.
///
/// Levels are preserved in the same order as requested by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiContour {
    pub contours: Vec<ContourSet>,
}
