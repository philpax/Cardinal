# cardinal-xr Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Stardust XR client that displays Cardinal VCV Rack modules as interactive 3D panels with hand-tracked input, cable patching, and a palm-anchored module browser.

**Architecture:** Hybrid approach — imperative fusion/molecules for the rack workspace (modules, cables, interaction), declarative asteroids for the hand menu. Cardinal engine runs on a dedicated thread communicating via channels. Module textures streamed via DMA-BUF zero-copy GPU sharing.

**Tech Stack:** Rust, cardinal-core, stardust-xr-fusion 0.51.0, stardust-xr-molecules 0.51.0, stardust-xr-asteroids 0.51.0, wgpu (Vulkan backend), DRM/DMA-BUF Linux APIs.

**Spec:** `docs/superpowers/specs/2026-04-16-cardinal-xr-design.md`

**Important context:**
- `cardinal-core` API reference: `crates/cardinal-core/src/lib.rs` — safe Rust wrappers for module/cable/event/audio
- `cardinal_thread.rs` — `Command` enum, `spawn_cardinal_thread()`, `RenderResult`
- Stardust fusion client API — generated protocol at `~/.cargo/git/checkouts/core-*/fusion/src/protocol.rs`
- Stardust molecules — `~/.cargo/git/checkouts/` after first build
- `CatalogEntry` currently only has `plugin_slug`, `model_slug`, `model_name` (no tags). The hand menu design assumes tags — Task 14 addresses adding tag support to cardinal-core, with a fallback to plugin-only grouping.

---

## File Structure

```
cardinal-xr/
├── Cargo.toml
├── assets/
│   └── panel.glb                  # Unit-sized extruded quad (1m×1m×PANEL_DEPTH_M)
├── src/
│   ├── main.rs                    # Entry point: client connection, event loop, subsystem wiring
│   ├── constants.rs               # All tunable constants (single source of truth)
│   ├── math.rs                    # Bezier curves, coordinate transforms, smoothing
│   ├── dmatex.rs                  # DMA-BUF texture export from wgpu, import to Stardust
│   ├── workspace.rs               # Owns all modules + cables, dispatches per-frame updates
│   ├── module_panel.rs            # Single module: scene graph, model, resize, delete, move
│   ├── interaction.rs             # Per-widget interaction boxes, fallback plane, input forwarding
│   ├── cable.rs                   # Cable rendering (Lines + Bezier), creation/deletion state machine
│   └── hand_menu.rs               # Asteroids hand menu: palm detection, state, Reify impl
```

---

## Task 1: Crate Skeleton and Constants

**Files:**
- Create: `cardinal-xr/Cargo.toml`
- Create: `cardinal-xr/src/main.rs`
- Create: `cardinal-xr/src/constants.rs`
- Modify: `Cargo.toml` (workspace)

- [ ] **Step 1: Add cardinal-xr to workspace**

In `Cargo.toml` (workspace root), add `"cardinal-xr"` to the members list:

```toml
[workspace]
members = [
    "cardinal-egui",
    "cardinal-xr",
    "crates/cardinal-core",
    "crates/plugins/*",
]
resolver = "2"
```

- [ ] **Step 2: Create cardinal-xr/Cargo.toml**

```toml
[package]
name = "cardinal-xr"
version = "0.1.0"
edition = "2024"

[dependencies]
cardinal-core = { path = "../crates/cardinal-core" }
glam = { version = "0.30.0", features = ["mint"] }
mint = "0.5.9"
tokio = { version = "1.45.0", features = ["macros", "time", "sync"] }
rustc-hash = "2.0.0"

[dependencies.stardust-xr-fusion]
version = "0.51.0"
git = "https://github.com/StardustXR/core.git"

[dependencies.stardust-xr-molecules]
version = "0.51.0"
git = "https://github.com/StardustXR/molecules.git"

[dependencies.stardust-xr-asteroids]
version = "0.51.0"
git = "https://github.com/StardustXR/asteroids.git"
```

- [ ] **Step 3: Create constants.rs**

```rust
// cardinal-xr/src/constants.rs
//! Single source of truth for all tunable values.

use glam::Vec4;

// ── Panel ──────────────────────────────────────────────────────────
pub const PIXELS_PER_METER: f32 = 3000.0;
pub const PANEL_DEPTH_M: f32 = 0.008;

// ── Interaction ────────────────────────────────────────────────────
pub const INTERACTION_BOX_MIN_SIZE_M: f32 = 0.015;
pub const INTERACTION_BOX_PROTRUSION_M: f32 = 0.005;
pub const INTERACTION_LAYER_OFFSET_M: f32 = 0.001;
pub const PORT_HOVER_COLOR: Vec4 = Vec4::new(0.3, 0.5, 1.0, 1.0);
pub const PARAM_HOVER_COLOR: Vec4 = Vec4::new(1.0, 0.6, 0.2, 1.0);
pub const HOVER_HIGHLIGHT_OPACITY_IDLE: f32 = 0.1;
pub const HOVER_HIGHLIGHT_OPACITY_ACTIVE: f32 = 0.6;

// ── Resize ─────────────────────────────────────────────────────────
pub const RESIZE_HANDLE_RADIUS_M: f32 = 0.01;

// ── Cables ─────────────────────────────────────────────────────────
pub const CABLE_THICKNESS_M: f32 = 0.003;
pub const CABLE_SEGMENT_COUNT: usize = 20;
pub const CABLE_SAG_FACTOR: f32 = 0.05;
pub const CABLE_COLORS: &[Vec4] = &[
    Vec4::new(1.0, 0.2, 0.2, 1.0), // red
    Vec4::new(0.2, 0.5, 1.0, 1.0), // blue
    Vec4::new(0.2, 0.9, 0.3, 1.0), // green
    Vec4::new(1.0, 0.9, 0.2, 1.0), // yellow
    Vec4::new(0.7, 0.3, 1.0, 1.0), // purple
    Vec4::new(1.0, 0.5, 0.0, 1.0), // orange
];

// ── Hand Menu ──────────────────────────────────────────────────────
pub const MENU_PALM_UP_THRESHOLD: f32 = 0.7;
pub const MENU_PALM_DOWN_THRESHOLD: f32 = 0.5;
pub const MENU_PALM_OFFSET_M: f32 = 0.05;
pub const MENU_POSITION_SMOOTHING: f32 = 0.3;
pub const MENU_HOVER_EXPAND_DELAY_SECS: f32 = 0.3;
pub const MENU_MAX_VISIBLE_ITEMS: usize = 10;
pub const MENU_ITEM_HEIGHT_M: f32 = 0.025;
pub const MENU_ITEM_WIDTH_M: f32 = 0.08;
pub const MENU_COLUMN_GAP_M: f32 = 0.01;

// ── Module Spawning ────────────────────────────────────────────────
pub const MODULE_SPAWN_DISTANCE_M: f32 = 0.5;

// ── Delete Button ──────────────────────────────────────────────────
pub const DELETE_BUTTON_SIZE_M: f32 = 0.015;
pub const DELETE_BUTTON_OFFSET_M: f32 = 0.01;
```

- [ ] **Step 4: Create minimal main.rs**

```rust
// cardinal-xr/src/main.rs
mod constants;

fn main() {
    eprintln!("cardinal-xr: starting");

    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
    let _audio_stream = cardinal_core::audio::start_audio_stream();

    eprintln!("cardinal-xr: cardinal engine ready, stardust connection not yet implemented");

    // Placeholder: keep process alive
    std::thread::park();
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles successfully (may take time on first build to fetch git deps). The binary starts the cardinal engine and parks.

- [ ] **Step 6: Commit**

```bash
git add cardinal-xr/ Cargo.toml
git commit -m "feat(cardinal-xr): scaffold crate with constants and cardinal engine init"
```

---

## Task 2: Stardust Client Connection and Event Loop

**Files:**
- Modify: `cardinal-xr/src/main.rs`

**Reference:** See `/home/philpax/programming/stardustxr/molecules/examples/button.rs` and `/home/philpax/programming/stardustxr/asteroids/src/client.rs` for event loop patterns. The key pattern is `client.sync_event_loop()` which gives us frame callbacks.

- [ ] **Step 1: Update main.rs with Stardust client connection**

```rust
// cardinal-xr/src/main.rs
mod constants;

use std::sync::mpsc;
use cardinal_core::cardinal_thread::{Command, RenderResult};

fn main() {
    eprintln!("cardinal-xr: starting");

    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
    let _audio_stream = cardinal_core::audio::start_audio_stream();

    eprintln!("cardinal-xr: cardinal engine ready");

    // Initialize GPU for cardinal thread — we need a wgpu device.
    // For now, create a standalone wgpu instance (not tied to Stardust's GPU).
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("No Vulkan adapter found");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("cardinal-xr"),
            ..Default::default()
        },
    ))
    .expect("Failed to create wgpu device");

    let device = std::sync::Arc::new(device);
    let queue = std::sync::Arc::new(queue);

    cmd_tx
        .send(Command::InitGpu {
            device: device.clone(),
            queue: queue.clone(),
        })
        .expect("Failed to send InitGpu");

    eprintln!("cardinal-xr: GPU initialized");

    // Fetch module catalog
    let (cat_tx, cat_rx) = mpsc::channel();
    cmd_tx.send(Command::GetCatalog(cat_tx)).unwrap();
    let catalog = cat_rx.recv().unwrap();
    eprintln!("cardinal-xr: {} modules in catalog", catalog.len());

    // Connect to Stardust XR server
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let client = stardust_xr_fusion::client::Client::connect()
            .await
            .expect("Failed to connect to Stardust XR server");

        let root = client.get_root().clone();
        eprintln!("cardinal-xr: connected to Stardust XR server");

        client.sync_event_loop(|client, _flow| {
            while let Some(root_event) = client.get_root().recv_root_event() {
                match root_event {
                    stardust_xr_fusion::root::RootEvent::Ping { response } => {
                        response.send_ok(());
                    }
                    stardust_xr_fusion::root::RootEvent::Frame { info } => {
                        // TODO: per-frame update logic goes here
                    }
                    stardust_xr_fusion::root::RootEvent::SaveState { response } => {
                        response.send_ok(&());
                    }
                }
            }
        });
    });
}
```

Note: This step may require adding `wgpu`, `pollster` dependencies to Cargo.toml. Add them:

```toml
wgpu = "29"
pollster = "0.4"
```

- [ ] **Step 2: Verify it compiles and connects**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles. Running it without a Stardust server will panic at the connect step, which is expected. With a running Stardust server, it should connect and log success.

- [ ] **Step 3: Commit**

```bash
git add -A cardinal-xr/
git commit -m "feat(cardinal-xr): stardust client connection and event loop"
```

---

## Task 3: Math Utilities

**Files:**
- Create: `cardinal-xr/src/math.rs`

Pure functions for coordinate conversion and curve generation. These are testable without Stardust or Cardinal.

- [ ] **Step 1: Create math.rs with Bezier curve generation**

```rust
// cardinal-xr/src/math.rs
use glam::Vec3;
use crate::constants;

