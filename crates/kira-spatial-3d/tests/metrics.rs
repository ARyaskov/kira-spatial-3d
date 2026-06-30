//! Ridge metrics (length, fragmentation, turn angles).

use kira_spatial_3d::{
    ContourSegment, ContourSet, Quantize, StitchOptions, compute_ridge_metrics,
    ridges_to_polylines_and_metrics,
};

#[test]
fn metrics_are_sane_and_finite() {
    let contours = ContourSet {
        level: 1.0,
        segments: vec![
            ContourSegment {
                p0: [0.0, 0.0, 1.0],
                p1: [1.0, 0.0, 1.0],
            },
            ContourSegment {
                p0: [1.0, 0.0, 1.0],
                p1: [2.0, 0.0, 1.0],
            },
            ContourSegment {
                p0: [5.0, 0.0, 1.0],
                p1: [6.0, 0.0, 1.0],
            },
        ],
    };

    let (poly, metrics) = ridges_to_polylines_and_metrics(
        &contours,
        StitchOptions {
            quantize: Quantize { grid: 1e-3 },
        },
    )
    .expect("pipeline");
    let direct = compute_ridge_metrics(&poly);

    assert_eq!(metrics, direct);
    assert_eq!(metrics.num_polylines, 2);
    assert_eq!(metrics.num_open, 2);
    assert_eq!(metrics.num_closed, 0);
    assert_eq!(metrics.num_endpoints, 4);
    assert_eq!(metrics.total_length, 3.0);
    assert_eq!(metrics.mean_length, 1.5);
    assert!(metrics.fragmentation_index.is_finite());
    assert!(metrics.mean_abs_turn_angle.is_finite());
}
