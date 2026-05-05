# dxf-render

Native Rust CLI that renders DXF files to PNG. Built to back a Tauri sidecar in [drawing-reviewer](https://github.com/jamesdowzard/drawing-reviewer), replacing a 158-second WASM cold-start with sub-second native rendering.

## Status

Phase 1 in progress. Compiles cleanly, renders something, fidelity not yet verified.

## Usage

```bash
cargo run --release -- input.dxf output.png --width 2048
```

Pre-process DWG with `libredwg`'s `dwg2dxf` (~9ms native conversion).

## Roadmap

- **Phase 1** (in progress) — `LWPOLYLINE`, `LINE`, `CIRCLE`, `MTEXT`, `INSERT` + block resolution
- **Phase 2** — `ATTRIB`, `TEXT`, title block content
- **Phase 3** — `DIMENSION`, `MULTILEADER`, `HATCH`

## Stack

- `tiny-skia` — 2D rasteriser
- `ab_glyph` — TTF text
- `clap` — CLI
- Hand-rolled DXF parser (no `dxf-rs` dependency)