/// Generate points along a cubic Bezier cable curve between two 3D port positions.
/// The curve sags downward (negative Y) to simulate a hanging cable.
pub fn cable_bezier_points(start: Vec3, end: Vec3) -> Vec<Vec3> {
    let count = constants::CABLE_SEGMENT_COUNT;
    let mid_y_offset = -constants::CABLE_SAG_FACTOR * start.distance(end);
    let midpoint = (start + end) / 2.0 + Vec3::new(0.0, mid_y_offset, 0.0);

    // Control points: start, sag below midpoint, sag below midpoint, end
    let cp1 = Vec3::new(start.x, start.y + mid_y_offset * 0.5, (start.z + midpoint.z) / 2.0);
    let cp2 = Vec3::new(end.x, end.y + mid_y_offset * 0.5, (end.z + midpoint.z) / 2.0);

    let mut points = Vec::with_capacity(count + 1);
    for i in 0..=count {
        let t = i as f32 / count as f32;
        let it = 1.0 - t;
        let p = it * it * it * start
            + 3.0 * it * it * t * cp1
            + 3.0 * it * t * t * cp2
            + t * t * t * end;
        points.push(p);
    }
    points
}

/// Convert a widget's pixel position (from cardinal-core metadata) to
/// a 3D offset relative to the module panel's center.
/// Cardinal reports widget positions in pixels from the top-left corner.
/// The panel's center is at local origin (0, 0, 0).
/// X goes right, Y goes up, Z is forward (toward user).
pub fn pixel_to_panel_offset(
    pixel_x: f32,
    pixel_y: f32,
    module_width_px: f32,
    module_height_px: f32,
) -> Vec3 {
    let x = (pixel_x - module_width_px / 2.0) / constants::PIXELS_PER_METER;
    // Flip Y: pixel Y increases downward, world Y increases upward
    let y = (module_height_px / 2.0 - pixel_y) / constants::PIXELS_PER_METER;
    Vec3::new(x, y, 0.0)
}

/// Convert a 3D point in module-local space back to pixel coordinates.
/// Inverse of `pixel_to_panel_offset`.
pub fn panel_offset_to_pixel(
    offset: Vec3,
    module_width_px: f32,
    module_height_px: f32,
) -> (f32, f32) {
    let px = offset.x * constants::PIXELS_PER_METER + module_width_px / 2.0;
    let py = module_height_px / 2.0 - offset.y * constants::PIXELS_PER_METER;
    (px, py)
}

