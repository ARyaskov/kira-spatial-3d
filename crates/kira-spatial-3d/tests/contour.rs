//! Marching-squares extraction and polyline stitching.

use kira_spatial_3d::{
    ContourSegment, ContourSet, Quantize, ScalarField, SpatialDomain, StitchOptions,
    extract_contours, stitch_contours,
};

#[test]
fn simple_slope_has_expected_segments() {
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [
        0.0_f32, 1.0, 2.0, //
        1.0, 2.0, 3.0, //
        2.0, 3.0, 4.0,
    ];
    let field = ScalarField::new(domain, &values).expect("field");

    let out = extract_contours(&field, &[1.5]).expect("contours");
    let segments = &out.contours[0].segments;
    assert_eq!(segments.len(), 3);

    assert_eq!(segments[0].p0, [1.0, 0.5, 1.5]);
    assert_eq!(segments[0].p1, [0.5, 1.0, 1.5]);
    assert_eq!(segments[1].p0, [1.0, 0.5, 1.5]);
    assert_eq!(segments[1].p1, [1.5, 0.0, 1.5]);
    assert_eq!(segments[2].p0, [0.0, 1.5, 1.5]);
    assert_eq!(segments[2].p1, [0.5, 1.0, 1.5]);
}

#[test]
fn ambiguous_case_5_uses_deterministic_center_rule() {
    let domain = SpatialDomain::new(2, 2, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [1.0_f32, 0.0, 0.0, 1.0];
    let field = ScalarField::new(domain, &values).expect("field");

    let out = extract_contours(&field, &[0.5]).expect("contours");
    let segments = &out.contours[0].segments;
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].p0, [0.5, 0.0, 0.5]);
    assert_eq!(segments[0].p1, [1.0, 0.5, 0.5]);
    assert_eq!(segments[1].p0, [0.5, 1.0, 0.5]);
    assert_eq!(segments[1].p1, [0.0, 0.5, 0.5]);
}

#[test]
fn repeated_extraction_is_bitwise_identical() {
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [
        0.0_f32, 1.0, 2.0, //
        2.0, 1.0, 0.0, //
        1.0, 2.0, 3.0,
    ];
    let field = ScalarField::new(domain, &values).expect("field");

    let a = extract_contours(&field, &[1.0, 1.5]).expect("first");
    let b = extract_contours(&field, &[1.0, 1.5]).expect("second");
    assert_eq!(a, b);
}

#[test]
fn stitch_open_path_produces_single_ordered_polyline() {
    let set = ContourSet {
        level: 1.0,
        segments: vec![
            ContourSegment {
                p0: [2.0, 0.0, 1.0],
                p1: [3.0, 0.0, 1.0],
            },
            ContourSegment {
                p0: [1.0, 0.0, 1.0],
                p1: [2.0, 0.0, 1.0],
            },
            ContourSegment {
                p0: [0.0, 0.0, 1.0],
                p1: [1.0, 0.0, 1.0],
            },
        ],
    };

    let out = stitch_contours(
        &set,
        StitchOptions {
            quantize: Quantize { grid: 1e-3 },
        },
    )
    .expect("stitch");

    assert_eq!(out.polylines.len(), 1);
    let p = &out.polylines[0];
    assert!(!p.is_closed);
    assert_eq!(p.points.len(), 4);
    assert_eq!(p.points[0], [0.0, 0.0]);
    assert_eq!(p.points[3], [3.0, 0.0]);
    assert_eq!(p.point_3d(0), [0.0, 0.0, 1.0]);
    assert_eq!(p.point_3d(3), [3.0, 0.0, 1.0]);
}

#[test]
fn stitch_loop_is_closed_and_canonicalized() {
    let set = ContourSet {
        level: 2.0,
        segments: vec![
            ContourSegment {
                p0: [1.0, 0.0, 2.0],
                p1: [1.0, 1.0, 2.0],
            },
            ContourSegment {
                p0: [0.0, 1.0, 2.0],
                p1: [0.0, 0.0, 2.0],
            },
            ContourSegment {
                p0: [0.0, 0.0, 2.0],
                p1: [1.0, 0.0, 2.0],
            },
            ContourSegment {
                p0: [1.0, 1.0, 2.0],
                p1: [0.0, 1.0, 2.0],
            },
        ],
    };

    let out = stitch_contours(
        &set,
        StitchOptions {
            quantize: Quantize { grid: 1e-3 },
        },
    )
    .expect("stitch");

    assert_eq!(out.polylines.len(), 1);
    let p = &out.polylines[0];
    assert!(p.is_closed);
    assert_eq!(p.points.len(), 4);
    assert_eq!(p.points[0], [0.0, 0.0]);
    assert_eq!(p.points[1], [0.0, 1.0]);
    assert_eq!(p.point_3d(0), [0.0, 0.0, 2.0]);
    assert_eq!(p.point_3d(1), [0.0, 1.0, 2.0]);
}

#[test]
fn quantization_stitches_near_equal_endpoints() {
    let set = ContourSet {
        level: 0.5,
        segments: vec![
            ContourSegment {
                p0: [0.0, 0.0, 0.5],
                p1: [1.000_000_1, 0.0, 0.5],
            },
            ContourSegment {
                p0: [1.0, 0.0, 0.5],
                p1: [2.0, 0.0, 0.5],
            },
        ],
    };

    let out = stitch_contours(
        &set,
        StitchOptions {
            quantize: Quantize { grid: 1e-3 },
        },
    )
    .expect("stitch");
    assert_eq!(out.polylines.len(), 1);
    assert_eq!(out.polylines[0].points.len(), 3);
}
