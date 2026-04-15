# cardinal-xr Design Spec

A Stardust XR client for Cardinal, displaying VCV Rack modules as interactive 3D panels in mixed reality.

## Architecture

### Crate Structure

New workspace member `cardinal-xr/` depending on:

- **cardinal-core** — engine, module lifecycle, NanoVG rendering, audio
- **stardust-xr-fusion** — Stardust client protocol, spatial primitives
- **stardust-xr-molecules** — input handling, grabbables, buttons, touch planes
- **stardust-xr-asteroids** — declarative UI for the hand menu

### Thread Architecture

- **Cardinal thread** (existing) — owns the Rack engine, processes `Command` messages, renders module textures via NanoVG to wgpu. Single-threaded access to Rack engine, communicates via `mpsc` channels.
- **Stardust client thread** (main) — runs the Stardust `sync_event_loop`, manages the 3D scene graph, forwards input events to the cardinal thread via `Command` sender.
- **Audio thread** (existing) — cpal audio callback invoking `audio_process()`, unchanged from cardinal-egui.

Communication uses the same `mpsc::Sender<Command>` / `mpsc::Receiver<RenderResult>` channel pattern as cardinal-egui. RenderResults carry DMA-BUF-exportable wgpu textures.

### Hybrid UI Paradigm

Two subsystems with different frameworks:

- **Rack workspace** (imperative, fusion + molecules) — manages module panels, cables, interaction boxes, and the DMA-BUF texture pipeline. Requires tight control over per-frame texture streaming, spatial positioning, and input forwarding.
- **Hand menu** (declarative, asteroids) — uses the `Reify` trait to define menu UI as a function of catalog state. Categories, plugins, and modules expand/collapse based on state changes. Natural fit for asteroids' diffing model.

## DMA-BUF Texture Pipeline

Zero-copy GPU texture sharing between Cardinal's wgpu renderer and Stardust XR's Bevy/Vulkan compositor.

### Setup (once per module)

1. Cardinal thread creates a wgpu texture for the module (existing behavior).
2. Export the texture's underlying Vulkan image as a DMA-BUF fd using wgpu's `hal::vulkan` layer and `VK_EXT_external_memory_dma_buf`.
3. Create a DRM timeline syncobj for GPU synchronization.
4. Call `import_dmatex` on the Stardust client to register the texture with an ID.
5. Load the panel slab glTF model and apply the dmatex to its `base_color_texture` material slot via `set_material_parameter` with a `DmatexSubmitInfo`.

### Per-Frame Streaming

1. Cardinal thread renders the module via NanoVG to the wgpu texture.
2. Signal the timeline syncobj's acquire point (tells Stardust "I'm done writing").
3. Call `set_material_parameter` with a new `DmatexSubmitInfo` containing current acquire/release points.
4. Stardust's server waits on the acquire point, composites the texture, then signals the release point.
5. Cardinal thread waits on the release point before rendering to that texture again.

### Double-Buffering

Two textures per module, alternated each frame. Cardinal renders to texture A while Stardust reads texture B, and vice versa. Timeline sync points coordinate this without CPU stalls.

### Platform Requirements

- Linux only (Stardust XR is Linux-only).
- Vulkan backend required for wgpu (for DMA-BUF export).
- Textures must be created with Vulkan external memory export flags. This may require creating textures through wgpu's hal layer rather than the safe API.

### Fallback

If DMA-BUF export proves impractical, fall back to CPU readback: `wgpu::Buffer::map_async` to read pixels, write to a shared-memory DMA-BUF. Adds a GPU-to-CPU-to-GPU round trip but is functionally correct.

## Module Panels

### Visual Appearance

Modules are displayed as extruded slab panels — a flat box with slight depth. The module texture is on the front face; sides and back use a neutral material.

- Panel model: a swappable glTF asset (`panel.glb`), not hardcoded geometry. The model is a unit-sized box (1m x 1m x `PANEL_DEPTH_M`) with the front face UV-mapped [0,0]-[1,1]. At runtime, the model is non-uniformly scaled to match each module's world-space dimensions (width and height from module pixel size / `PIXELS_PER_METER`). Depth remains constant.
- Panel dimensions derived from module pixel size and `PIXELS_PER_METER` constant.
- Depth controlled by `PANEL_DEPTH_M` constant.

