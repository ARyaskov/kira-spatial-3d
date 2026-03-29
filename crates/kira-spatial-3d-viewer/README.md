# kira-spatial-3d-viewer

Minimal real-time preview for `kira-spatial-3d` outputs.

Supported inputs (v1):
- `<prefix>.k3d.json`
- `<prefix>.k3d.bin`
- optional `polylines.json`

When `--polylines` points to `polylines.level_*.json`, the viewer auto-loads all sibling
`polylines.level_*.json` files from the same directory and allows runtime level switching.

## Usage

```bash
cargo run -p kira-spatial-3d-viewer -- --mesh out/mesh --polylines out/polylines.level_0.5.json
```

## Controls

- `1/2/3`: camera presets (top/oblique/side)
- `LMB + drag`: orbit
- `Mouse wheel`: zoom
- `+/-`: increase/decrease vertical exaggeration
- `Q/E`: sensitivity mode (low/normal/high)
- `C`: toggle height colormap
- `L`: toggle contour visibility
- `,` and `.` (or `PageDown/PageUp`): previous/next contour level

The current values and hotkey legend are shown in the window title.

## Determinism

The viewer does not recompute or reorder geometry.
Given identical buffers and the same GPU/driver stack, render output is stable across runs.
