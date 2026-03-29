# kira-spatial-3d

Deterministic projection crate that turns precomputed scalar fields into 3D mesh, contour, and export artifacts.

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