### Scene Graph (per module)

```
Spatial (module root — world position/rotation)
├── Model (panel slab with dmatex on front face)
├── Spatial (interaction layer, offset forward by INTERACTION_LAYER_OFFSET_M)
│   ├── [per port] InputHandler + Field (box, protruding from surface)
│   ├── [per param] InputHandler + Field (box, protruding from surface)
│   └── InputHandler + Field (fallback full-module touch plane, behind boxes)
├── Button (delete "X" affordance, top-right corner)
├── [4x] Grabbable (corner resize handles)
└── Grabbable (whole panel body — for moving)
```

### Spawning

- Modules spawn at the right hand's pointing position, at `MODULE_SPAWN_DISTANCE_M` along the ray.
- Oriented to face the user's HMD at spawn time, then fixed in place.
- Freeform positioning — no grid or snap alignment.

### Moving

- Grabbing the panel body (not a corner, not an interaction box) moves the whole module.
- Uses molecules `Grabbable` with momentum for natural feel.

### Resizing

- Four corner grabbables (small spheres at panel corners, radius `RESIZE_HANDLE_RADIUS_M`).
- Grabbing a corner and moving scales the panel while maintaining aspect ratio.
- The opposite corner stays anchored.
- New pixel dimensions trigger a re-render at updated resolution.

### Deletion

- A small "X" button affordance near the top-right corner of the panel.
- Implemented as a molecules `Button` with touch plane.
- Pinching it sends `DestroyModule` to cardinal-core and removes all scene graph nodes, cables, and dmatex resources for that module.

## Interaction Model

### Per-Widget Interaction Boxes

Every port and param reported by cardinal-core gets a dedicated 3D interaction box:

- Positioned using x,y metadata from `module_inputs()`, `module_outputs()`, and `module_params()`, projected onto the panel face.
- Offset forward from the panel surface by `INTERACTION_BOX_PROTRUSION_M`.
- Minimum size `INTERACTION_BOX_MIN_SIZE_M` per side (comfortable finger target in VR).
- Each box is a `Field` (box shape) with an `InputHandler`.

**Port boxes** and **param boxes** use distinct highlight colors so users can tell them apart at a glance.

### Hover Feedback

When a hand enters an interaction box's field:

- Box transitions from nearly transparent to a visible highlight (soft glow / color shift).
- Port boxes: `PORT_HOVER_COLOR` (e.g. a blue-ish tint).
- Param boxes: `PARAM_HOVER_COLOR` (e.g. an orange-ish tint).
- Transition controlled by `HOVER_HIGHLIGHT_OPACITY_IDLE` and `HOVER_HIGHLIGHT_OPACITY_ACTIVE`.

### Input Forwarding

- **Pinch = click**: Pinch gesture maps to `EVENT_BUTTON` (action=1) at the widget's x,y coordinates. Release maps to action=0.
- **Pinch + drag**: After initial pinch, hand movement is tracked and forwarded as `EVENT_HOVER` updates with updated coordinates. Release sends `EVENT_BUTTON` action=0. Cardinal's internal widget system handles value computation for knobs, sliders, etc.
- **Scroll**: If Stardust exposes scroll data from input devices, forward as `EVENT_SCROLL`.

### Fallback Touch Plane

Behind all interaction boxes, a full-module touch plane covers the entire panel face:

- Catches any interaction that misses dedicated boxes.
- Converts 3D touch point to 2D module-local coordinates.
- Forwards as standard mouse events to cardinal-core.
- Handles custom widgets that aren't reported in port/param metadata.

Priority: dedicated boxes take precedence (being physically closer to the user). Touches that miss all boxes pass through to the fallback plane.

## Cables

### Visual Representation

- Rendered as Stardust `Lines` drawables.
- Path: 3D catenary/Bezier curve between port world positions, approximated with `CABLE_SEGMENT_COUNT` line segments.
- Color-coded per cable from a palette (`CABLE_COLORS`), matching VCV Rack's aesthetic.
- Thickness: `CABLE_THICKNESS_M`.

### Cable Creation