/// Exponential smoothing for position tracking (used for hand menu).
/// Returns the new smoothed value.
pub fn smooth(current: Vec3, target: Vec3, factor: f32) -> Vec3 {
    current.lerp(target, factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cable_bezier_correct_endpoints() {
        let start = Vec3::new(0.0, 1.0, -0.5);
        let end = Vec3::new(1.0, 1.0, -0.5);
        let points = cable_bezier_points(start, end);

        assert_eq!(points.len(), constants::CABLE_SEGMENT_COUNT + 1);
        assert!((points[0] - start).length() < 1e-5);
        assert!((points[constants::CABLE_SEGMENT_COUNT] - end).length() < 1e-5);
    }

    #[test]
    fn test_cable_bezier_sags_below_endpoints() {
        let start = Vec3::new(0.0, 1.0, 0.0);
        let end = Vec3::new(1.0, 1.0, 0.0);
        let points = cable_bezier_points(start, end);

        // Midpoint should be below the line connecting start and end
        let mid = &points[constants::CABLE_SEGMENT_COUNT / 2];
        assert!(mid.y < 1.0, "Cable midpoint should sag below endpoints");
    }

    #[test]
    fn test_pixel_to_panel_offset_center() {
        let offset = pixel_to_panel_offset(150.0, 200.0, 300.0, 400.0);
        assert!((offset.x).abs() < 1e-5, "Center pixel X should map to 0");
        assert!((offset.y).abs() < 1e-5, "Center pixel Y should map to 0");
    }

    #[test]
    fn test_pixel_to_panel_roundtrip() {
        let w = 300.0;
        let h = 400.0;
        let px = 75.0;
        let py = 100.0;
        let offset = pixel_to_panel_offset(px, py, w, h);
        let (rx, ry) = panel_offset_to_pixel(offset, w, h);
        assert!((rx - px).abs() < 1e-3);
        assert!((ry - py).abs() < 1e-3);
    }

    #[test]
    fn test_pixel_to_panel_top_left_is_positive_y() {
        // Top-left pixel (0,0) should map to negative X, positive Y
        let offset = pixel_to_panel_offset(0.0, 0.0, 300.0, 400.0);
        assert!(offset.x < 0.0, "Left side should be negative X");
        assert!(offset.y > 0.0, "Top should be positive Y");
    }
}
```

- [ ] **Step 2: Add math module to main.rs**

Add `mod math;` to `cardinal-xr/src/main.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p cardinal-xr -- --nocapture 2>&1 | tail -20`
Expected: All 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add cardinal-xr/src/math.rs cardinal-xr/src/main.rs
git commit -m "feat(cardinal-xr): math utilities for Bezier cables and coordinate conversion"
```

---

## Task 4: Panel glTF Asset

**Files:**
- Create: `cardinal-xr/assets/panel.glb`
- Create: `cardinal-xr/build_panel.py` (build script for the asset, can be deleted after)

We need a unit-sized box model: 1m wide × 1m tall × `PANEL_DEPTH_M` deep, with the front face (+Z) UV-mapped [0,0]-[1,1] for the module texture. The simplest approach is to generate it programmatically.

- [ ] **Step 1: Create a Python script to generate panel.glb**

This requires the `pygltflib` or `trimesh` package. Alternatively, use a minimal glTF JSON + binary approach. We'll use `trimesh` for simplicity:

```python
#!/usr/bin/env python3
"""Generate panel.glb — a unit box with front face UV-mapped for texturing."""
import struct
import json
import base64
import os

# Box: 1m x 1m x 0.008m (PANEL_DEPTH_M), centered at origin
# Front face is at z = +depth/2, facing +Z
# We only need 6 faces (12 triangles)
DEPTH = 0.008
hw, hh, hd = 0.5, 0.5, DEPTH / 2

# Vertices: position (3f), normal (3f), texcoord (2f)
# Front face (+Z): UV mapped [0,1] for module texture
# Other faces: UV (0,0) — they'll use the material base color
faces = {
    'front':  {'n': ( 0, 0, 1), 'verts': [(-hw,-hh, hd), ( hw,-hh, hd), ( hw, hh, hd), (-hw, hh, hd)],
               'uvs': [(0,0), (1,0), (1,1), (0,1)]},
    'back':   {'n': ( 0, 0,-1), 'verts': [( hw,-hh,-hd), (-hw,-hh,-hd), (-hw, hh,-hd), ( hw, hh,-hd)],
               'uvs': [(0,0), (0,0), (0,0), (0,0)]},
    'right':  {'n': ( 1, 0, 0), 'verts': [( hw,-hh, hd), ( hw,-hh,-hd), ( hw, hh,-hd), ( hw, hh, hd)],
               'uvs': [(0,0), (0,0), (0,0), (0,0)]},
    'left':   {'n': (-1, 0, 0), 'verts': [(-hw,-hh,-hd), (-hw,-hh, hd), (-hw, hh, hd), (-hw, hh,-hd)],
               'uvs': [(0,0), (0,0), (0,0), (0,0)]},
    'top':    {'n': ( 0, 1, 0), 'verts': [(-hw, hh, hd), ( hw, hh, hd), ( hw, hh,-hd), (-hw, hh,-hd)],
               'uvs': [(0,0), (0,0), (0,0), (0,0)]},
    'bottom': {'n': ( 0,-1, 0), 'verts': [(-hw,-hh,-hd), ( hw,-hh,-hd), ( hw,-hh, hd), (-hw,-hh, hd)],
               'uvs': [(0,0), (0,0), (0,0), (0,0)]},
}

positions = []
normals = []
texcoords = []
indices = []

for face in faces.values():
    base = len(positions)
    for v in face['verts']:
        positions.append(v)
        normals.append(face['n'])
    for uv in face['uvs']:
        texcoords.append(uv)
    # Two triangles per quad
    indices.extend([base, base+1, base+2, base, base+2, base+3])

# Pack binary data
pos_bin = b''.join(struct.pack('3f', *p) for p in positions)
norm_bin = b''.join(struct.pack('3f', *n) for n in normals)
uv_bin = b''.join(struct.pack('2f', *uv) for uv in texcoords)
idx_bin = b''.join(struct.pack('H', i) for i in indices)

# Pad each to 4-byte alignment
def pad4(b):
    r = len(b) % 4
    return b + b'\x00' * (4 - r) if r else b

idx_bin_padded = pad4(idx_bin)
buffer_data = idx_bin_padded + pos_bin + norm_bin + uv_bin

# Compute bounds
min_pos = [min(p[i] for p in positions) for i in range(3)]
max_pos = [max(p[i] for p in positions) for i in range(3)]

gltf = {
    "asset": {"version": "2.0", "generator": "cardinal-xr-panel-gen"},
    "scene": 0,
    "scenes": [{"nodes": [0]}],
    "nodes": [{"mesh": 0, "name": "Panel"}],
    "meshes": [{
        "primitives": [{
            "attributes": {"POSITION": 1, "NORMAL": 2, "TEXCOORD_0": 3},
            "indices": 0,
            "material": 0,
        }]
    }],
    "materials": [{
        "name": "PanelMaterial",
        "pbrMetallicRoughness": {
            "baseColorFactor": [0.15, 0.15, 0.15, 1.0],
            "metallicFactor": 0.0,
            "roughnessFactor": 0.8,
        },
    }],
    "accessors": [
        {"bufferView": 0, "componentType": 5123, "count": len(indices), "type": "SCALAR",
         "max": [max(indices)], "min": [min(indices)]},
        {"bufferView": 1, "componentType": 5126, "count": len(positions), "type": "VEC3",
         "max": max_pos, "min": min_pos},
        {"bufferView": 2, "componentType": 5126, "count": len(normals), "type": "VEC3"},
        {"bufferView": 3, "componentType": 5126, "count": len(texcoords), "type": "VEC2"},
    ],
    "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": len(idx_bin_padded), "target": 34963},
        {"buffer": 0, "byteOffset": len(idx_bin_padded), "byteLength": len(pos_bin), "target": 34962},
        {"buffer": 0, "byteOffset": len(idx_bin_padded) + len(pos_bin), "byteLength": len(norm_bin), "target": 34962},
        {"buffer": 0, "byteOffset": len(idx_bin_padded) + len(pos_bin) + len(norm_bin), "byteLength": len(uv_bin), "target": 34962},
    ],
    "buffers": [{"byteLength": len(buffer_data)}],
}

# Write GLB
json_str = json.dumps(gltf, separators=(',', ':'))
json_bin = json_str.encode('utf-8')
json_pad = b' ' * ((4 - len(json_bin) % 4) % 4)
json_chunk = json_bin + json_pad

bin_pad = b'\x00' * ((4 - len(buffer_data) % 4) % 4)
bin_chunk = buffer_data + bin_pad

total_length = 12 + 8 + len(json_chunk) + 8 + len(bin_chunk)

out_path = os.path.join(os.path.dirname(__file__), 'assets', 'panel.glb')
os.makedirs(os.path.dirname(out_path), exist_ok=True)
with open(out_path, 'wb') as f:
    # Header
    f.write(struct.pack('<4sII', b'glTF', 2, total_length))
    # JSON chunk
    f.write(struct.pack('<I4s', len(json_chunk), b'JSON'))
    f.write(json_chunk)
    # BIN chunk
    f.write(struct.pack('<I4s', len(bin_chunk), b'BIN\x00'))
    f.write(bin_chunk)

print(f"Wrote {out_path} ({total_length} bytes)")
```

- [ ] **Step 2: Run the script**

Run: `cd cardinal-xr && python3 build_panel.py`
Expected: `Wrote cardinal-xr/assets/panel.glb (XXX bytes)`

- [ ] **Step 3: Verify the GLB is valid**

If you have a glTF viewer available, open `panel.glb` to confirm it's a flat box. Otherwise, verify the file is non-empty and starts with the glTF magic bytes:

Run: `xxd cardinal-xr/assets/panel.glb | head -1`
Expected: Contains `676c 5446` (ASCII "glTF")

- [ ] **Step 4: Commit (include the .glb, optionally keep or delete the build script)**

```bash
git add cardinal-xr/assets/panel.glb cardinal-xr/build_panel.py
git commit -m "feat(cardinal-xr): panel slab glTF model (unit box, front face UV-mapped)"
```

---

## Task 5: DMA-BUF Texture Module

**Files:**
- Create: `cardinal-xr/src/dmatex.rs`
- Modify: `cardinal-xr/src/main.rs`
- Modify: `cardinal-xr/Cargo.toml`

This is the highest-risk task. We need to export wgpu textures as DMA-BUF fds and import them into Stardust. The exact Vulkan interop API depends on wgpu version 29's hal layer.

**Important:** This task may require significant iteration depending on wgpu's Vulkan hal API surface. The code below outlines the intended approach; the implementing engineer should consult `wgpu` v29 docs and the `ash` crate for exact Vulkan function signatures.

- [ ] **Step 1: Add dependencies**

Add to `cardinal-xr/Cargo.toml`:

```toml
ash = "0.38"
drm-fourcc = "2.2"
rustix = { version = "1.0", features = ["fs", "ioctl"] }
```

- [ ] **Step 2: Create dmatex.rs skeleton**

```rust
// cardinal-xr/src/dmatex.rs
//! DMA-BUF texture export from wgpu and import into Stardust XR.
//!
//! This module handles the zero-copy GPU texture sharing pipeline:
//! 1. Create wgpu textures with Vulkan external memory export capability
//! 2. Export the underlying VkImage as a DMA-BUF file descriptor
//! 3. Create DRM timeline syncobjs for GPU synchronization
//! 4. Import the DMA-BUF into Stardust via the dmatex protocol
//!
//! Fallback: If DMA-BUF export is unavailable, falls back to CPU readback.

use std::os::fd::{OwnedFd, AsRawFd, FromRawFd};
use std::sync::Arc;

/// Manages a double-buffered texture pair for one module.
pub struct ModuleTextures {
    /// The two alternating textures
    pub textures: [wgpu::Texture; 2],
    /// Which texture index is currently being written by Cardinal (0 or 1)
    pub write_index: usize,
    /// Stardust dmatex IDs for each texture
    pub dmatex_ids: [u64; 2],
    /// DMA-BUF fds (kept alive to prevent GC)
    _dmabuf_fds: [Option<OwnedFd>; 2],
    /// Current timeline sync point
    pub timeline_point: u64,
    /// Width and height in pixels
    pub width: u32,
    pub height: u32,
}

impl ModuleTextures {
    /// The texture that Cardinal should render to this frame.
    pub fn write_texture(&self) -> &wgpu::Texture {
        &self.textures[self.write_index]
    }

    /// The dmatex ID of the texture that Stardust should read this frame
    /// (the one we last finished writing to).
    pub fn read_dmatex_id(&self) -> u64 {
        self.dmatex_ids[1 - self.write_index]
    }

    /// Advance to the next frame: swap write/read buffers, bump timeline.
    pub fn swap(&mut self) {
        self.write_index = 1 - self.write_index;
        self.timeline_point += 1;
    }
}

/// Create a pair of wgpu textures suitable for DMA-BUF export.
///
/// This attempts to use Vulkan external memory. If that fails,
/// falls back to standard wgpu textures (for CPU readback path).
pub fn create_exportable_textures(
    device: &Arc<wgpu::Device>,
    width: u32,
    height: u32,
) -> [wgpu::Texture; 2] {
    // TODO: For DMA-BUF export, we need textures created with
    // VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT.
    // This requires using wgpu's hal::vulkan layer to access the raw
    // VkDevice and create VkImages with VkExternalMemoryImageCreateInfo.
    //
    // For now, create standard textures. The DMA-BUF export will be
    // implemented once we verify the wgpu hal API surface.
    let desc = wgpu::TextureDescriptor {
        label: Some("cardinal_module_texture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    [device.create_texture(&desc), device.create_texture(&desc)]
}

/// Attempt to export a wgpu texture as a DMA-BUF fd.
///
/// Returns `None` if the Vulkan backend doesn't support DMA-BUF export
/// or if the texture wasn't created with external memory flags.
pub fn export_dmabuf(
    _device: &Arc<wgpu::Device>,
    _texture: &wgpu::Texture,
) -> Option<DmaBufInfo> {
    // TODO: Implement Vulkan interop via wgpu's hal layer:
    // 1. device.as_hal::<wgpu::hal::vulkan::Api, _, _>(|hal_device| { ... })
    // 2. texture.as_hal::<wgpu::hal::vulkan::Api, _, _>(|hal_texture| { ... })
    // 3. Use ash to call vkGetMemoryFdKHR with VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT
    // 4. Return the fd, stride, offset, and DRM format modifier
    //
    // This is the highest-risk piece. If wgpu v29 doesn't expose sufficient
    // hal access, we fall back to CPU readback.
    eprintln!("dmatex: DMA-BUF export not yet implemented, will use CPU fallback");
    None
}

/// Info about an exported DMA-BUF.
pub struct DmaBufInfo {
    pub fd: OwnedFd,
    pub stride: u32,
    pub offset: u32,
    pub modifier: u64,
}

/// Create a DRM timeline syncobj for GPU synchronization.
///
/// Returns the syncobj fd, or None if DRM ioctls aren't available.
pub fn create_timeline_syncobj(_drm_fd: &OwnedFd) -> Option<OwnedFd> {
    // TODO: Use rustix DRM ioctls:
    // 1. Open /dev/dri/renderD128 (or appropriate render node)
    // 2. DRM_IOCTL_SYNCOBJ_CREATE with DRM_SYNCOBJ_CREATE_TYPE_TIMELINE
    // 3. DRM_IOCTL_SYNCOBJ_HANDLE_TO_FD to get the fd
    eprintln!("dmatex: timeline syncobj creation not yet implemented");
    None
}

/// Fallback: read texture pixels back to CPU via wgpu buffer mapping.
/// Returns RGBA8 pixel data.
pub fn cpu_readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let bytes_per_row = width * 4;
    // wgpu requires rows to be aligned to 256 bytes
    let padded_bytes_per_row = (bytes_per_row + 255) & !255;
    let buffer_size = (padded_bytes_per_row * height) as u64;

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback_staging"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let mapped = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    pixels
}
```

- [ ] **Step 3: Add module to main.rs**

Add `mod dmatex;` to `cardinal-xr/src/main.rs`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles. The DMA-BUF functions are stubs that log and return None — the actual Vulkan interop will be implemented iteratively once we can test against a running Stardust server.

- [ ] **Step 5: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): DMA-BUF texture module with CPU readback fallback"
```

---

## Task 6: Workspace Manager

**Files:**
- Create: `cardinal-xr/src/workspace.rs`
- Modify: `cardinal-xr/src/main.rs`

The workspace owns all module panels and cables, processes render results from the cardinal thread, and dispatches per-frame updates.

- [ ] **Step 1: Create workspace.rs**

```rust
// cardinal-xr/src/workspace.rs
//! Manages the collection of module panels and cables in 3D space.

