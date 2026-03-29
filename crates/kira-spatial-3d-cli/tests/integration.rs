use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;

use kira_spatial_3d_cli::run_manifest;
use tempfile::tempdir;

#[test]
fn run_manifest_is_deterministic() {
    let dir = tempdir().expect("tmp");
    let root = dir.path();

    let field_path = root.join("grad_mag.f32");
    write_f32le(&field_path, &[0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0]);

    let manifest_path = root.join("manifest.json");
    let manifest = r#"{
  "version": "kira-spatial-manifest/v1",
  "domain": {
    "nx": 3,
    "ny": 3,
    "origin_x": 0.0,
    "origin_y": 0.0,
    "step_x": 1.0,
    "step_y": 1.0
  },
  "field": {
    "name": "grad_mag",
    "format": "f32le",
    "path": "grad_mag.f32"
  },
  "mapping": {
    "mode": "Abs",
    "normalization": { "type": "Percentile", "lo": 5.0, "hi": 95.0 },
    "z_scale": 20.0,
    "z_offset": 0.0
  },
  "contours": {
    "levels": [0.2, 0.6],
    "quantize_grid": 0.01
  },
  "export": {
    "out_dir": "out",
    "float_decimals": 6,
    "write_obj": true,
    "write_ply": true,
    "write_polylines": true,
    "write_metrics": true,
    "write_metadata": true
  }
}"#;
    fs::write(&manifest_path, manifest).expect("manifest");

    run_manifest(&manifest_path, false, false).expect("run1");
    let first = read_out_files(&root.join("out"));

    run_manifest(&manifest_path, false, false).expect("run2");
    let second = read_out_files(&root.join("out"));

    assert_eq!(first, second);
    assert!(second.contains_key("surface.obj"));
    assert!(second.contains_key("surface.ply"));
    assert!(second.contains_key("metadata.json"));
    assert!(second.contains_key("index.json"));
}

#[test]
fn run_manifest_scalar_no_contours() {
    let dir = tempdir().expect("tmp");
    let root = dir.path();

    let field_path = root.join("delta.f32");
    write_f32le(&field_path, &[1.0, -1.0, 1.0, -1.0]);

    let manifest_path = root.join("manifest.json");
    let manifest = r#"{
  "version": "kira-spatial-manifest/v1",
  "domain": {
    "nx": 2,
    "ny": 2,
    "origin_x": 0.0,
    "origin_y": 0.0,
    "step_x": 1.0,
    "step_y": 1.0
  },
  "field": {
    "name": "delta",
    "format": "f32le",
    "path": "delta.f32"
  },
  "mapping": {
    "mode": "Signed",
    "normalization": { "type": "None" },
    "z_scale": 2.0,
    "z_offset": 0.0
  },
  "contours": {
    "levels": [0.1],
    "quantize_grid": 0.01
  },
  "export": {
    "out_dir": "out",
    "float_decimals": 6,
    "write_obj": true,
    "write_ply": true,
    "write_polylines": true,
    "write_metrics": true,
    "write_metadata": true
  }
}"#;
    fs::write(&manifest_path, manifest).expect("manifest");

    run_manifest(&manifest_path, true, true).expect("run");
    let out = read_out_files(&root.join("out"));

    assert!(out.contains_key("surface.obj"));
    assert!(out.contains_key("surface.ply"));
    assert!(out.contains_key("metadata.json"));
    assert!(!out.keys().any(|k| k.starts_with("polylines.level_")));
    assert!(!out.keys().any(|k| k.starts_with("ridge_metrics.level_")));
}

fn write_f32le(path: &std::path::Path, values: &[f32]) {
    let mut f = File::create(path).expect("field create");
    for v in values {
        f.write_all(&v.to_le_bytes()).expect("field write");
    }
}

fn read_out_files(out_dir: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    let mut map = BTreeMap::new();
    for entry in fs::read_dir(out_dir).expect("read out") {
        let entry = entry.expect("dir entry");
        let file_name = entry.file_name().to_string_lossy().to_string();
        let bytes = fs::read(entry.path()).expect("read file");
        map.insert(file_name, bytes);
    }
    map
}
