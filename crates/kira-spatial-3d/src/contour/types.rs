/// 2.5D contour line segment in world coordinates. `z` is the contour level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContourSegment {
    pub p0: [f32; 3],
    pub p1: [f32; 3],
}

/// Contour result for one iso-level. Segments are in scanline order.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourSet {
    pub level: f32,
    pub segments: Vec<ContourSegment>,
}

impl ContourSet {
    #[inline]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Multi-level contour collection. Levels preserve caller-requested order.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiContour {
    pub contours: Vec<ContourSet>,
}

impl MultiContour {
    #[inline]
    pub fn len(&self) -> usize {
        self.contours.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.contours.is_empty()
    }

    pub fn find_level(&self, level: f32) -> Option<&ContourSet> {
        self.contours.iter().find(|c| c.level == level)
    }
}
