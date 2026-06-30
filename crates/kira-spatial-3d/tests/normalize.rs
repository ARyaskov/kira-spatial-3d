//! Normalization policy behaviour (RobustZ, MinMax, Percentile, None).

use kira_spatial_3d::{Normalization, NormalizeOptions, normalize};

#[test]
fn percentile_is_deterministic() {
    let values = [3.0_f32, 2.0, 7.0, 1.0, 9.0, 5.0];
    let opts = NormalizeOptions {
        policy: Normalization::Percentile { lo: 10.0, hi: 90.0 },
    };
    let a = normalize(&values, opts);
    let b = normalize(&values, opts);
    assert_eq!(a, b);
}

#[test]
fn missing_values_map_to_zero() {
    let values = [1.0_f32, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 2.0];
    let out = normalize(
        &values,
        NormalizeOptions {
            policy: Normalization::MinMax { clip: None },
        },
    );
    assert_eq!(out[1], 0.0);
    assert_eq!(out[2], 0.0);
    assert_eq!(out[3], 0.0);
}

#[test]
fn robust_z_zero_mad_outputs_zeros() {
    let values = [4.0_f32, 4.0, 4.0, 4.0];
    let out = normalize(
        &values,
        NormalizeOptions {
            policy: Normalization::RobustZ { clip_z: None },
        },
    );
    assert!(out.iter().all(|&v| v == 0.0));
}
