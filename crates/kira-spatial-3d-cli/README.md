# kira-spatial-3d-cli

Manifest-driven deterministic CLI wrapper for `kira-spatial-3d`.

This tool consumes precomputed scalar fields (for example `|∇f|` or `Δf`) and produces geometry/export artifacts without recomputing spatial operators.

## Manifest v1

File: `manifest.json`

```json
{
  "version": "kira-spatial-manifest/v1",
  "domain": {
    "nx": 128,
    "ny": 128,
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
    "levels": [0.2, 0.4, 0.6, 0.8],
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
}
```

Notes:
- `version` must be exactly `kira-spatial-manifest/v1`.
- `field.format` supports only `f32le` in v1.
- `domain` is required and validated (`nx, ny >= 2`, positive steps).
- `contours` is optional.

## Usage

```bash
cargo run -p kira-spatial-3d-cli -- run --manifest /path/to/manifest.json
```

Optional flags:
- `--scalar`: force scalar backend
- `--no-contours`: ignore `contours` section from manifest

## Contour Rule

Contours are extracted on the normalized field (before final `z_scale`/`z_offset` affine mapping).

## Outputs

Deterministic file names:
- `surface.obj`, `surface.ply`
- `polylines.level_<L>.json` / `polylines.level_<L>.tsv`
- `ridge_metrics.level_<L>.json` / `ridge_metrics.level_<L>.tsv`
- `metadata.json`
- `index.json` (when multiple contour levels are emitted)

## Determinism

Given identical inputs and manifest, output bytes are stable.
No timestamps, no randomness, and non-pretty JSON writers are used.
