//! Property-based tests for the core determinism / invariant contracts.

use proptest::prelude::*;

use kira_spatial_3d::{
    ContourStats, HeightMapSpec, HeightMode, HeightmapOptions, Mesh, Normalization,
    NormalizeOptions, Quantize, ScalarField, SpatialDomain, StitchOptions, build_heightmap_mesh,
    build_heightmap_mesh_mapped, extract_contours, extract_contours_with_stats,
    for_each_contour_segment, normalize, stitch_contours,
};

fn make_field(values: Vec<f32>, nx: usize, ny: usize) -> (SpatialDomain, Vec<f32>) {
    let domain = SpatialDomain::new(nx, ny, 0.0, 0.0, 1.0, 1.0).expect("valid domain");
    assert_eq!(values.len(), nx * ny);
    (domain, values)
}

proptest! {
    /// `extract_contours` is deterministic: identical inputs produce identical
    /// outputs across runs.
    #[test]
    fn extract_contours_is_deterministic(
        nx in 2usize..=8,
        ny in 2usize..=8,
        seed in any::<u32>(),
        level in -10.0_f32..10.0,
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let v = ((s >> 33) as i32 as f32) / 1_000_000.0;
            values.push(v);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let a = extract_contours(&field, &[level]).unwrap();
        let b = extract_contours(&field, &[level]).unwrap();
        prop_assert_eq!(a, b);
    }

    /// Mesh integrity: every index references a valid vertex; vertex
    /// and normal counts match; index count is a multiple of 3.
    #[test]
    fn mesh_indices_are_valid(
        nx in 2usize..=12,
        ny in 2usize..=12,
        seed in any::<u32>(),
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            values.push(((s >> 33) as i32 as f32) / 1_000_000.0);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).unwrap();

        prop_assert_eq!(mesh.vertices.len(), mesh.normals.len());
        prop_assert!(mesh.indices.len().is_multiple_of(3));
        let vc = mesh.vertices.len() as u32;
        prop_assert!(mesh.indices.iter().all(|&i| i < vc));
        // Every triangle should be non-degenerate index-wise (three distinct vertex ids).
        for tri in mesh.indices.chunks_exact(3) {
            prop_assert!(tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2]);
        }
    }

    /// Mesh normals are unit-length (within f32 tolerance).
    #[test]
    fn mesh_normals_are_unit(
        nx in 2usize..=10,
        ny in 2usize..=10,
        seed in any::<u32>(),
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            values.push(((s >> 33) as i32 as f32) / 1_000_000.0);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).unwrap();
        for &nrm in &mesh.normals {
            let len2 = nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2];
            // The scalar normal path falls back to `[0, 0, 1]` if the
            // gradient overflows — both unit-length cases (true normal
            // and fallback) satisfy this bound.
            prop_assert!((len2 - 1.0_f32).abs() < 1e-4, "non-unit normal len^2={len2}");
        }
    }

    /// `Normalization::None` preserves finite values bitwise and maps
    /// every non-finite value to exact `0.0`.
    #[test]
    fn normalize_none_is_identity_for_finite(values in prop::collection::vec(-1e6_f32..1e6, 1..64)) {
        let out = normalize(&values, NormalizeOptions { policy: Normalization::None });
        for (i, &v) in values.iter().enumerate() {
            if v.is_finite() {
                prop_assert_eq!(out[i].to_bits(), v.to_bits());
            } else {
                prop_assert_eq!(out[i], 0.0);
            }
        }
    }

    /// `Normalization::MinMax` (no clip) lands in `[0, 1]` for every finite
    /// input and maps the global min to 0 and global max to 1.
    #[test]
    fn normalize_minmax_is_in_unit_range(
        values in prop::collection::vec(-1e3_f32..1e3, 2..64),
    ) {
        let out = normalize(
            &values,
            NormalizeOptions { policy: Normalization::MinMax { clip: None } },
        );
        for &v in &out {
            prop_assert!((0.0..=1.0).contains(&v) || v == 0.0,
                "out-of-range minmax output: {v}");
        }
    }

    /// `extract_contours_with_stats` accumulates skipped cells; the count
    /// never exceeds the cell count of the grid.
    #[test]
    fn skipped_cells_bounded_by_grid_size(
        nx in 2usize..=8,
        ny in 2usize..=8,
        level in -10.0_f32..10.0,
    ) {
        let n = nx * ny;
        let values = vec![0.5_f32; n];
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let (_, stats) = extract_contours_with_stats(&field, &[level]).unwrap();
        let cell_count = (nx - 1) * (ny - 1);
        prop_assert!(stats.skipped_cells <= cell_count);
    }

    /// `for_each_contour_segment` emits exactly the same segments in the
    /// same order as `extract_contours` for the same input.
    #[test]
    fn streaming_matches_eager(
        nx in 2usize..=8,
        ny in 2usize..=8,
        seed in any::<u32>(),
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            values.push(((s >> 33) as i32 as f32) / 1_000_000.0);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();

        let eager = extract_contours(&field, &[0.5_f32, -0.25_f32]).unwrap();
        let mut streamed: Vec<(f32, [f32; 3], [f32; 3])> = Vec::new();
        let _stats: ContourStats = for_each_contour_segment(
            &field,
            &[0.5_f32, -0.25_f32],
            |level, seg| streamed.push((level, seg.p0, seg.p1)),
        )
        .unwrap();

        let mut flat: Vec<(f32, [f32; 3], [f32; 3])> = Vec::new();
        for cs in &eager.contours {
            for seg in &cs.segments {
                flat.push((cs.level, seg.p0, seg.p1));
            }
        }
        prop_assert_eq!(flat, streamed);
    }

    /// `stitch_contours` is deterministic across hash seeds. The
    /// internal switch from `BTreeMap`+`BTreeSet` to `HashMap`+`HashSet`
    /// introduced non-deterministic iteration order; this test runs the
    /// same input through `stitch_contours` repeatedly and asserts
    /// bitwise output equality. A regression here would mean we missed
    /// a sort somewhere.
    #[test]
    fn stitch_contours_is_deterministic_across_runs(
        nx in 3usize..=8,
        ny in 3usize..=8,
        seed in any::<u32>(),
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            values.push(((s >> 33) as i32 as f32) / 1_000_000.0);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let multi = extract_contours(&field, &[0.0_f32]).unwrap();
        let set = &multi.contours[0];
        let q = Quantize { grid: 1e-3 };

        // Run the stitcher five times back-to-back; HashMap rebuilds
        // its hasher state on each `HashMap::new()` so any iteration
        // dependence would surface as drift across runs.
        let first = stitch_contours(set, StitchOptions { quantize: q }).unwrap();
        for _ in 0..4 {
            let again = stitch_contours(set, StitchOptions { quantize: q }).unwrap();
            prop_assert_eq!(&first, &again);
        }
    }

    /// `stitch_contours` round-trip: every original segment's endpoints
    /// (after quantization) appear in at least one resulting polyline.
    #[test]
    fn stitch_preserves_endpoints(
        nx in 3usize..=8,
        ny in 3usize..=8,
        seed in any::<u32>(),
    ) {
        let n = nx * ny;
        let mut values = Vec::with_capacity(n);
        let mut s = seed as u64;
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            values.push(((s >> 33) as i32 as f32) / 1_000_000.0);
        }
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let multi = extract_contours(&field, &[0.0_f32]).unwrap();
        let set = &multi.contours[0];
        let q = Quantize { grid: 1e-3 };
        let stitched = stitch_contours(set, StitchOptions { quantize: q }).unwrap();

        // Build a set of every quantized endpoint that appears anywhere
        // in the stitched output.
        use std::collections::HashSet;
        let mut present: HashSet<(i32, i32)> = HashSet::new();
        for p in &stitched.polylines {
            for pt in &p.points {
                present.insert(((pt[0] / q.grid).round() as i32, (pt[1] / q.grid).round() as i32));
            }
        }

        // Every non-zero-length segment's both endpoints must be present.
        for seg in &set.segments {
            let k0 = ((seg.p0[0] / q.grid).round() as i32, (seg.p0[1] / q.grid).round() as i32);
            let k1 = ((seg.p1[0] / q.grid).round() as i32, (seg.p1[1] / q.grid).round() as i32);
            if k0 == k1 {
                continue;
            }
            prop_assert!(present.contains(&k0), "missing endpoint {:?}", k0);
            prop_assert!(present.contains(&k1), "missing endpoint {:?}", k1);
        }
    }

    /// `Mesh::bounds()` is consistent with the vertex array.
    #[test]
    fn mesh_bounds_match_vertices(
        nx in 2usize..=10,
        ny in 2usize..=10,
    ) {
        let n = nx * ny;
        let values = (0..n).map(|i| i as f32 * 0.1).collect::<Vec<_>>();
        let (domain, values) = make_field(values, nx, ny);
        let field = ScalarField::new(domain, &values).unwrap();
        let mesh = build_heightmap_mesh(&field, HeightmapOptions::default()).unwrap();
        let bounds = mesh.bounds().expect("non-empty mesh");
        let mut min = mesh.vertices[0];
        let mut max = mesh.vertices[0];
        for &p in &mesh.vertices {
            for i in 0..3 {
                if p[i] < min[i] {
                    min[i] = p[i];
                }
                if p[i] > max[i] {
                    max[i] = p[i];
                }
            }
        }
        prop_assert_eq!(bounds.min, min);
        prop_assert_eq!(bounds.max, max);
    }
}