use std::sync::mpsc;
use cardinal_core::cardinal_thread::{Command, RenderResult, ModuleInfo};
use cardinal_core::{ModuleId, CableId, CatalogEntry};
use rustc_hash::FxHashMap;

use crate::module_panel::ModulePanel;
use crate::cable::Cable;

pub struct Workspace {
    pub modules: FxHashMap<ModuleId, ModulePanel>,
    pub cables: FxHashMap<CableId, Cable>,
    pub catalog: Vec<CatalogEntry>,
    pub cmd_tx: mpsc::Sender<Command>,
    render_rx: mpsc::Receiver<RenderResult>,
    next_cable_color_idx: usize,
}

impl Workspace {
    pub fn new(
        catalog: Vec<CatalogEntry>,
        cmd_tx: mpsc::Sender<Command>,
        render_rx: mpsc::Receiver<RenderResult>,
    ) -> Self {
        Self {
            modules: FxHashMap::default(),
            cables: FxHashMap::default(),
            catalog,
            cmd_tx,
            render_rx,
            next_cable_color_idx: 0,
        }
    }

    /// Spawn a new module. Sends CreateModule to cardinal thread and sets up
    /// the 3D scene graph when the reply comes back.
    /// Returns the ModuleId if successful.
    pub fn spawn_module(
        &mut self,
        plugin: &str,
        model: &str,
        position: glam::Vec3,
        rotation: glam::Quat,
        // root: &stardust_xr_fusion::spatial::Spatial, // uncomment when wiring up
    ) -> Option<ModuleId> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::CreateModule {
                plugin: plugin.to_string(),
                model: model.to_string(),
                reply: reply_tx,
            })
            .ok()?;

        let info: ModuleInfo = reply_rx.recv().ok()?.as_ref()?.clone();

        // Request initial render
        let (w, h) = info.size;
        self.cmd_tx
            .send(Command::RenderModule {
                module_id: info.id,
                width: w as i32,
                height: h as i32,
            })
            .ok()?;

        // TODO: create ModulePanel scene graph (Task 7)
        eprintln!(
            "workspace: spawned module {:?} ({}x{}) at {:?}",
            info.id, w, h, position
        );

        Some(info.id)
    }

    /// Remove a module and all its connected cables.
    pub fn destroy_module(&mut self, id: ModuleId) {
        // Remove cables connected to this module
        let connected_cables: Vec<CableId> = self
            .cables
            .iter()
            .filter(|(_, c)| c.out_module == id || c.in_module == id)
            .map(|(id, _)| *id)
            .collect();

        for cable_id in connected_cables {
            self.destroy_cable(cable_id);
        }

        // Remove module panel
        if self.modules.remove(&id).is_some() {
            self.cmd_tx.send(Command::DestroyModule(id)).ok();
        }
    }

    /// Create a cable between two ports.
    pub fn create_cable(
        &mut self,
        out_mod: ModuleId,
        out_port: i32,
        in_mod: ModuleId,
        in_port: i32,
    ) -> Option<CableId> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::CreateCable {
                out_mod,
                out_port,
                in_mod,
                in_port,
                reply: reply_tx,
            })
            .ok()?;

        let cable_id = reply_rx.recv().ok()??;

        let color_idx = self.next_cable_color_idx;
        self.next_cable_color_idx =
            (self.next_cable_color_idx + 1) % crate::constants::CABLE_COLORS.len();

        let cable = Cable::new(cable_id, out_mod, out_port, in_mod, in_port, color_idx);
        self.cables.insert(cable_id, cable);

        Some(cable_id)
    }

    /// Remove a cable.
    pub fn destroy_cable(&mut self, id: CableId) {
        if self.cables.remove(&id).is_some() {
            cardinal_core::cable_destroy(id);
        }
    }

    /// Poll for completed render results from the cardinal thread.
    pub fn poll_render_results(&mut self) {
        while let Ok(result) = self.render_rx.try_recv() {
            if let Some(panel) = self.modules.get_mut(&result.module_id) {
                panel.on_render_result(result);
            }
        }
    }

    /// Per-frame update: poll renders, update cable geometry, etc.
    pub fn frame_update(&mut self) {
        self.poll_render_results();

        // TODO: update cable Lines geometry to track port world positions
        // TODO: update module texture streaming (dmatex)
    }
}
```

Note: `ModuleInfo` needs `Clone`. We'll need to derive it, or copy the fields. Check if it's already Clone — if not, we'll work around it by extracting the fields we need.

- [ ] **Step 2: Check if ModuleInfo is Clone**

Read `crates/cardinal-core/src/cardinal_thread.rs` line 56-61. If `ModuleInfo` doesn't derive `Clone`, change the spawn_module code to extract fields before the borrow ends:

```rust
let info = reply_rx.recv().ok()??;
let id = info.id;
let size = info.size;
let inputs = info.inputs;
let outputs = info.outputs;
```

- [ ] **Step 3: Create stub module_panel.rs and cable.rs**

Create `cardinal-xr/src/module_panel.rs`:

```rust
// cardinal-xr/src/module_panel.rs
//! A single module's 3D scene graph: panel model, interaction, resize, delete.

use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, PortInfo, ParamInfo};

pub struct ModulePanel {
    pub id: ModuleId,
    pub size_px: (f32, f32),
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub params: Vec<ParamInfo>,
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    // TODO: Stardust scene graph nodes (Model, Spatials, Fields, etc.)
}

impl ModulePanel {
    pub fn new(
        id: ModuleId,
        size_px: (f32, f32),
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
        position: glam::Vec3,
        rotation: glam::Quat,
    ) -> Self {
        let params = cardinal_core::module_params(id);
        Self {
            id,
            size_px,
            inputs,
            outputs,
            params,
            position,
            rotation,
        }
    }

    /// Called when a new render texture arrives from the cardinal thread.
    pub fn on_render_result(&mut self, _result: RenderResult) {
        // TODO: update dmatex / stream texture to Stardust model
    }

    /// World-space width in meters.
    pub fn width_m(&self) -> f32 {
        self.size_px.0 / crate::constants::PIXELS_PER_METER
    }

    /// World-space height in meters.
    pub fn height_m(&self) -> f32 {
        self.size_px.1 / crate::constants::PIXELS_PER_METER
    }
}
```

Create `cardinal-xr/src/cable.rs`:

```rust
// cardinal-xr/src/cable.rs
//! Cable rendering and lifecycle management.

use cardinal_core::{CableId, ModuleId};

pub struct Cable {
    pub id: CableId,
    pub out_module: ModuleId,
    pub out_port: i32,
    pub in_module: ModuleId,
    pub in_port: i32,
    pub color_idx: usize,
    // TODO: Lines drawable handle
}

impl Cable {
    pub fn new(
        id: CableId,
        out_module: ModuleId,
        out_port: i32,
        in_module: ModuleId,
        in_port: i32,
        color_idx: usize,
    ) -> Self {
        Self {
            id,
            out_module,
            out_port,
            in_module,
            in_port,
            color_idx,
        }
    }
}
```

- [ ] **Step 4: Wire workspace into main.rs**

Add module declarations and create the workspace in main.rs:

```rust
mod constants;
mod math;
mod dmatex;
mod workspace;
mod module_panel;
mod cable;
```

In the `main()` function, after fetching the catalog and before the Stardust event loop:

```rust
let mut workspace = workspace::Workspace::new(catalog, cmd_tx, render_rx);
```

Inside the `Frame` event handler:

```rust
stardust_xr_fusion::root::RootEvent::Frame { info } => {
    workspace.frame_update();
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): workspace manager with module spawn/destroy and cable tracking"
```

---

## Task 7: Module Panel Scene Graph

**Files:**
- Modify: `cardinal-xr/src/module_panel.rs`
- Modify: `cardinal-xr/src/workspace.rs`

Wire up the Stardust scene graph for each module: Spatial root, Model (panel slab), delete button. Interaction boxes and resize handles come in later tasks.

- [ ] **Step 1: Expand module_panel.rs with Stardust nodes**

This step depends on having the Stardust fusion API available. The exact types come from `stardust_xr_fusion`. Update `module_panel.rs`:

```rust
// cardinal-xr/src/module_panel.rs
use cardinal_core::cardinal_thread::RenderResult;
use cardinal_core::{ModuleId, PortInfo, ParamInfo};
use stardust_xr_fusion::drawable::Model as SxModel;
use stardust_xr_fusion::spatial::Spatial;
use stardust_xr_fusion::spatial::SpatialAspect;
use stardust_xr_fusion::resource::NamespacedResource;
use glam::{Vec3, Quat};
use mint;

use crate::constants::*;

pub struct ModulePanel {
    pub id: ModuleId,
    pub size_px: (f32, f32),
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub params: Vec<ParamInfo>,

    // Stardust scene graph
    pub root: Spatial,
    pub model: SxModel,
    // TODO: interaction boxes, resize handles, delete button
}