1. User pinches a port's interaction box.
2. `EVENT_BUTTON` sent to cardinal-core → returns `EventResult { consumed: true, port_drag: Some(PortDragInfo) }`.
3. `SetIncompleteCable` sent to cardinal-core (enables port highlighting in rendered texture).
4. Preview cable (Lines drawable) created, endpoint follows hand position each frame.
5. On release:
   - If hand is inside a compatible port's interaction box (output↔input): send `CreateCable` → receive `CableId` → create permanent Lines drawable.
   - Otherwise: cancel, send `ClearIncompleteCable`.

### Cable Tracking

Each cable stores:

- `CableId` (from cardinal-core)
- Output module ID + port ID
- Input module ID + port ID
- `Lines` drawable handle
- Color

Cable geometry updates each frame to track port world positions (since modules can be moved).

### Cable Deletion

Pinching an occupied port initiates a drag of the existing cable off that port. Cardinal-core internally disconnects the old cable when the port drag starts. Releasing over empty space completes the deletion; releasing over another compatible port re-routes the cable.

If cardinal-core's internal cable state diverges from our tracking (e.g. a cable was implicitly destroyed during a port drag), we detect this by checking whether a `CableId` we hold was invalidated. Cardinal-core doesn't currently expose a cable listing API, so we track divergence locally: when a port drag starts on an occupied port, we mark the cable connected to that port as "pending deletion" and remove it from our list if no new cable is created on release. If more robust sync is needed later, we can add a `cable_list()` API to cardinal-core.

On module deletion, all connected cables are also destroyed.

## Hand Menu

### Trigger

- Monitor left hand palm normal each frame.
- Menu opens when palm normal Y component exceeds `MENU_PALM_UP_THRESHOLD` (palm facing up).
- Menu closes when Y component drops below `MENU_PALM_DOWN_THRESHOLD` (hysteresis prevents flicker).
- Closing the menu resets all expansion state.

### Positioning

- Anchored to the left palm spatial.
- Hovers above the palm, offset by `MENU_PALM_OFFSET_M`.
- Position smoothed with a low-pass filter (`MENU_POSITION_SMOOTHING`) to reduce hand tracking jitter.

### Layout

Three-level cascading menu expanding to the right:

```
Left column: Tag categories (vertical)
    → expands right to:
    Middle column: Plugins matching that tag (vertical)
        → expands right to:
        Right column: Modules in that plugin matching that tag (vertical)
```

Each level's submenu is vertically aligned with the selected entry in the previous column.

### Tag Categories

- Derived from `tags` field in `CatalogEntry` returned by cardinal-core's `catalog()`.
- An "All" tag at the top shows all plugins.
- Tags sorted alphabetically.

### Plugin Filtering

- Selecting a tag filters the plugin list to only plugins containing at least one module with that tag.
- Plugins sorted alphabetically within the filtered list.
- If the filtered list exceeds `MENU_MAX_VISIBLE_ITEMS`, the list is scrollable with up/down navigation buttons.

### Module List

- Selecting a plugin shows its modules that match the currently selected tag.
- If "All" tag is selected, all modules in the plugin are shown.
- Modules sorted alphabetically.

### Interaction

- **Hover-to-expand**: Right hand finger enters an entry's zone → timer increments. After `MENU_HOVER_EXPAND_DELAY_MS`, the submenu expands. Moving to a different entry resets the timer.
- **Press-to-toggle**: Pressing an entry immediately toggles its submenu. Pressing another entry at the same level closes the previous one.
- Both mechanisms work at all three levels.

### Module Spawning

Pressing a module entry in the right column:

1. Sends `CreateModule` to cardinal-core with the plugin and model slugs.
2. Module panel spawns at the right hand's pointing ray, `MODULE_SPAWN_DISTANCE_M` along the ray.
3. Panel faces the user's HMD.
4. Hand menu remains open for spawning additional modules.

### Declarative State (asteroids)

```rust
struct PluginGroup {
    plugin_slug: String,
    display_name: String,
    modules: Vec<ModuleEntry>,
}

struct ModuleEntry {
    model_slug: String,
    display_name: String,
    tags: Vec<String>,
}

enum MenuLevel { Tag, Plugin, Module }

struct HandMenuState {
    catalog: Vec<CatalogEntry>,         // raw catalog from cardinal-core
    tags: Vec<String>,                  // unique tags, sorted alphabetically
    selected_tag: Option<usize>,        // index into tags
    plugins_for_tag: Vec<PluginGroup>,  // filtered by selected tag
    selected_plugin: Option<usize>,     // index into plugins_for_tag
    modules_for_plugin: Vec<ModuleEntry>, // filtered by tag + plugin
    hover_timers: HashMap<(MenuLevel, usize), f32>,
    menu_visible: bool,
    scroll_offsets: HashMap<MenuLevel, usize>,
}
```

