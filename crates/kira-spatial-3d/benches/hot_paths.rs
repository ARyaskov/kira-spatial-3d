//! Micro-benchmarks for the hot paths.
//!
//! Run with:
//!
//! ```bash
//! cargo bench -p kira-spatial-3d --bench hot_paths
//! ```

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use kira_spatial_3d::{
    FloatFmt, HeightmapOptions, Normalization, NormalizeOptions, ObjOptions, Quantize, ScalarField,
    SpatialDomain, StitchOptions, build_heightmap_mesh, extract_contours, normalize, save_obj,
    stitch_contours,
};

fn build_field(nx: usize, ny: usize) -> (SpatialDomain, Vec<f32>) {
    let domain = SpatialDomain::new(nx, ny, 0.0, 0.0, 1.0, 1.0).expect("valid domain");
    let mut values = Vec::with_capacity(nx * ny);
    for y in 0..ny {
        for x in 0..nx {
            let cx = x as f32 - (nx / 2) as f32;
            let cy = y as f32 - (ny / 2) as f32;
            values.push((-(cx * cx + cy * cy) / 64.0).exp());
        }
    }
    (domain, values)
}

fn bench_build_mesh(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_heightmap_mesh");
    for &n in &[64_usize, 256, 1024] {
        let (domain, values) = build_field(n, n);
        group.throughput(Throughput::Elements((n * n) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{n}")),
            &(),
            |b, _| {
                b.iter(|| {
                    let field = ScalarField::new(domain, &values).unwrap();
                    let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).unwrap();
                    criterion::black_box(mesh);
                })
            },
        );
    }
    group.finish();
}

fn bench_extract_contours(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_contours");
    for &n in &[64_usize, 256, 1024] {
        let (domain, values) = build_field(n, n);
        let field = ScalarField::new(domain, &values).unwrap();
        group.throughput(Throughput::Elements((n * n) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{n}")),
            &(),
            |b, _| {
                b.iter(|| {
                    let r = extract_contours(&field, &[0.2_f32, 0.5, 0.8]).unwrap();
                    criterion::black_box(r);
                })
            },
        );
    }
    group.finish();
}

fn bench_stitch_contours(c: &mut Criterion) {
    let mut group = c.benchmark_group("stitch_contours");
    for &n in &[256_usize, 1024] {
        let (domain, values) = build_field(n, n);
        let field = ScalarField::new(domain, &values).unwrap();
        let multi = extract_contours(&field, &[0.5_f32]).unwrap();
        group.throughput(Throughput::Elements(multi.contours[0].segments.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{n}")),
            &(),
            |b, _| {
                b.iter(|| {
                    let p = stitch_contours(
                        &multi.contours[0],
                        StitchOptions {
                            quantize: Quantize { grid: 1e-3 },
                        },
                    )
                    .unwrap();
                    criterion::black_box(p);
                })
            },
        );
    }
    group.finish();
}

fn bench_normalize_minmax(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_minmax");
    for &n in &[10_000_usize, 100_000, 1_000_000] {
        let (_, values) = build_field(
            (n as f32).sqrt() as usize + 1,
            (n as f32).sqrt() as usize + 1,
        );
        let values = values[..n].to_vec();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &(), |b, _| {
            b.iter(|| {
                let r = normalize(
                    &values,
                    NormalizeOptions {
                        policy: Normalization::MinMax { clip: None },
                    },
                );
                criterion::black_box(r);
            })
        });
    }
    group.finish();
}

fn bench_write_obj(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_obj_in_memory");
    for &n in &[64_usize, 256] {
        let (domain, values) = build_field(n, n);
        let field = ScalarField::new(domain, &values).unwrap();
        let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).unwrap();
        group.throughput(Throughput::Elements(mesh.vertices.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{n}")),
            &(),
            |b, _| {
                b.iter(|| {
                    let path = std::env::temp_dir().join("kira-bench.obj");
                    save_obj(
                        &mesh,
                        &path,
                        ObjOptions {
                            float: FloatFmt::DEFAULT,
                            write_normals: true,
                        },
                    )
                    .unwrap();
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_build_mesh,
    bench_extract_contours,
    bench_stitch_contours,
    bench_normalize_minmax,
    bench_write_obj
);
criterion_main!(benches);