impl ModulePanel {
    pub fn new(
        id: ModuleId,
        size_px: (f32, f32),
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
        parent: &impl SpatialAspect,
        position: Vec3,
        rotation: Quat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let params = cardinal_core::module_params(id);

        let width_m = size_px.0 / PIXELS_PER_METER;
        let height_m = size_px.1 / PIXELS_PER_METER;

        // Create root spatial at the desired world position
        let transform = stardust_xr_fusion::spatial::Transform::from_translation_rotation(
            mint::Vector3::from(position.to_array()),
            mint::Quaternion::from(rotation.to_array()),
        );
        let root = Spatial::create(parent, transform, false)?;

        // Load panel model, scaled to match module dimensions
        // The panel.glb is a 1m×1m×PANEL_DEPTH_M box
        let resource = NamespacedResource::new("cardinal-xr", "panel");
        let model_transform =
            stardust_xr_fusion::spatial::Transform::from_scale(mint::Vector3::from([
                width_m, height_m, 1.0,
            ]));
        let model = SxModel::create(&root, model_transform, &resource)?;

        Ok(Self {
            id,
            size_px,
            inputs,
            outputs,
            params,
            root,
            model,
        })
    }

    pub fn on_render_result(&mut self, _result: RenderResult) {
        // TODO: update dmatex texture on the model
    }

    pub fn width_m(&self) -> f32 {
        self.size_px.0 / PIXELS_PER_METER
    }

