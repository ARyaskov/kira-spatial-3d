//! Minimal end-to-end example: build a heightmap mesh from a synthetic
//! scalar field, extract one iso-contour, write OBJ and polylines JSON to
//! a temp directory.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example heightmap_to_obj
//! ```

use std::fs;

use kira_spatial_3d::{
    FloatFmt, HeightMapSpec, HeightMode, HeightmapOptions, Normalization, ObjOptions, Quantize,
    ScalarField, SpatialDomain, StitchOptions, TsvOptions, build_heightmap_mesh_mapped,
    extract_contours, save_obj, save_polylines_json, save_polylines_tsv, stitch_contours,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let domain = SpatialDomain::new(8, 8, 0.0, 0.0, 1.0, 1.0)?;
    let nx = domain.nx;
    let ny = domain.ny;
    let mut values = Vec::with_capacity(nx * ny);
    for y in 0..ny {
        for x in 0..nx {
            let cx = x as f32 - 3.5;
            let cy = y as f32 - 3.5;
            values.push((-(cx * cx + cy * cy) / 8.0).exp());
        }
    }
    let field = ScalarField::new(domain, &values)?;

    let spec = HeightMapSpec {
        mode: HeightMode::Raw,
        normalization: Normalization::MinMax { clip: None },
        z_scale: 1.0,
        z_offset: 0.0,
        compute: Default::default(),
    };
    let mesh = build_heightmap_mesh_mapped(&field, spec)?;
    let _direct = build_heightmap_mesh(&field, HeightmapOptions::default())?;

    let multi = extract_contours(&field, &[0.5])?;
    let polylines = stitch_contours(
        &multi.contours[0],
        StitchOptions {
            quantize: Quantize { grid: 1e-3 },
        },
    )?;

    let out_dir = std::env::temp_dir().join("kira-spatial-3d-example");
    fs::create_dir_all(&out_dir)?;

    let obj_path = out_dir.join("surface.obj");
    save_obj(
        &mesh,
        &obj_path,
        ObjOptions {
            float: FloatFmt::DEFAULT,
            write_normals: true,
        },
    )?;
    save_polylines_json(&polylines, out_dir.join("polylines.json"))?;
    save_polylines_tsv(
        &polylines,
        out_dir.join("polylines.tsv"),
        TsvOptions {
            float: FloatFmt::DEFAULT,
        },
    )?;

    println!("wrote OBJ + polylines to {}", out_dir.display());
    println!(
        "mesh has {} vertices, {} faces",
        mesh.vertex_count(),
        mesh.face_count()
    );
    Ok(())
}

use kira_spatial_3d::build_heightmap_mesh;
