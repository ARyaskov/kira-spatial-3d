//! Streaming heightmap mesh builder regression tests.

use std::fs;

use kira_spatial_3d::{
    BufferOptions, HeightmapOptions, ScalarField, SpatialDomain, build_heightmap_mesh,
    build_heightmap_mesh_to_k3d, write_k3d_mesh_buffer,
};
use tempfile::tempdir;

fn run_streaming_vs_eager(nx: usize, ny: usize, write_normals: bool) {
    let domain = SpatialDomain::new(nx, ny, 0.0, 0.0, 1.0, 1.0).expect("domain");
    let mut values = Vec::with_capacity(nx * ny);
    for y in 0..ny {
        for x in 0..nx {
            let cx = x as f32 - (nx as f32 - 1.0) * 0.5;
            let cy = y as f32 - (ny as f32 - 1.0) * 0.5;
            values.push((-(cx * cx + cy * cy) / 16.0).exp());
        }
    }
    let field = ScalarField::new(domain, &values).expect("field");

    let dir = tempdir().expect("tmpdir");

    // Eager path: build the full Mesh, then write the K3D buffer.
    let eager_prefix = dir.path().join("eager");
    let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).expect("eager mesh");
    let (eager_bin, eager_json) =
        write_k3d_mesh_buffer(&mesh, &eager_prefix, BufferOptions { write_normals })
            .expect("eager k3d");

    // Streaming path: same inputs, write straight to K3D.
    let stream_prefix = dir.path().join("stream");
    let (stream_bin, stream_json) = build_heightmap_mesh_to_k3d(
        &field,
        HeightmapOptions::default(),
        &stream_prefix,
        BufferOptions { write_normals },
    )
    .expect("stream k3d");

    let eager_bin_bytes = fs::read(&eager_bin).expect("read eager bin");
    let stream_bin_bytes = fs::read(&stream_bin).expect("read stream bin");
    assert_eq!(eager_bin_bytes, stream_bin_bytes, "bin bytes differ");

    let eager_json_bytes = fs::read(&eager_json).expect("read eager json");
    let stream_json_bytes = fs::read(&stream_json).expect("read stream json");
    assert_eq!(eager_json_bytes, stream_json_bytes, "json bytes differ");
}

#[test]
fn streaming_matches_eager_small_grid_with_normals() {
    run_streaming_vs_eager(4, 4, true);
}

#[test]
fn streaming_matches_eager_small_grid_no_normals() {
    run_streaming_vs_eager(4, 4, false);
}

#[test]
fn streaming_matches_eager_rectangular_grid() {
    run_streaming_vs_eager(7, 3, true);
}

#[test]
fn streaming_matches_eager_medium_grid() {
    run_streaming_vs_eager(32, 32, true);
}