    pub fn height_m(&self) -> f32 {
        self.size_px.1 / PIXELS_PER_METER
    }
}
```

**Note:** The exact Stardust API calls (`Spatial::create`, `Model::create`, `Transform::from_*`, `NamespacedResource`) need to match fusion v0.51.0's actual signatures. The implementing engineer should check the generated API. The pattern above follows what we observed in flatland and molecules examples.

- [ ] **Step 2: Update workspace.rs spawn_module to create ModulePanel**

Replace the TODO in `spawn_module`:

```rust
pub fn spawn_module(
    &mut self,
    plugin: &str,
    model: &str,
    position: glam::Vec3,
    rotation: glam::Quat,
    parent: &impl stardust_xr_fusion::spatial::SpatialAspect,
) -> Option<ModuleId> {
    let (reply_tx, reply_rx) = mpsc::channel();
    self.cmd_tx
        .send(Command::CreateModule {
            plugin: plugin.to_string(),
            model: model.to_string(),
            reply: reply_tx,
        })
        .ok()?;

    let info = reply_rx.recv().ok()??;
    let id = info.id;
    let size = info.size;
    let inputs = info.inputs;
    let outputs = info.outputs;

    // Request initial render
    self.cmd_tx
        .send(Command::RenderModule {
            module_id: id,
            width: size.0 as i32,
            height: size.1 as i32,
        })
        .ok()?;

    let panel = ModulePanel::new(id, size, inputs, outputs, parent, position, rotation);
    match panel {
        Ok(panel) => {
            self.modules.insert(id, panel);
            Some(id)
        }
        Err(e) => {
            eprintln!("workspace: failed to create panel for {:?}: {}", id, e);
            self.cmd_tx.send(Command::DestroyModule(id)).ok();
            None
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -10`
Expected: Compiles. May require adjusting Stardust API calls based on actual fusion v0.51.0 signatures.

- [ ] **Step 4: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): module panel scene graph with Stardust spatial + model"
```

---

## Task 8: Interaction Boxes and Input Forwarding

**Files:**
- Create: `cardinal-xr/src/interaction.rs`
- Modify: `cardinal-xr/src/module_panel.rs`

Create per-widget interaction boxes with hover feedback and input forwarding to cardinal-core. Also create the fallback touch plane.

- [ ] **Step 1: Create interaction.rs**

```rust
// cardinal-xr/src/interaction.rs
//! Per-widget interaction boxes and fallback touch plane.
//! Each port and param gets a 3D box that highlights on hover
//! and forwards pinch gestures as cardinal-core events.

use std::sync::mpsc;
use std::sync::Arc;
use cardinal_core::cardinal_thread::{Command, EventResult};
use cardinal_core::{ModuleId, PortInfo, ParamInfo};
use glam::Vec3;

use crate::constants::*;
use crate::math;

/// Identifies what kind of widget an interaction box represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    InputPort { port_id: i32 },
    OutputPort { port_id: i32 },
    Param { param_id: i32 },
}

impl WidgetKind {
    pub fn is_port(&self) -> bool {
        matches!(self, WidgetKind::InputPort { .. } | WidgetKind::OutputPort { .. })
    }

    pub fn port_id(&self) -> Option<i32> {
        match self {
            WidgetKind::InputPort { port_id } | WidgetKind::OutputPort { port_id } => {
                Some(*port_id)
            }
            _ => None,
        }
    }

    pub fn is_output(&self) -> bool {
        matches!(self, WidgetKind::OutputPort { .. })
    }
}

/// Data for one interaction box (port or param).
pub struct InteractionBox {
    pub kind: WidgetKind,
    /// Position in module pixel coordinates
    pub pixel_x: f32,
    pub pixel_y: f32,
    /// Position as 3D offset from module panel center
    pub panel_offset: Vec3,
    /// Whether currently being hovered
    pub hovered: bool,
    // TODO: Stardust Field + InputHandler + Lines (for visual box)
}

/// Build interaction boxes for all ports and params on a module.
pub fn build_interaction_boxes(
    inputs: &[PortInfo],
    outputs: &[PortInfo],
    params: &[ParamInfo],
    module_width_px: f32,
    module_height_px: f32,
) -> Vec<InteractionBox> {
    let mut boxes = Vec::new();

    for port in inputs {
        let offset = math::pixel_to_panel_offset(
            port.x, port.y, module_width_px, module_height_px,
        );
        boxes.push(InteractionBox {
            kind: WidgetKind::InputPort { port_id: port.id },
            pixel_x: port.x,
            pixel_y: port.y,
            panel_offset: offset,
            hovered: false,
        });
    }

    for port in outputs {
        let offset = math::pixel_to_panel_offset(
            port.x, port.y, module_width_px, module_height_px,
        );
        boxes.push(InteractionBox {
            kind: WidgetKind::OutputPort { port_id: port.id },
            pixel_x: port.x,
            pixel_y: port.y,
            panel_offset: offset,
            hovered: false,
        });
    }

    for param in params {
        let offset = math::pixel_to_panel_offset(
            param.x, param.y, module_width_px, module_height_px,
        );
        boxes.push(InteractionBox {
            kind: WidgetKind::Param { param_id: param.id },
            pixel_x: param.x,
            pixel_y: param.y,
            panel_offset: offset,
            hovered: false,
        });
    }

    boxes
}

/// Send a button event to cardinal-core and wait for the result.
pub fn send_button_event(
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
    x: f32,
    y: f32,
    action: i32,
    mods: i32,
) -> Option<EventResult> {
    let (reply_tx, reply_rx) = mpsc::channel();
    cmd_tx
        .send(Command::ModuleEvent {
            module_id,
            event_type: cardinal_core::EVENT_BUTTON,
            x,
            y,
            button: 0,
            action,
            mods,
            scroll_x: 0.0,
            scroll_y: 0.0,
            reply: Some(reply_tx),
        })
        .ok()?;
    reply_rx.recv().ok()
}

/// Send a hover event to cardinal-core (fire-and-forget).
pub fn send_hover_event(
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
    x: f32,
    y: f32,
) {
    let _ = cmd_tx.send(Command::ModuleEvent {
        module_id,
        event_type: cardinal_core::EVENT_HOVER,
        x,
        y,
        button: 0,
        action: 0,
        mods: 0,
        scroll_x: 0.0,
        scroll_y: 0.0,
        reply: None,
    });
}

/// Send a scroll event to cardinal-core (fire-and-forget).
pub fn send_scroll_event(
    cmd_tx: &mpsc::Sender<Command>,
    module_id: ModuleId,
    x: f32,
    y: f32,
    scroll_x: f32,
    scroll_y: f32,
) {
    let _ = cmd_tx.send(Command::ModuleEvent {
        module_id,
        event_type: cardinal_core::EVENT_SCROLL,
        x,
        y,
        button: 0,
        action: 0,
        mods: 0,
        scroll_x,
        scroll_y,
        reply: None,
    });
}
```

- [ ] **Step 2: Add module to main.rs**

Add `mod interaction;` to `cardinal-xr/src/main.rs`.

- [ ] **Step 3: Wire interaction boxes into ModulePanel::new()**

In `module_panel.rs`, add to the struct:

```rust
pub interaction_boxes: Vec<crate::interaction::InteractionBox>,
```

In the constructor, after creating the model:

```rust
let interaction_boxes = crate::interaction::build_interaction_boxes(
    &inputs, &outputs, &params, size_px.0, size_px.1,
);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): interaction boxes with per-widget positioning and input forwarding"
```

---

## Task 9: Cable Rendering

**Files:**
- Modify: `cardinal-xr/src/cable.rs`
- Modify: `cardinal-xr/src/workspace.rs`

Implement cable rendering using Stardust Lines drawables and the Bezier curve math from Task 3.

- [ ] **Step 1: Expand cable.rs with Lines rendering**

```rust
// cardinal-xr/src/cable.rs
use cardinal_core::{CableId, ModuleId};
use glam::{Vec3, Vec4};

use crate::constants::*;
use crate::math;

/// State machine for cable creation via port dragging.
pub enum CableDragState {
    /// No drag in progress
    Idle,
    /// Dragging from a port — preview cable follows hand
    Dragging {
        from_module: ModuleId,
        from_port: i32,
        is_output: bool,
        /// Current hand position in world space
        hand_pos: Vec3,
    },
}

pub struct Cable {
    pub id: CableId,
    pub out_module: ModuleId,
    pub out_port: i32,
    pub in_module: ModuleId,
    pub in_port: i32,
    pub color_idx: usize,
    // TODO: stardust_xr_fusion::drawable::Lines handle
}

impl Cable {
    pub fn new(
        id: CableId,
        out_module: ModuleId,
        out_port: i32,
        in_module: ModuleId,
        in_port: i32,
        color_idx: usize,
    ) -> Self {
        Self {
            id,
            out_module,
            out_port,
            in_module,
            in_port,
            color_idx,
        }
    }

    pub fn color(&self) -> Vec4 {
        CABLE_COLORS[self.color_idx % CABLE_COLORS.len()]
    }

    /// Compute the current cable curve points given the world-space
    /// positions of both connected ports.
    pub fn compute_points(&self, out_pos: Vec3, in_pos: Vec3) -> Vec<Vec3> {
        math::cable_bezier_points(out_pos, in_pos)
    }
}

/// Compute a preview cable from a port to the user's hand position.
pub fn preview_cable_points(port_pos: Vec3, hand_pos: Vec3) -> Vec<Vec3> {
    math::cable_bezier_points(port_pos, hand_pos)
}
```

- [ ] **Step 2: Add CableDragState to Workspace**

In `workspace.rs`, add:

```rust
use crate::cable::CableDragState;
```

Add field to `Workspace`:

```rust
pub cable_drag: CableDragState,
```

Initialize in `new()`:

```rust
cable_drag: CableDragState::Idle,
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): cable rendering with Bezier curves and drag state machine"
```

---

## Task 10: Hand Menu State and Catalog Processing

**Files:**
- Create: `cardinal-xr/src/hand_menu.rs`
- Modify: `cardinal-xr/src/main.rs`

Build the hand menu state management and catalog grouping logic. This is the pure-logic part that we can test without Stardust.

**Important:** `CatalogEntry` currently lacks `tags`. For now, we group by plugin only. Task 14 addresses adding tag support to cardinal-core's FFI. The menu structure supports tags but falls back to showing all modules under each plugin when tags aren't available.

- [ ] **Step 1: Create hand_menu.rs**

```rust
// cardinal-xr/src/hand_menu.rs
//! Palm-anchored hand menu for browsing and spawning modules.
//!
//! State management and catalog processing logic.
//! The Stardust scene graph (asteroids Reify) will be added in Task 11.

use cardinal_core::CatalogEntry;
use rustc_hash::FxHashMap;
use glam::Vec3;

use crate::constants::*;

/// A group of modules belonging to the same plugin.
#[derive(Debug, Clone)]
pub struct PluginGroup {
    pub plugin_slug: String,
    pub display_name: String,
    pub modules: Vec<ModuleEntry>,
}

/// A single module within a plugin group.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    pub model_slug: String,
    pub display_name: String,
    pub plugin_slug: String,
    /// Tags for this module (empty until tag support is added to cardinal-core)
    pub tags: Vec<String>,
}

/// Identifies a level in the cascading menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuLevel {
    Tag,
    Plugin,
    Module,
}

pub struct HandMenuState {
    /// All plugins, grouped and sorted
    pub all_plugins: Vec<PluginGroup>,
    /// Unique tags extracted from catalog (empty until tag support added)
    pub tags: Vec<String>,
    /// Currently selected tag index (None = "All")
    pub selected_tag: Option<usize>,
    /// Plugins filtered by the selected tag
    pub filtered_plugins: Vec<PluginGroup>,
    /// Currently selected plugin index within filtered_plugins
    pub selected_plugin: Option<usize>,
    /// Modules for the selected plugin (filtered by tag)
    pub filtered_modules: Vec<ModuleEntry>,
    /// Per-entry hover timers (seconds)
    pub hover_timers: FxHashMap<(MenuLevel, usize), f32>,
    /// Whether the menu is visible
    pub visible: bool,
    /// Smoothed menu world position
    pub smoothed_position: Vec3,
    /// Scroll offsets per level
    pub scroll_offsets: FxHashMap<MenuLevel, usize>,
}

impl HandMenuState {
    /// Build menu state from the cardinal-core catalog.
    pub fn from_catalog(catalog: &[CatalogEntry]) -> Self {
        let mut plugin_map: FxHashMap<String, Vec<ModuleEntry>> = FxHashMap::default();

        for entry in catalog {
            let modules = plugin_map
                .entry(entry.plugin_slug.clone())
                .or_default();
            modules.push(ModuleEntry {
                model_slug: entry.model_slug.clone(),
                display_name: entry.model_name.clone(),
                plugin_slug: entry.plugin_slug.clone(),
                tags: Vec::new(), // No tags yet
            });
        }

        let mut all_plugins: Vec<PluginGroup> = plugin_map
            .into_iter()
            .map(|(slug, mut modules)| {
                modules.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                PluginGroup {
                    display_name: slug.clone(),
                    plugin_slug: slug,
                    modules,
                }
            })
            .collect();
        all_plugins.sort_by(|a, b| a.display_name.cmp(&b.display_name));

        // Extract unique tags (empty for now)
        let tags = Vec::new();

        let filtered_plugins = all_plugins.clone();

        Self {
            all_plugins,
            tags,
            selected_tag: None,
            filtered_plugins,
            selected_plugin: None,
            filtered_modules: Vec::new(),
            hover_timers: FxHashMap::default(),
            visible: false,
            smoothed_position: Vec3::ZERO,
            scroll_offsets: FxHashMap::default(),
        }
    }

    /// Check if the left palm is facing up (Y component of palm normal).
    /// Updates visibility with hysteresis.
    pub fn update_palm_visibility(&mut self, palm_up_amount: f32) {
        if self.visible {
            if palm_up_amount < MENU_PALM_DOWN_THRESHOLD {
                self.close();
            }
        } else if palm_up_amount > MENU_PALM_UP_THRESHOLD {
            self.visible = true;
        }
    }

    /// Close the menu and reset all state.
    pub fn close(&mut self) {
        self.visible = false;
        self.selected_tag = None;
        self.selected_plugin = None;
        self.filtered_modules.clear();
        self.hover_timers.clear();
        self.scroll_offsets.clear();
        self.refilter_plugins();
    }

    /// Select a tag (or None for "All"), refilter plugins.
    pub fn select_tag(&mut self, tag_idx: Option<usize>) {
        self.selected_tag = tag_idx;
        self.selected_plugin = None;
        self.filtered_modules.clear();
        self.refilter_plugins();
    }

    /// Select a plugin, populate filtered_modules.
    pub fn select_plugin(&mut self, plugin_idx: Option<usize>) {
        self.selected_plugin = plugin_idx;
        self.filtered_modules.clear();

        if let Some(idx) = plugin_idx {
            if let Some(plugin) = self.filtered_plugins.get(idx) {
                // If a tag is selected, filter modules by that tag
                // For now (no tags), show all modules
                self.filtered_modules = if let Some(_tag_idx) = self.selected_tag {
                    // TODO: filter by tag when tags are available
                    plugin.modules.clone()
                } else {
                    plugin.modules.clone()
                };
            }
        }
    }

    /// Update hover timer for an entry. Returns true if the timer just exceeded the threshold.
    pub fn update_hover(&mut self, level: MenuLevel, index: usize, dt: f32) -> bool {
        let key = (level, index);
        let timer = self.hover_timers.entry(key).or_insert(0.0);
        let was_below = *timer < MENU_HOVER_EXPAND_DELAY_SECS;
        *timer += dt;
        was_below && *timer >= MENU_HOVER_EXPAND_DELAY_SECS
    }

    /// Reset hover timer for an entry.
    pub fn reset_hover(&mut self, level: MenuLevel, index: usize) {
        self.hover_timers.remove(&(level, index));
    }

    /// Reset all hover timers for a given menu level.
    pub fn reset_level_hovers(&mut self, level: MenuLevel) {
        self.hover_timers.retain(|(l, _), _| *l != level);
    }

    fn refilter_plugins(&mut self) {
        // When tags are available, filter by selected tag.
        // For now, show all plugins.
        self.filtered_plugins = self.all_plugins.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardinal_core::CatalogEntry;

    fn sample_catalog() -> Vec<CatalogEntry> {
        vec![
            CatalogEntry {
                plugin_slug: "Fundamental".into(),
                model_slug: "VCO".into(),
                model_name: "VCO".into(),
            },
            CatalogEntry {
                plugin_slug: "Fundamental".into(),
                model_slug: "VCF".into(),
                model_name: "VCF".into(),
            },
            CatalogEntry {
                plugin_slug: "Befaco".into(),
                model_slug: "Mixer".into(),
                model_name: "Mixer".into(),
            },
        ]
    }

    #[test]
    fn test_catalog_grouping() {
        let state = HandMenuState::from_catalog(&sample_catalog());
        assert_eq!(state.all_plugins.len(), 2);
        // Sorted alphabetically: Befaco before Fundamental
        assert_eq!(state.all_plugins[0].plugin_slug, "Befaco");
        assert_eq!(state.all_plugins[1].plugin_slug, "Fundamental");
        assert_eq!(state.all_plugins[1].modules.len(), 2);
    }

    #[test]
    fn test_plugin_selection_populates_modules() {
        let mut state = HandMenuState::from_catalog(&sample_catalog());
        state.select_plugin(Some(1)); // Fundamental
        assert_eq!(state.filtered_modules.len(), 2);
        // Sorted: VCF before VCO
        assert_eq!(state.filtered_modules[0].display_name, "VCF");
        assert_eq!(state.filtered_modules[1].display_name, "VCO");
    }

    #[test]
    fn test_palm_hysteresis() {
        let mut state = HandMenuState::from_catalog(&sample_catalog());
        assert!(!state.visible);

        // Below open threshold — stays hidden
        state.update_palm_visibility(0.6);
        assert!(!state.visible);

        // Above open threshold — opens
        state.update_palm_visibility(0.8);
        assert!(state.visible);

        // Above close threshold — stays open (hysteresis)
        state.update_palm_visibility(0.6);
        assert!(state.visible);

        // Below close threshold — closes
        state.update_palm_visibility(0.4);
        assert!(!state.visible);
    }

    #[test]
    fn test_hover_timer() {
        let mut state = HandMenuState::from_catalog(&sample_catalog());

        // Accumulate below threshold
        let triggered = state.update_hover(MenuLevel::Plugin, 0, 0.1);
        assert!(!triggered);

        // Accumulate past threshold
        let triggered = state.update_hover(MenuLevel::Plugin, 0, 0.25);
        assert!(triggered);

        // Already past — no re-trigger
        let triggered = state.update_hover(MenuLevel::Plugin, 0, 0.1);
        assert!(!triggered);

        // Reset
        state.reset_hover(MenuLevel::Plugin, 0);
        let triggered = state.update_hover(MenuLevel::Plugin, 0, 0.1);
        assert!(!triggered);
    }

    #[test]
    fn test_close_resets_state() {
        let mut state = HandMenuState::from_catalog(&sample_catalog());
        state.visible = true;
        state.select_tag(None);
        state.select_plugin(Some(0));
        assert!(!state.filtered_modules.is_empty());

        state.close();
        assert!(!state.visible);
        assert!(state.selected_tag.is_none());
        assert!(state.selected_plugin.is_none());
        assert!(state.filtered_modules.is_empty());
    }
}
```

- [ ] **Step 2: Add module to main.rs**

Add `mod hand_menu;` to `cardinal-xr/src/main.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p cardinal-xr -- --nocapture 2>&1 | tail -20`
Expected: All tests pass (math tests + hand menu tests).

- [ ] **Step 4: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): hand menu state management with catalog grouping and hover timers"
```

---

## Task 11: Hand Menu Asteroids UI

**Files:**
- Modify: `cardinal-xr/src/hand_menu.rs`
- Modify: `cardinal-xr/src/main.rs`

Implement the asteroids `Reify` trait for the hand menu, producing the three-column cascading layout. This wires the state from Task 10 into a Stardust scene graph.

- [ ] **Step 1: Add Reify implementation**

This depends heavily on the asteroids API. The implementing engineer should follow the pattern from `/home/philpax/programming/stardustxr/protostar/sirius/src/main.rs` and `/home/philpax/programming/stardustxr/asteroids/examples/basic_layout.rs`.

Add to `hand_menu.rs`:

```rust
use stardust_xr_asteroids::{Element, Reify, ValidState, context::Context, task::Tasker};
use stardust_xr_asteroids::elements::{
    spatial::Spatial as SxSpatial,
    text::Text as SxText,
    button::Button as SxButton,
};

impl Reify for HandMenuState {
    fn reify(&self, context: &Context, tasks: impl Tasker<Self>) -> impl Element<Self> {
        SxSpatial::default()
            .pos([0.0, MENU_PALM_OFFSET_M, 0.0])
            .build()
            .maybe_child(self.visible.then(|| {
                SxSpatial::default()
                    .build()
                    // Left column: tags (or "All" when no tags)
                    .child(self.reify_tag_column(context, tasks.clone()))
                    // Middle column: filtered plugins
                    .maybe_child(self.selected_tag.is_some().then(|| {
                        self.reify_plugin_column(context, tasks.clone())
                    }))
                    // Also show plugins when no tags exist (skip tag column)
                    .maybe_child((self.tags.is_empty() && self.selected_tag.is_none()).then(|| {
                        self.reify_plugin_column_no_tags(context, tasks.clone())
                    }))
                    // Right column: modules
                    .maybe_child(self.selected_plugin.is_some().then(|| {
                        self.reify_module_column(context, tasks.clone())
                    }))
            }))
    }
}

impl HandMenuState {
    fn reify_tag_column(
        &self,
        _context: &Context,
        _tasks: impl Tasker<Self>,
    ) -> impl Element<Self> {
        // If no tags available, show a single "All" entry that goes straight to plugins
        if self.tags.is_empty() {
            return SxSpatial::default().build();
        }

        // Tag entries stacked vertically
        SxSpatial::default()
            .build()
            .stable_children(
                std::iter::once(("All".to_string(), {
                    SxButton::new(|state: &mut HandMenuState| {
                        state.select_tag(None);
                    })
                    .size([MENU_ITEM_WIDTH_M, MENU_ITEM_HEIGHT_M])
                    .build()
                    .child(
                        SxText::new("All")
                            .character_height(MENU_ITEM_HEIGHT_M * 0.6)
                            .build(),
                    )
                }))
                .chain(self.tags.iter().enumerate().map(|(i, tag)| {
                    (tag.clone(), {
                        let y = -((i + 1) as f32) * (MENU_ITEM_HEIGHT_M + 0.002);
                        SxButton::new(move |state: &mut HandMenuState| {
                            state.select_tag(Some(i));
                        })
                        .size([MENU_ITEM_WIDTH_M, MENU_ITEM_HEIGHT_M])
                        .pos([0.0, y, 0.0])
                        .build()
                        .child(
                            SxText::new(tag)
                                .character_height(MENU_ITEM_HEIGHT_M * 0.6)
                                .build(),
                        )
                    })
                })),
            )
    }

    fn reify_plugin_column_no_tags(
        &self,
        _context: &Context,
        _tasks: impl Tasker<Self>,
    ) -> impl Element<Self> {
        self.reify_plugin_list(0.0)
    }

    fn reify_plugin_column(
        &self,
        _context: &Context,
        _tasks: impl Tasker<Self>,
    ) -> impl Element<Self> {
        let x_offset = MENU_ITEM_WIDTH_M + MENU_COLUMN_GAP_M;
        self.reify_plugin_list(x_offset)
    }

    fn reify_plugin_list(&self, x_offset: f32) -> impl Element<Self> {
        let scroll = self.scroll_offsets.get(&MenuLevel::Plugin).copied().unwrap_or(0);
        let visible_plugins = self.filtered_plugins.iter().enumerate()
            .skip(scroll)
            .take(MENU_MAX_VISIBLE_ITEMS);

        SxSpatial::default()
            .pos([x_offset, 0.0, 0.0])
            .build()
            .stable_children(
                visible_plugins.map(|(i, plugin)| {
                    let y = -((i - scroll) as f32) * (MENU_ITEM_HEIGHT_M + 0.002);
                    (plugin.plugin_slug.clone(), {
                        SxButton::new(move |state: &mut HandMenuState| {
                            if state.selected_plugin == Some(i) {
                                state.select_plugin(None);
                            } else {
                                state.select_plugin(Some(i));
                            }
                        })
                        .size([MENU_ITEM_WIDTH_M, MENU_ITEM_HEIGHT_M])
                        .pos([0.0, y, 0.0])
                        .build()
                        .child(
                            SxText::new(&plugin.display_name)
                                .character_height(MENU_ITEM_HEIGHT_M * 0.6)
                                .build(),
                        )
                    })
                }),
            )
    }

    fn reify_module_column(
        &self,
        _context: &Context,
        _tasks: impl Tasker<Self>,
    ) -> impl Element<Self> {
        let x_offset = if self.tags.is_empty() {
            MENU_ITEM_WIDTH_M + MENU_COLUMN_GAP_M
        } else {
            2.0 * (MENU_ITEM_WIDTH_M + MENU_COLUMN_GAP_M)
        };

        // Align vertically with the selected plugin entry
        let plugin_y = self.selected_plugin.map_or(0.0, |i| {
            let scroll = self.scroll_offsets.get(&MenuLevel::Plugin).copied().unwrap_or(0);
            -((i - scroll) as f32) * (MENU_ITEM_HEIGHT_M + 0.002)
        });

        SxSpatial::default()
            .pos([x_offset, plugin_y, 0.0])
            .build()
            .stable_children(
                self.filtered_modules.iter().enumerate().map(|(i, module)| {
                    let y = -(i as f32) * (MENU_ITEM_HEIGHT_M + 0.002);
                    let plugin_slug = module.plugin_slug.clone();
                    let model_slug = module.model_slug.clone();
                    (format!("{}:{}", plugin_slug, model_slug), {
                        SxButton::new(move |state: &mut HandMenuState| {
                            // Module selected — the main loop will check for this
                            // and call workspace.spawn_module()
                            // For now, just log it
                            eprintln!(
                                "hand_menu: spawn {}:{}",
                                plugin_slug, model_slug
                            );
                        })
                        .size([MENU_ITEM_WIDTH_M, MENU_ITEM_HEIGHT_M])
                        .pos([0.0, y, 0.0])
                        .build()
                        .child(
                            SxText::new(&module.display_name)
                                .character_height(MENU_ITEM_HEIGHT_M * 0.6)
                                .build(),
                        )
                    })
                }),
            )
    }
}
```

**Note:** The asteroids API types (`Element`, `Reify`, `Tasker`, `Button::new`, `Text::new`, `Spatial::default()`) must match the actual v0.51.0 signatures. The implementing engineer should check imports and adapt as needed. The pattern follows `protostar/sirius` and `asteroids/examples/basic_layout.rs`.

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -10`
Expected: Compiles (may require import adjustments).

- [ ] **Step 3: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): hand menu asteroids Reify implementation with cascading columns"
```

---

## Task 12: Wire Everything Together in Main Loop

**Files:**
- Modify: `cardinal-xr/src/main.rs`

Connect all subsystems: workspace, hand menu, and the Stardust event loop. Spawn a test module to verify the pipeline works end-to-end.

- [ ] **Step 1: Update main.rs with full event loop**

```rust
// cardinal-xr/src/main.rs
mod cable;
mod constants;
mod dmatex;
mod hand_menu;
mod interaction;
mod math;
mod module_panel;
mod workspace;

use std::sync::mpsc;
use cardinal_core::cardinal_thread::Command;

fn main() {
    eprintln!("cardinal-xr: starting");

    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
    let _audio_stream = cardinal_core::audio::start_audio_stream();

    // GPU init
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("No Vulkan adapter found");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("cardinal-xr"),
            ..Default::default()
        },
    ))
    .expect("Failed to create wgpu device");

    let device = std::sync::Arc::new(device);
    let queue = std::sync::Arc::new(queue);

    cmd_tx
        .send(Command::InitGpu {
            device: device.clone(),
            queue: queue.clone(),
        })
        .expect("Failed to send InitGpu");

    // Fetch catalog
    let (cat_tx, cat_rx) = mpsc::channel();
    cmd_tx.send(Command::GetCatalog(cat_tx)).unwrap();
    let catalog = cat_rx.recv().unwrap();
    eprintln!("cardinal-xr: {} modules in catalog", catalog.len());

    // Build hand menu state
    let hand_menu_state = hand_menu::HandMenuState::from_catalog(&catalog);

    // Create workspace
    let mut workspace = workspace::Workspace::new(catalog, cmd_tx.clone(), render_rx);

    // Connect to Stardust
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let client = stardust_xr_fusion::client::Client::connect()
            .await
            .expect("Failed to connect to Stardust XR server");

        let _root = client.get_root().clone();
        eprintln!("cardinal-xr: connected to Stardust XR");

        // TODO: spawn test module for debugging:
        // workspace.spawn_module("Fundamental", "VCO-1", Vec3::new(0.0, 1.0, -0.5), Quat::IDENTITY, &root);

        client.sync_event_loop(|client, _flow| {
            while let Some(root_event) = client.get_root().recv_root_event() {
                match root_event {
                    stardust_xr_fusion::root::RootEvent::Ping { response } => {
                        response.send_ok(());
                    }
                    stardust_xr_fusion::root::RootEvent::Frame { info } => {
                        // 1. Poll render results from cardinal thread
                        workspace.frame_update();

                        // 2. Hand menu updates would go here:
                        // - Read left palm state
                        // - Update hand_menu_state.update_palm_visibility()
                        // - Update hover timers
                        // - projector.frame() / projector.update()

                        // 3. Cable geometry updates would go here
                    }
                    stardust_xr_fusion::root::RootEvent::SaveState { response } => {
                        response.send_ok(&());
                    }
                }
            }
        });
    });
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): wire all subsystems into main event loop"
```

---

## Task 13: Module Moving, Resizing, and Deletion

**Files:**
- Modify: `cardinal-xr/src/module_panel.rs`
- Modify: `cardinal-xr/src/workspace.rs`

Add the grabbable (for moving), corner resize handles, and delete button to each module panel. These use molecules' `Grabbable` and `Button` types.

- [ ] **Step 1: Add moving, resize, and delete scaffolding to module_panel.rs**

The implementing engineer should follow the patterns from:
- `/home/philpax/programming/stardustxr/flatland/src/grab_ball.rs` for grabbable panels
- `/home/philpax/programming/stardustxr/flatland/src/resize_handles.rs` for resize
- `/home/philpax/programming/stardustxr/molecules/examples/button.rs` for delete button

Key additions to `ModulePanel`:

```rust
// In ModulePanel struct, add:
// pub grabbable: stardust_xr_molecules::Grabbable,
// pub resize_handles: [ResizeHandle; 4],  // four corners
// pub delete_button: stardust_xr_molecules::button::Button,