## Configuration Constants

All tunable values defined in a single `constants.rs` module:

### Panel

| Constant | Description | Default |
|----------|-------------|---------|
| `PIXELS_PER_METER` | Module pixel-to-world scale | 3000.0 |
| `PANEL_DEPTH_M` | Panel slab thickness | 0.008 |
| `PANEL_SIDE_COLOR` | Side/back face color | dark gray |

### Interaction

| Constant | Description | Default |
|----------|-------------|---------|
| `INTERACTION_BOX_MIN_SIZE_M` | Minimum interaction box side length | 0.015 |
| `INTERACTION_BOX_PROTRUSION_M` | How far boxes extend from panel face | 0.005 |
| `INTERACTION_LAYER_OFFSET_M` | Interaction layer offset from panel | 0.001 |
| `PORT_HOVER_COLOR` | Port box highlight color | blue-ish |
| `PARAM_HOVER_COLOR` | Param box highlight color | orange-ish |
| `HOVER_HIGHLIGHT_OPACITY_IDLE` | Box opacity when not hovered | 0.1 |
| `HOVER_HIGHLIGHT_OPACITY_ACTIVE` | Box opacity when hovered | 0.6 |

### Resize

| Constant | Description | Default |
|----------|-------------|---------|
| `RESIZE_HANDLE_RADIUS_M` | Corner grab sphere radius | 0.01 |

### Cables

| Constant | Description | Default |
|----------|-------------|---------|
| `CABLE_THICKNESS_M` | Cable line thickness | 0.003 |
| `CABLE_SEGMENT_COUNT` | Line segments per cable curve | 20 |
| `CABLE_SAG_FACTOR` | Catenary sag amount | 0.05 |
| `CABLE_COLORS` | Color palette for cables | [red, blue, green, yellow, purple, ...] |

### Hand Menu

| Constant | Description | Default |
|----------|-------------|---------|
| `MENU_PALM_UP_THRESHOLD` | Palm Y normal to open menu | 0.7 |
| `MENU_PALM_DOWN_THRESHOLD` | Palm Y normal to close menu | 0.5 |
| `MENU_PALM_OFFSET_M` | Menu height above palm | 0.05 |
| `MENU_POSITION_SMOOTHING` | Low-pass filter factor (0-1) | 0.3 |
| `MENU_HOVER_EXPAND_DELAY_MS` | Hover time before submenu expands | 300 |
| `MENU_MAX_VISIBLE_ITEMS` | Max entries before scrolling | 10 |
| `MENU_ITEM_HEIGHT_M` | Height of each menu entry | 0.025 |
| `MENU_ITEM_WIDTH_M` | Width of each menu entry | 0.08 |
| `MENU_COLUMN_GAP_M` | Horizontal gap between columns | 0.01 |

### Module Spawning

| Constant | Description | Default |
|----------|-------------|---------|
| `MODULE_SPAWN_DISTANCE_M` | Distance along pointing ray | 0.5 |

### Delete Button

| Constant | Description | Default |
|----------|-------------|---------|
| `DELETE_BUTTON_SIZE_M` | Delete button side length | 0.015 |
| `DELETE_BUTTON_OFFSET_M` | Offset from panel top-right corner | 0.01 |

## Stardust XR API Notes

Findings from researching the Stardust XR ecosystem for this design. Intended to be useful for Stardust developers and future complex client authors.

### DMA-BUF Texture Import (dmatex)

Stardust XR supports importing GPU textures from external processes via DMA-BUF file descriptors with DRM timeline syncobj synchronization. This is the mechanism for streaming rendered content into the scene graph.

