//! Heightmap mesh build, backend repeatability, and `build_heights`.

use kira_spatial_3d::{
    ComputeBackend, ComputeConfig, HeightMapSpec, HeightMode, HeightmapOptions, Normalization,
    ScalarField, SpatialDomain, build_heightmap_mesh, build_heights,
};

#[test]
fn two_by_two_mesh_is_deterministic() {
    let domain = SpatialDomain::new(2, 2, 0.0, 0.0, 1.0, 1.0).expect("valid domain");
    let values = [0.0_f32, 1.0, 2.0, 3.0];
    let field = ScalarField::new(domain, &values).expect("valid scalar field");

    let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).expect("mesh builds");

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert_eq!(mesh.indices, vec![0, 1, 2, 1, 3, 2]);
    assert_eq!(mesh.normals.len(), 4);
    assert!(mesh.normals.iter().all(|n| n.iter().all(|c| c.is_finite())));
}

#[test]
fn apply_mode_and_affine_via_build_heights_handles_non_finite() {
    let domain = SpatialDomain::new(2, 2, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [1.0_f32, f32::NAN, f32::INFINITY, -2.0];
    let field = ScalarField::new(domain, &values).expect("field");

    let spec = HeightMapSpec {
        mode: HeightMode::Raw,
        normalization: Normalization::None,
        z_scale: 2.0,
        z_offset: 1.0,
        compute: ComputeConfig {
            backend: ComputeBackend::Auto,
        },
    };

    let out = build_heights(&field, spec).expect("heights");
    assert_eq!(out.len(), 4);
    assert_eq!(out[0], 3.0);
    assert_eq!(out[1], 1.0);
    assert_eq!(out[2], 1.0);
    assert_eq!(out[3], -3.0);
    assert!(out.iter().all(|v| v.is_finite()));
}

#[test]
fn scalar_backend_repeatability() {
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [0.0_f32, 1.0, 2.0, 1.0, 3.0, 5.0, 2.0, 5.0, 8.0];
    let field = ScalarField::new(domain, &values).expect("field");
    let opts = HeightmapOptions {
        z_scale: 1.5,
        z_offset: -0.25,
        compute: ComputeConfig {
            backend: ComputeBackend::Scalar,
        },
    };

    let a = build_heightmap_mesh(&field, opts).expect("mesh a");
    let b = build_heightmap_mesh(&field, opts).expect("mesh b");
    assert_eq!(a.vertices, b.vertices);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.indices, b.indices);
}

#[test]
fn auto_backend_repeatability_and_matches_scalar_for_heights() {
    let domain = SpatialDomain::new(2, 3, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let values = [0.1_f32, -2.0, 3.0, 4.0, 5.5, -6.5];
    let field = ScalarField::new(domain, &values).expect("field");

    let scalar_spec = HeightMapSpec {
        mode: HeightMode::Abs,
        normalization: Normalization::None,
        z_scale: 2.0,
        z_offset: 0.5,
        compute: ComputeConfig {
            backend: ComputeBackend::Scalar,
        },
    };
    let auto_spec = HeightMapSpec {
        compute: ComputeConfig {
            backend: ComputeBackend::Auto,
        },
        ..scalar_spec
    };

    let s = build_heights(&field, scalar_spec).expect("scalar heights");
    let a1 = build_heights(&field, auto_spec).expect("auto heights1");
    let a2 = build_heights(&field, auto_spec).expect("auto heights2");
    assert_eq!(a1, a2);
    assert_eq!(s, a1);
}