// ResizeHandle wraps a small sphere Grabbable at each corner.
// When dragged, compute new panel size maintaining aspect ratio.
pub struct ResizeHandle {
    // TODO: Grabbable with sphere field at corner position
    pub corner: Corner,
}

#[derive(Debug, Clone, Copy)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    /// Returns the corner position relative to panel center, given panel size in meters.
    pub fn offset(&self, width_m: f32, height_m: f32) -> glam::Vec3 {
        let hw = width_m / 2.0;
        let hh = height_m / 2.0;
        match self {
            Corner::TopLeft => glam::Vec3::new(-hw, hh, 0.0),
            Corner::TopRight => glam::Vec3::new(hw, hh, 0.0),
            Corner::BottomLeft => glam::Vec3::new(-hw, -hh, 0.0),
            Corner::BottomRight => glam::Vec3::new(hw, -hh, 0.0),
        }
    }
}
```

The full implementation involves creating Stardust Fields, InputHandlers, and Grabbables for each corner and the panel body. The exact API calls depend on molecules v0.51.0. The implementing engineer should:

1. Create a `Field` (sphere shape, radius `RESIZE_HANDLE_RADIUS_M`) at each corner
2. Create a `Grabbable` wrapping the panel's root spatial with a box field covering the panel face
3. Create a `Button` (size `DELETE_BUTTON_SIZE_M`) offset from the top-right corner
4. In the frame loop, check `grabbable.grab_action().actor_started/stopped` for move state
5. Check each resize handle for grab, compute aspect-ratio-locked new size
6. Check delete button for press, call `workspace.destroy_module()`

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p cardinal-xr 2>&1 | tail -5`