**Client-side API** (from fusion's generated protocol):

- `import_dmatex(client, dmatex_id, size, format, drm_format_modifier, srgb, array_layers, planes, timeline_syncobj_fd)` — registers an external DMA-BUF texture.
- Textures are applied to model surfaces via `set_material_parameter(name, MaterialParameter::Dmatex(DmatexSubmitInfo { dmatex_id, acquire_point, release_point }))`.
- 105 DRM FourCC formats supported (RGBA, BGRA, YUV variants, etc.).

**Synchronization model**: Client signals `acquire_point` when done writing to the texture. Server waits on acquire, renders, then signals `release_point` via `SignalOnDrop`. Client waits on release before writing again.

**What works well**:
- Zero-copy GPU-to-GPU texture sharing.
- Timeline syncobj model enables double-buffering without CPU stalls.
- Material parameter system allows swapping textures at runtime.

**Gaps / rough edges for this use case**:
- No built-in "textured quad" drawable. To display a texture, you must load a glTF model and apply the dmatex to a material slot. A simple `create_textured_quad(size, dmatex_id)` API would simplify many use cases.
- DMA-BUF export from wgpu requires dropping to the Vulkan hal layer. There's no wgpu-native path for this. Client authors need Vulkan interop knowledge.
- DRM syncobj creation requires direct kernel ioctl calls or the `drm` crate. No Stardust helper for this.
- No documentation or examples of the dmatex workflow from the client side. The protocol definition exists but the end-to-end pattern (create texture → export fd → import → apply → synchronize → stream) is undocumented.
- `import_dmatex` is a one-shot signal, not a method on a node. It's unclear how to unregister/replace a dmatex cleanly when a module is deleted.

### Drawable Types

Stardust provides:
- **Lines** — multi-point polylines with per-vertex color and thickness. Used for cable rendering and debug visualization.
- **Model** — glTF model loading with material parameter overrides and dmatex texture application.
- **Text** — 3D text with font, alignment, bounds, and color control.
- **SkyTexture / SkyLight** — environment lighting.

**Missing for this use case**:
- No primitive quad/plane drawable with texture support. Every textured surface requires a glTF model.
- No dynamic mesh creation (vertex buffer upload). Lines are the only dynamic geometry primitive.

### Input System

Molecules provides a layered input system:
- `InputQueue` → receives raw input events (hand, pointer, tip).
- `SingleAction` / `MultiAction` → tracks interaction state with debouncing.
- `TouchPlane` / `HoverPlane` → 2D interaction surfaces with coordinate mapping.
- `Button` → press/release detection with visual feedback.
- `Grabbable` → physics-based grab with momentum, multiple pointer modes.

**Works well for this use case**:
- `InputData` provides hand joint positions (thumb tip, index tip, palm) and `pinch_strength` in the datamap.
- `SingleAction` cleanly models the pinch-to-interact pattern.
- `Grabbable` handles module moving with momentum out of the box.
- `Field` shapes (box, sphere) map naturally to interaction volumes.

**Considerations**:
- Hand tracking palm normal for menu trigger requires raw access to `InputData::Hand` palm rotation, which is available through the input handler.
- Coordinate conversion from 3D hand position to 2D module-local coordinates needs careful transform math (world → module-local → pixel space).

### Asteroids Declarative Framework

Used for the hand menu subsystem.

**Works well**:
- `Reify` trait cleanly models menu state → UI tree.
- `stable_children` with string keys handles dynamic filtered lists (tags, plugins, modules) with stable identity.
- `Button` element integrates press callbacks directly.
- `Text` element handles labels.
- `Spatial` element handles positioning of columns and entries.

**Considerations**:
- Asteroids' `Projector` runs inside `sync_event_loop`. The hand menu state updates (palm detection, hover timers) need to happen in the frame callback before `projector.update()`.
- The menu needs access to the left hand's spatial data each frame, which comes from the Stardust input system, not from asteroids' element tree. The `ClientState::on_frame()` callback is the right place to read hand state and update menu visibility/timers.

### General Ecosystem Notes

- Stardust XR is Linux-only, targeting OpenXR runtimes (Monado, WiVRn, etc.).
- The client protocol is socket-based (not GPU shared). All scene graph manipulation happens via message passing.
- The server uses Bevy 0.16 with custom patched wgpu. Client wgpu versions may differ — texture format compatibility should be verified.
- 66 Cardinal plugins produce a large module catalog. UI must handle hundreds of entries gracefully.
- The `molecules` and `asteroids` crates are at version 0.51.0, matching the server. API stability is not guaranteed across versions.
