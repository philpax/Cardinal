# Input Forwarding and Audio I/O Widget Design

## Problem

Module widgets render visually but are non-interactive — knobs don't turn, buttons don't click, lights don't respond to hover. The egui overlay has its own interaction layer (hit-testing port/knob positions from metadata), but this is visually disconnected from the rendered widgets and doesn't cover the full range of widget interactions.

Additionally, the Audio I/O module is a special case with no widget — it renders as a gray rectangle with port indicators in the egui overlay, has its own spawn command, and its own button in the browser. It should be a normal module with a proper rendered widget.

## Solution

1. **Forward mouse events from egui into Rack's widget tree** so modules handle their own interaction natively (knobs, buttons, switches, ports all work through Rack's existing event system).
2. **Intercept port drags** to manage cables in the egui layer (not Rack's RackWidget), preserving our per-module rendering model for future 3D/XR use.
3. **Give Audio I/O a proper ModuleWidget** using Cardinal's HostAudio SVG panel, registered as a normal catalog module.

## Key Design Decisions

1. **Inject events directly into ModuleWidget** — skip Scene/RackWidget event chain. Each module is a self-contained interactive unit. This supports the future where modules exist as independent objects in 3D space.
2. **Cables stay in egui** — port drag interactions are intercepted before Rack creates CableWidgets. Cable rendering and management remain in our layer since cables are a spatial concern (will span 3D space between modules).
3. **Continuous re-rendering** — all modules re-render every frame to keep lights, meters, oscilloscopes, and other animated displays up to date.
4. **Audio I/O becomes a normal module** — registered in the catalog with a proper widget, no special-case UI code.

## Architecture

### Input Forwarding

```
egui WindowEvent (mouse move/click/scroll)
  -> Check which module rect contains the mouse
  -> Translate to module-local coordinates
  -> Bridge: cardinal_module_event(handle, event_type, x, y, button, mods)
  -> C++ constructs Rack event (HoverEvent, ButtonEvent, DragMoveEvent, etc.)
  -> Calls moduleWidget->onHover(e) / onButton(e) / etc.
  -> Widget tree handles natively
  -> Module re-renders next frame
```

### Event Mapping

| egui event | Rack event | Notes |
|---|---|---|
| Mouse move | `HoverEvent` | Position update, hover highlights |
| Mouse press | `ButtonEvent` (action=press) | Starts knob drags, button clicks |
| Mouse release | `ButtonEvent` (action=release) | Completes interactions |
| Mouse drag | `DragMoveEvent` | Knob value changes |
| Mouse scroll | `ScrollEvent` | Fine knob adjustment |
| Mouse leave module | `LeaveEvent` | Clears hover state |

### Events NOT forwarded to Rack

| Event | Reason |
|---|---|
| Port drag start/end | Cables managed by egui layer |
| Module-level drag | Module positioning is egui's concern |
| Right-click context menu | Rack menus need a real window — skip for now |

### Drag State Ownership

Track `active_drag: Option<ModuleId>` in the egui App. On mouse press over a module, set it. All subsequent move/release events go to that module until mouse release clears it. This ensures dragging a knob continues working even if the mouse leaves the module rect.

### Port Drag Interception (Cables)

After forwarding a ButtonEvent (press) to a ModuleWidget, check if Rack's EventState set `draggedWidget` to a PortWidget:

1. Record which port (module, port_id, is_output) started the drag
2. Clear Rack's drag state so it doesn't create a CableWidget
3. Switch to egui cable-dragging mode (yellow preview line)
4. On mouse release over another module's port, complete via `cable_create`

Bridge API:
```c
// After forwarding a button-press event, check if a port was clicked.
// Returns 1 if a port drag started, filling out the port info.
// Clears Rack's drag state to prevent CableWidget creation.
int cardinal_module_check_port_drag(ModuleHandle h, int* port_id, int* is_output);
```

### Bridge API Additions

```c
// Forward a mouse/input event to a module's widget tree.
// event_type: CARDINAL_EVENT_HOVER, _BUTTON, _DRAG_MOVE, _SCROLL, _LEAVE
// button: 0=left, 1=right, 2=middle
// action: 0=release, 1=press (for BUTTON events)
// mods: bitmask (shift=1, ctrl=2, alt=4, super=8)
// Returns 1 if the event was consumed by the widget.
int cardinal_module_event(ModuleHandle h, int event_type,
                          float x, float y,
                          int button, int action, int mods,
                          float scroll_x, float scroll_y);

// Check if the last button-press started a port drag.
int cardinal_module_check_port_drag(ModuleHandle h, int* port_id, int* is_output);
```

### What Gets Removed from egui

- `find_knob_at` / knob hit-testing
- `find_port_at` / port hit-testing
- `DragState::Knob` and its dragging logic
- `module_set_param` calls from egui (Rack handles param changes natively)
- Port/knob metadata queries (`module_inputs`, `module_outputs`, `module_params` — still available but no longer used for interaction)

### What Stays in egui

- `DragState::Cable` — cable creation via port drag interception
- `DragState::Module` — module positioning in the rack
- Cable rendering (bezier curves in egui overlay)
- Module browser and catalog
- The `+ Audio I/O` button (until Audio I/O is a catalog module)

## Continuous Re-rendering

All modules re-render every frame. The current "create texture + send via mpsc" approach works for the initial implementation. Future optimization: persistent render target textures with double-buffering to avoid per-frame allocation.

## Audio I/O Module Widget

### Widget Class

`AudioIOWidget` — a `ModuleWidget` subclass that:
- Loads Cardinal's `HostAudio.svg` panel (`plugins/Cardinal/res/HostAudio.svg`)
- Adds 2 input `PJ301MPort` widgets (left/right to speakers)
- Adds 2 output `PJ301MPort` widgets (left/right from mic/input)
- Draws "Audio I/O" title and "L"/"R" channel labels via NanoVG

### Model Change

Update `AudioIOModel::createModuleWidget` to return `new AudioIOWidget(module)` instead of `nullptr`.

### Catalog Registration

Register `AudioIOModel` under a "Cardinal" plugin slug so it appears in the module browser alongside other modules. Remove the special-case `CreateAudio` command, `spawn_audio` method, and `+ Audio I/O` button from egui.

### What We Reuse from Cardinal

- `HostAudio.svg` panel asset (8HP, dark gradient, matches Cardinal theme)
- Visual style and port placement pattern
- Standard Rack port widgets (`PJ301MPort`)

### What We Don't Reuse

- `CardinalPluginContext` audio bridging (keep cpal-based approach)
- `NanoMeter` / `NanoKnob` (future enhancement)
- `ModuleWidgetWith8HP` base class (hardcode the simple layout)

## Implementation Order

1. Audio I/O widget (independent, can ship first)
2. Bridge event API (`cardinal_module_event`, `cardinal_module_check_port_drag`)
3. egui event translation and forwarding
4. Port drag interception for cables
5. Remove old egui interaction overlay
6. Continuous re-rendering optimization (persistent textures)