- [ ] **Step 3: Commit**

```bash
git add cardinal-xr/
git commit -m "feat(cardinal-xr): module moving, corner resize handles, and delete button"
```

---

## Task 14: Add Tag Support to Cardinal-Core (Optional Enhancement)

**Files:**
- Modify: `crates/cardinal-core/src/ffi.rs`
- Modify: `crates/cardinal-core/src/lib.rs`
- Modify: `cardinal-xr/src/hand_menu.rs`

The hand menu design uses tags for filtering, but `CatalogEntry` currently only has `plugin_slug`, `model_slug`, `model_name`. This task adds tag extraction from the C++ plugin metadata.

**Note:** This requires changes to the C++ FFI bridge. If the C++ side doesn't expose tags, this task should be deferred — the hand menu already works with plugin-only grouping.

- [ ] **Step 1: Check if the C++ side exposes model tags**

Search the Cardinal C++ code for tag-related fields in the `Model` or `Plugin` structures:

Run: `grep -r "tags\|tagIds\|model.*tag" ../Cardinal/include/ ../Cardinal/src/ 2>/dev/null | head -20`

If tags are available in the C++ model metadata, add them to `ModuleCatalogEntry` in `ffi.rs`:

```rust
#[repr(C)]
#[derive(Clone)]
pub struct ModuleCatalogEntry {
    pub plugin_slug: *const c_char,
    pub model_slug: *const c_char,
    pub model_name: *const c_char,
    pub tags: *const *const c_char,  // null-terminated array of tag strings
    pub tag_count: i32,
}
```

And update `CatalogEntry` in `lib.rs`:

```rust
pub struct CatalogEntry {
    pub plugin_slug: String,
    pub model_slug: String,
    pub model_name: String,
    pub tags: Vec<String>,
}
```

- [ ] **Step 2: Update hand_menu.rs to use tags**

In `HandMenuState::from_catalog`, populate the `tags` field on each `ModuleEntry` from `CatalogEntry::tags`. Build the unique tag list. Update `refilter_plugins` to actually filter by tag.

- [ ] **Step 3: Run tests**

Run: `cargo test -p cardinal-xr -- --nocapture 2>&1 | tail -20`
Expected: All tests still pass. Add new tests for tag filtering if tags are available.

- [ ] **Step 4: Commit**

```bash
git add crates/cardinal-core/ cardinal-xr/
git commit -m "feat(cardinal-core): add tag support to catalog entries for hand menu filtering"
```

---

## Task 15: Integration Testing and Polish

**Files:**
- Modify: `cardinal-xr/src/main.rs`
- Modify: various files as needed

End-to-end testing with a running Stardust XR server and VR headset.

- [ ] **Step 1: Manual integration test checklist**

Run: `cargo run -p cardinal-xr`

With a Stardust XR server running, verify:

1. [ ] Client connects to Stardust without errors
2. [ ] Cardinal engine initializes (check log output)
3. [ ] Audio output works (if audio module is created)
4. [ ] Panel model appears in 3D space (even without texture — should show dark slab)
5. [ ] Interaction boxes are positioned correctly over ports/params
6. [ ] Hover feedback highlights boxes when hand enters
7. [ ] Pinch on a knob sends events and the module re-renders with updated state
8. [ ] Cable creation: pinch output port → drag → release on input port → cable appears
9. [ ] Cable follows port positions when module is moved
10. [ ] Module deletion via X button removes panel and cables
11. [ ] Hand menu appears when left palm faces up
12. [ ] Menu closes when palm faces down
13. [ ] Plugin list is browsable, module items spawn modules

- [ ] **Step 2: Fix issues found during integration testing**

Address any API mismatches, coordinate system issues, or timing problems discovered during testing.

- [ ] **Step 3: Commit fixes**

```bash
git add -A cardinal-xr/
git commit -m "fix(cardinal-xr): integration testing fixes"
```

---

## Summary

| Task | Component | Testable Without Stardust? |
|------|-----------|---------------------------|
| 1 | Crate skeleton + constants | Yes (compiles) |
| 2 | Stardust client connection | No (needs server) |
| 3 | Math utilities | Yes (unit tests) |
| 4 | Panel glTF asset | Yes (file generation) |
| 5 | DMA-BUF texture module | Partial (CPU fallback testable) |
| 6 | Workspace manager | Yes (compiles, logic) |
| 7 | Module panel scene graph | No (needs Stardust) |
| 8 | Interaction boxes | Partial (box building logic testable) |
| 9 | Cable rendering | Yes (curve math tested in Task 3) |
| 10 | Hand menu state | Yes (unit tests) |
| 11 | Hand menu asteroids UI | No (needs Stardust) |
| 12 | Main loop wiring | No (needs Stardust) |
| 13 | Moving/resize/delete | No (needs Stardust) |
| 14 | Tag support (optional) | Depends on C++ FFI |
| 15 | Integration testing | No (needs full stack) |
