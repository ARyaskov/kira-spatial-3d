# kira-spatial-3d

Deterministic projection crate that turns precomputed scalar fields into 3D
mesh, contour, and export artifacts.

## Determinism contract

- Identical inputs produce bitwise-identical outputs across runs and across
  binaries built for the same CPU vendor family.
- SIMD paths (`x86_64` AVX2, `aarch64` NEON) must match the scalar reference
  path bit-for-bit on the operations they cover. The scalar path is the source
  of truth.
- `#![deny(unsafe_code)]` at the crate root; `simd::x86_avx2` and
  `simd::aarch64_neon` opt back in via `#[allow(unsafe_code)]` and call
  `target_feature`-gated platform intrinsics. The unsafe surface is confined
  to those two modules and the `ScalarField::get_unchecked` fast accessor.

## MSRV

Rust **1.95** (edition 2024).

## Examples

```bash
cargo run --example heightmap_to_obj
```

The example builds a small synthetic field, runs the deterministic
projection, extracts a contour, and writes an OBJ and a polylines JSON to a
temp directory.

## K3D Mesh Buffer v1

GPU-ready export format:

- `mesh.k3d.bin`: raw tightly packed little-endian blocks
  1. positions: `N * 3 * f32`
  2. normals: `N * 3 * f32` (optional)
  3. indices: `M * u32`
- `mesh.k3d.json`: deterministic metadata with offsets/sizes

No padding is inserted between blocks.

### Upload to wgpu (high-level)

Use offsets from `mesh.k3d.json` to create vertex/index buffer slices:

- positions slice for `Float32x3` vertex attribute
- normals slice for `Float32x3` normal attribute (if present)
- indices slice as `Uint32`

## Minimal glTF Export (optional)

Enable feature:

```bash
cargo build -p kira-spatial-3d --features gltf
```

Writer emits:

- `mesh.gltf`
- `mesh.bin`

The JSON references external `.bin` (no base64/data URI embedding).

## Benchmarks

```bash
cargo bench -p kira-spatial-3d --bench hot_paths
```

Bench coverage: `build_heightmap_mesh`, `extract_contours`,
`stitch_contours`, `normalize_minmax`, `write_obj`.
