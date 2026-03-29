use std::fs;

use kira_spatial_3d::{
    ComputeBackend, ComputeConfig, ContourSegment, ContourSet, ExportBundleOptions, FloatFmt,
    HeightMapSpec, HeightMode, HeightmapOptions, Mesh, Normalization, NormalizeOptions, ObjOptions,
    Polyline, PolylineSet, Quantize, ScalarField, SpatialDomain, StitchOptions, TsvOptions,
    build_heightmap_mesh, build_heights, compute_ridge_metrics, export_bundle, extract_contours,
    fmt_f32, normalize, ridges_to_polylines_and_metrics, save_obj, save_polylines_json,
    save_polylines_tsv, stitch_contours, write_k3d_mesh_buffer,
};
use tempfile::tempdir;

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
    assert_eq!(p.points[0], [0.0, 0.0, 1.0]);
    assert_eq!(p.points[3], [3.0, 0.0, 1.0]);
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
    assert_eq!(p.points[0], [0.0, 0.0, 2.0]);
    assert_eq!(p.points[1], [0.0, 1.0, 2.0]);
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

#[test]
fn negative_zero_matches_positive_zero() {
    let fmt = FloatFmt::DEFAULT;
    assert_eq!(fmt_f32(-0.0, fmt), fmt_f32(0.0, fmt));
}

#[test]
fn no_scientific_notation() {
    let fmt = FloatFmt { decimals: 9 };
    let s = fmt_f32(123_456_790.0, fmt);
    assert!(!s.contains('e'));
    assert!(!s.contains('E'));
}

#[test]
fn exporting_same_mesh_twice_is_byte_identical() {
    let mesh = Mesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        vec![[0.0, 0.0, 1.0]; 4],
        vec![0, 1, 2, 1, 3, 2],
    )
    .expect("mesh");

    let dir = tempdir().expect("tmp");
    let p1 = dir.path().join("a.obj");
    let p2 = dir.path().join("b.obj");

    let opts = ObjOptions {
        float: FloatFmt::DEFAULT,
        write_normals: true,
    };
    save_obj(&mesh, &p1, opts).expect("save1");
    save_obj(&mesh, &p2, opts).expect("save2");

    let b1 = fs::read(&p1).expect("read1");
    let b2 = fs::read(&p2).expect("read2");
    assert_eq!(b1, b2);
}

#[test]
fn exporting_same_polylines_twice_is_byte_identical() {
    let set = PolylineSet {
        level: 1.0,
        polylines: vec![Polyline {
            level: 1.0,
            points: vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [2.0, 0.0, 1.0]],
            is_closed: false,
        }],
    };

    let dir = tempdir().expect("tmp");
    let j1 = dir.path().join("a.json");
    let j2 = dir.path().join("b.json");
    save_polylines_json(&set, &j1).expect("save j1");
    save_polylines_json(&set, &j2).expect("save j2");
    assert_eq!(fs::read(&j1).expect("j1"), fs::read(&j2).expect("j2"));

    let t1 = dir.path().join("a.tsv");
    let t2 = dir.path().join("b.tsv");
    let tops = TsvOptions {
        float: FloatFmt::DEFAULT,
    };
    save_polylines_tsv(&set, &t1, tops).expect("save t1");
    save_polylines_tsv(&set, &t2, tops).expect("save t2");
    assert_eq!(fs::read(&t1).expect("t1"), fs::read(&t2).expect("t2"));
}

#[test]
fn export_bundle_writes_expected_files() {
    let mesh = Mesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0.0, 0.0, 1.0]; 3],
        vec![0, 1, 2],
    )
    .expect("mesh");
    let dir = tempdir().expect("tmp");

    export_bundle(
        dir.path(),
        Some(&mesh),
        None,
        None,
        None,
        ExportBundleOptions::default(),
    )
    .expect("bundle");

    assert!(dir.path().join("surface.obj").exists());
    assert!(dir.path().join("surface.ply").exists());
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