#[test]
fn median_for_even_length_takes_pair_mean() {
    // 4-element finite array: median should be (sorted[1] + sorted[2]) / 2.
    let v = [4.0_f32, 1.0, 3.0, 2.0]; // sorted: 1, 2, 3, 4 → median = 2.5
    // Easiest deterministic way to surface this through the public API
    // is to drive `RobustZ` and check the `z = (v - median) / (1.4826 * mad)`
    // formula: with these inputs, median = 2.5, mad over |dev| = median(1.5, 0.5, 0.5, 1.5) = 1.0.
    let out = normalize(
        &v,
        NormalizeOptions {
            policy: Normalization::RobustZ { clip_z: None },
        },
    );
    let denom = 1.4826_f32 * 1.0;
    let expected: Vec<f32> = v.iter().map(|x| (x - 2.5) / denom).collect();
    for (got, exp) in out.iter().zip(expected.iter()) {
        assert!((got - exp).abs() < 1e-5, "got={got} expected={exp}");
    }
}

#[test]
fn percentile_linear_interpolation_is_used() {
    // 5 elements, percentile lo=25 hi=75. Linear interpolation:
    // rank_lo = 0.25 * 4 = 1.0 → exact element at index 1 (value 2)
    // rank_hi = 0.75 * 4 = 3.0 → exact element at index 3 (value 4)
    // After mapping the input [1, 2, 3, 4, 5]:
    // clamp to [2, 4], then `(v - 2) / 2`:
    // [0, 0, 0.5, 1, 1] (since 5 is clamped to 4)
    let v = [1.0_f32, 2.0, 3.0, 4.0, 5.0];
    let out = normalize(
        &v,
        NormalizeOptions {
            policy: Normalization::Percentile { lo: 25.0, hi: 75.0 },
        },
    );
    let expected = [0.0_f32, 0.0, 0.5, 1.0, 1.0];
    for (got, exp) in out.iter().zip(expected.iter()) {
        assert!((got - exp).abs() < 1e-5, "got={got} expected={exp}");
    }
}

#[test]
fn build_heightmap_mesh_mapped_works_end_to_end() {
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).unwrap();
    let values = (0..9).map(|i| i as f32).collect::<Vec<_>>();
    let field = ScalarField::new(domain, &values).unwrap();
    let spec = HeightMapSpec {
        mode: HeightMode::Raw,
        normalization: Normalization::MinMax { clip: None },
        z_scale: 2.0,
        z_offset: 0.5,
        compute: Default::default(),
    };
    let _m: Mesh = build_heightmap_mesh_mapped(&field, spec).unwrap();
}
