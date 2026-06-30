//! OBJ / PLY / K3D / polyline / metadata exporters.

use std::fs;

use kira_spatial_3d::{
    ExportBundleOptions, FloatFmt, Mesh, ObjOptions, Polyline, PolylineSet, TsvOptions,
    export_bundle, fmt_f32, save_obj, save_polylines_json, save_polylines_tsv,
    write_k3d_mesh_buffer,
};
use tempfile::tempdir;

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
            points: vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
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
    let opts = ExportBundleOptions {
        write_k3d: true,
        ..ExportBundleOptions::default()
    };

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