#[test]
fn k3d_buffer_is_deterministic_and_offsets_are_valid() {
    let mesh = Mesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        vec![[0.0, 0.0, 1.0]; 4],
        vec![0, 1, 2, 1, 3, 2],
    )
    .expect("mesh");

    let dir = tempdir().expect("tmp");
    let p1 = dir.path().join("mesh_a");
    let p2 = dir.path().join("mesh_b");

    let (bin1, json1) = write_k3d_mesh_buffer(
        &mesh,
        &p1,
        kira_spatial_3d::BufferOptions {
            write_normals: true,
        },
    )
    .expect("write1");
    let (bin2, json2) = write_k3d_mesh_buffer(
        &mesh,
        &p2,
        kira_spatial_3d::BufferOptions {
            write_normals: true,
        },
    )
    .expect("write2");

    let b1 = fs::read(&bin1).expect("b1");
    let b2 = fs::read(&bin2).expect("b2");
    assert_eq!(b1, b2);

    let m1 = fs::read_to_string(&json1).expect("m1");
    let m2 = fs::read_to_string(&json2).expect("m2");
    assert_eq!(m1, m2);

    let meta: serde_json::Value = serde_json::from_str(&m1).expect("json");
    assert_eq!(meta["version"], "k3d-mesh-buffer/v1");
    assert_eq!(meta["positions_offset"], 0);
    let pos_bytes = meta["positions_bytes"].as_u64().expect("pos bytes");
    let nor_off = meta["normals_offset"].as_u64().expect("nor off");
    let nor_bytes = meta["normals_bytes"].as_u64().expect("nor bytes");
    let ind_off = meta["indices_offset"].as_u64().expect("ind off");
    assert_eq!(nor_off, pos_bytes);
    assert_eq!(ind_off, pos_bytes + nor_bytes);
}

#[test]
fn export_bundle_can_write_k3d_files() {
    let mesh = Mesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0.0, 0.0, 1.0]; 3],
        vec![0, 1, 2],
    )
    .expect("mesh");
    let dir = tempdir().expect("tmp");
    let mut opts = ExportBundleOptions::default();
    opts.write_k3d = true;

    export_bundle(dir.path(), Some(&mesh), None, None, None, opts).expect("bundle");

    assert!(dir.path().join("mesh.k3d.bin").exists());
    assert!(dir.path().join("mesh.k3d.json").exists());
}

#[cfg(feature = "gltf")]
#[test]
fn gltf_export_is_deterministic_and_sane() {
    let mesh = Mesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 2.0], [0.0, 1.0, 3.0]],
        vec![[0.0, 0.0, 1.0]; 3],
        vec![0, 1, 2],
    )
    .expect("mesh");

    let dir = tempdir().expect("tmp");
    let p = dir.path().join("mesh");

    let (gltf1, bin1) = kira_spatial_3d::write_gltf(
        &mesh,
        &p,
        kira_spatial_3d::GltfOptions {
            write_normals: true,
        },
    )
    .expect("gltf1");
    let gltf_bytes_1 = fs::read(&gltf1).expect("gltf1");
    let bin_bytes_1 = fs::read(&bin1).expect("bin1");

    let (gltf2, bin2) = kira_spatial_3d::write_gltf(
        &mesh,
        &p,
        kira_spatial_3d::GltfOptions {
            write_normals: true,
        },
    )
    .expect("gltf2");

    assert_eq!(gltf_bytes_1, fs::read(&gltf2).expect("gltf2"));
    assert_eq!(bin_bytes_1, fs::read(&bin2).expect("bin2"));

    let doc: serde_json::Value =
        serde_json::from_slice(&fs::read(&gltf1).expect("read gltf")).expect("parse gltf");
    assert!(doc.get("accessors").is_some());
    assert!(doc.get("bufferViews").is_some());
    assert_eq!(doc["buffers"][0]["byteLength"], 84);
}
