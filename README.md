# kira-spatial-3d

Deterministic projection from scalar fields to 3D mesh, contour, and export
artifacts for spatial omics workflows.

The workspace ships three crates:

- [`kira-spatial-3d`](crates/kira-spatial-3d) — library: heightmap meshing,
  marching-squares contours, polyline stitching, ridge metrics, OBJ/PLY/glTF/K3D
  exporters, scalar normalization, SIMD acceleration with scalar-equivalent
  fallback.
- [`kira-spatial-3d-cli`](crates/kira-spatial-3d-cli) — manifest-driven
  command-line front end (`kira-spatial-3d run --manifest spec.json`).
- [`kira-spatial-3d-viewer`](crates/kira-spatial-3d-viewer) — interactive
  wgpu/winit viewer for K3D-buffer meshes and JSON polyline overlays.

## Determinism contract

For identical inputs and identical binary on identical CPU vendor families,
every public entry point produces bitwise-identical output across runs. SIMD
paths are gated on runtime feature detection and are required to match their
scalar reference path bit-for-bit; the scalar path is the source of truth.

Sort order is canonicalized (`f32::total_cmp` for floats, structural ordering
elsewhere), allocation order is fixed, normalization statistics are computed
in a fixed pass, and Marching Squares ambiguous cases (`5`, `10`) use a
documented center tie-break.

## MSRV

The workspace targets **Rust 1.95** (edition 2024).

## Quick start

```bash
# library
cargo build -p kira-spatial-3d
cargo test -p kira-spatial-3d
cargo run --example heightmap_to_obj -p kira-spatial-3d

# CLI
cargo run -p kira-spatial-3d-cli -- run --manifest spec.json

# viewer (requires a working GPU)
cargo run -p kira-spatial-3d-viewer -- --mesh path/to/mesh
```

## Features

- `default = []`
- `gltf` — opt-in minimal glTF 2.0 writer (`mesh.gltf` + external `.bin`).

## Lints, CI, CHANGELOG

- `clippy.toml` / `rustfmt.toml` at the workspace root.
- `.github/workflows/ci.yml` runs build/test/fmt/clippy/MSRV across
  Linux/macOS/Windows × (default | `gltf`).
- See [`CHANGELOG.md`](CHANGELOG.md) for release notes.

## License

MIT — see [`LICENSE`](LICENSE).
