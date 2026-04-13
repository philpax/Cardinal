# Input Forwarding and Audio I/O Widget Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Forward mouse events from egui into Rack's widget tree so modules handle their own interaction natively (knobs, buttons, lights), intercept port drags for egui cable management, and give Audio I/O a proper rendered widget.

**Architecture:** The C++ bridge gains an event-injection API that constructs Rack events and dispatches them into a ModuleWidget's child tree, with per-module drag state tracking. egui translates its mouse events to bridge calls (routed through the cardinal thread for thread safety), replacing its custom hit-testing overlay. Port clicks are intercepted before Rack creates CableWidgets, keeping cable management in egui. The AudioIOModule gets a proper ModuleWidget with an SVG panel and port widgets, registered as a catalog module.

**Tech Stack:** C++ (Rack widget/event system, NanoVG), Rust (egui, wgpu, FFI bindings)

**Spec:** `docs/superpowers/specs/2026-04-13-input-forwarding-and-audio-widget-design.md`

**Build environment:** All build commands must be run inside the nix-shell. Use `nix-shell --run '<command>'` from the repo root. **Use `cargo clippy` instead of `cargo build`** — full builds take minutes; clippy checks correctness fast. E.g.: `nix-shell --run 'cd cardinal-rs && cargo clippy'`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `cardinal-rs/crates/cardinal-core/cpp/bridge.h` | C API: new event types, `cardinal_module_event`, `cardinal_module_check_port_drag` |
| `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp` | C++ impl: AudioIOWidget, per-module event state, event dispatch |
| `cardinal-rs/crates/cardinal-core/src/ffi.rs` | Rust FFI declarations for new bridge functions |
| `cardinal-rs/crates/cardinal-core/src/lib.rs` | Safe Rust wrappers for event forwarding |
| `cardinal-rs/crates/cardinal-egui/src/main.rs` | egui event translation, remove old overlay, catalog Audio I/O |

## Threading Model

All `cc::` calls must happen on the **cardinal thread** (where the Rack engine, widgets, and NanoVG context live). The UI thread sends `Command` messages and receives results via `mpsc` channels. Event forwarding follows this same pattern:
- Button-press events use synchronous reply (UI needs to know if a widget consumed the event to decide between widget drag, cable drag, or module drag).
- Hover, scroll, drag-move, and button-release events use fire-and-forget sends (no reply needed).

## Notes

- `find_port_at` is kept for cable **drop** detection (determining which port a cable is dropped on). It could be replaced with a bridge function in the future, but the spec doesn't require this.
- `params` field is removed from `PlacedModule` since Rack handles param interaction natively.
- Continuous re-rendering is already implemented (`request_renders` called every frame).

---

### Task 1: AudioIOWidget — SVG Panel and Port Widgets

Give AudioIOModule a proper ModuleWidget so it renders like any other module. The widget loads Cardinal's HostAudio.svg panel and adds PJ301MPort widgets for 2 inputs and 2 outputs.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`

- [ ] **Step 1: Add required includes**

At the top of bridge.cpp, add includes needed for SvgPanel, PortWidget, and port creation helpers. Add after the existing `#include <app/SvgPanel.hpp>`:

```cpp
#include <app/SvgPort.hpp>
#include <componentlibrary.hpp>
```

These are available from the Rack include path (`src/Rack/include/`). `componentlibrary.hpp` provides `PJ301MPort` and the `createInput`/`createOutput` template helpers.

- [ ] **Step 2: Implement AudioIOWidget class**

Add the AudioIOWidget class above `AudioIOModel` in bridge.cpp (after the AudioIOModule struct, before the AudioIOModel struct):

```cpp
// ── AudioIO widget — SVG panel + port widgets ───────────────────────
struct AudioIOWidget : rack::app::ModuleWidget {
    AudioIOWidget(AudioIOModule* module) {
        setModule(module);
        setPanel(rack::window::Svg::load(
            rack::asset::plugin(nullptr, "plugins/Cardinal/res/HostAudio.svg")));

        // Inputs: audio from patch → speakers (left column)
        // 8HP panel = 120px wide. Positions match Cardinal's HostAudio layout.
        addInput(rack::createInputCentered<rack::componentlibrary::PJ301MPort>(
            rack::math::Vec(30.0f, 220.0f), module, 0));
        addInput(rack::createInputCentered<rack::componentlibrary::PJ301MPort>(
            rack::math::Vec(30.0f, 275.0f), module, 1));

        // Outputs: audio from mic/input → patch (right column)
        addOutput(rack::createOutputCentered<rack::componentlibrary::PJ301MPort>(
            rack::math::Vec(90.0f, 220.0f), module, 0));
        addOutput(rack::createOutputCentered<rack::componentlibrary::PJ301MPort>(
            rack::math::Vec(90.0f, 275.0f), module, 1));
    }

    void draw(const DrawArgs& args) override {
        ModuleWidget::draw(args);

        NVGcontext* vg = args.vg;

        // Title
        nvgFontSize(vg, 13.0f);
        nvgFillColor(vg, nvgRGBf(1.0f, 1.0f, 1.0f));
        nvgTextAlign(vg, NVG_ALIGN_CENTER | NVG_ALIGN_MIDDLE);
        nvgText(vg, box.size.x * 0.5f, 30.0f, "Audio I/O", nullptr);

        // Port labels
        nvgFontSize(vg, 10.0f);
        nvgTextAlign(vg, NVG_ALIGN_CENTER | NVG_ALIGN_BOTTOM);
        nvgText(vg, 30.0f, 210.0f, "L", nullptr);
        nvgText(vg, 30.0f, 265.0f, "R", nullptr);
        nvgText(vg, 90.0f, 210.0f, "L", nullptr);
        nvgText(vg, 90.0f, 265.0f, "R", nullptr);

        // Section labels
        nvgFontSize(vg, 9.0f);
        nvgFillColor(vg, nvgRGBf(0.7f, 0.7f, 0.7f));
        nvgTextAlign(vg, NVG_ALIGN_CENTER | NVG_ALIGN_MIDDLE);
        nvgText(vg, 30.0f, 190.0f, "TO SPKR", nullptr);
        nvgText(vg, 90.0f, 190.0f, "FROM IN", nullptr);
    }
};
```

- [ ] **Step 3: Update AudioIOModel to return AudioIOWidget**

Replace the `createModuleWidget` override in `AudioIOModel`:

```cpp
struct AudioIOModel : rack::plugin::Model {
    rack::engine::Module* createModule() override {
        auto* m = new AudioIOModule();
        m->model = this;
        return m;
    }
    rack::app::ModuleWidget* createModuleWidget(rack::engine::Module* m) override {
        return new AudioIOWidget(static_cast<AudioIOModule*>(m));
    }
};
```

- [ ] **Step 4: Update cardinal_audio_create to store the widget**

In `cardinal_audio_create`, create the widget and store it in g_modules. Replace the existing function body:

```cpp
ModuleHandle cardinal_audio_create(void) {
    if (!g_engine || g_audioIO) return -1;  // only one audio module

    g_audioIOModel.slug = "AudioIO";
    g_audioIOModel.name = "Audio I/O";

    // Register as a terminal model so the engine processes it
    // before/after all other modules
    hostTerminalModels.push_back(&g_audioIOModel);

    auto* module = new AudioIOModule();
    module->model = &g_audioIOModel;
    g_engine->addModule(module);
    g_audioIO = module;

    int64_t handle = module->id;

    // Create widget for rendering
    rack::app::ModuleWidget* widget = nullptr;
    if (!rack::settings::headless) {
        try {
            widget = g_audioIOModel.createModuleWidget(module);
        } catch (...) {
            fprintf(stderr, "cardinal: failed to create AudioIO widget\n");
        }
    }

    g_modules[handle] = { module, widget, &g_audioIOModel };
    return handle;
}
```

- [ ] **Step 5: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation. If `componentlibrary.hpp` or `SvgPort.hpp` aren't found, check the Rack include path. If `asset::plugin(nullptr, ...)` doesn't resolve the path correctly, try `asset::system("plugins/Cardinal/res/HostAudio.svg")` instead.

- [ ] **Step 6: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.cpp
git commit -m "feat: add AudioIOWidget with SVG panel and port widgets"
```

---

### Task 2: Register Audio I/O in the Module Catalog

Make AudioIO appear in the module browser as a normal catalog entry. Remove the special `Command::CreateAudio` pathway and `+ Audio I/O` button.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`

- [ ] **Step 1: Register AudioIOModel with a Cardinal plugin during init**

In `cardinal_init`, after creating the EventState and before the final log line, register the AudioIO model. The real `rack::plugin::Model` (from `src/Rack/include/plugin/Model.hpp`) has a `Plugin* plugin` field — our AudioIOModel inherits this.

Add this block:

```cpp
    // Register built-in AudioIO module as a catalog entry
    {
        static rack::plugin::Plugin audioPlugin;
        audioPlugin.slug = "Cardinal";
        g_audioIOModel.slug = "AudioIO";
        g_audioIOModel.name = "Audio I/O";
        g_audioIOModel.plugin = &audioPlugin;
        audioPlugin.models.push_back(&g_audioIOModel);
        rack::plugin::plugins.push_back(&audioPlugin);
        // Register as terminal model so engine processes it before/after others
        hostTerminalModels.push_back(&g_audioIOModel);
    }
```

- [ ] **Step 2: Simplify cardinal_audio_create to use normal creation path**

Replace the body of `cardinal_audio_create`:

```cpp
ModuleHandle cardinal_audio_create(void) {
    if (!g_engine || g_audioIO) return -1;

    ModuleHandle handle = cardinal_module_create("Cardinal", "AudioIO");
    if (handle < 0) return -1;

    auto it = g_modules.find(handle);
    g_audioIO = static_cast<AudioIOModule*>(it->second.module);

    return handle;
}
```

Remove the `hostTerminalModels.push_back` from here since it moved to `cardinal_init`.

- [ ] **Step 3: Remove special-case Audio I/O UI from egui**

In `main.rs`:

**a)** Remove `Command::CreateAudio` variant from the `Command` enum:

```rust
// DELETE these lines:
    CreateAudio {
        reply: mpsc::Sender<Option<ModuleInfo>>,
    },
```

**b)** Remove its handler in the cardinal thread match block:

```rust
// DELETE this match arm:
                    Command::CreateAudio { reply } => {
                        let info = cc::audio_create().map(|id| {
                            let (w, h) = cc::module_size(id);
                            ModuleInfo {
                                id,
                                size: (w.max(90.0), h.max(200.0)),
                                inputs: cc::module_inputs(id),
                                outputs: cc::module_outputs(id),
                                params: cc::module_params(id),
                            }
                        });
                        let _ = reply.send(info);
                    }
```

**c)** Remove the `spawn_audio` method from `App`:

```rust
// DELETE this entire method:
    fn spawn_audio(&mut self, pos: egui::Pos2) {
        ...
    }
```

**d)** Remove the `+ Audio I/O` button from the UI. In `App::ui`, delete the block:

```rust
// DELETE this block (including the separator after it):
                // Audio I/O button — creates the stereo terminal module
                if ui.add(egui::Button::new(
                    egui::RichText::new("+ Audio I/O").strong()
                ).min_size(egui::vec2(180.0, 0.0))).clicked() {
                    let x = 220.0 + self.modules.len() as f32 * 20.0;
                    let y = 50.0 + (self.modules.len() % 3) as f32 * 120.0;
                    self.spawn_audio(egui::pos2(x, y));
                }
                ui.separator();
```

- [ ] **Step 4: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation. Audio I/O now appears in the module browser under "Cardinal" section.

- [ ] **Step 5: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.cpp cardinal-rs/crates/cardinal-egui/src/main.rs
git commit -m "feat: register Audio I/O as catalog module, remove special-case UI"
```

---

### Task 3: Bridge Event API — Types and State

Add event type constants to the C header, per-module event state tracking, and function declarations for the event API.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.h`
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`

- [ ] **Step 1: Add event type constants and function declarations to bridge.h**

Add after the `cardinal_module_render` declaration and before the "Cable management" section:

```c
// ── Event forwarding ─────────────────────────────────────────────────
// Event types for cardinal_module_event
#define CARDINAL_EVENT_HOVER       0
#define CARDINAL_EVENT_BUTTON      1
#define CARDINAL_EVENT_SCROLL      2
#define CARDINAL_EVENT_LEAVE       3

/// Forward a mouse/input event to a module's widget tree.
/// event_type: CARDINAL_EVENT_HOVER, _BUTTON, _SCROLL, _LEAVE
/// button: 0=left, 1=right, 2=middle (matches GLFW)
/// action: 0=release, 1=press (for BUTTON events)
/// mods: GLFW modifier bitmask (shift=1, ctrl=2, alt=4, super=8)
/// Returns 1 if the event was consumed by a child widget (not the ModuleWidget itself).
int cardinal_module_event(ModuleHandle h, int event_type,
                          float x, float y,
                          int button, int action, int mods,
                          float scroll_x, float scroll_y);

/// After forwarding a button-press event, check if a port was clicked.
/// Returns 1 if a port drag started, filling out port_id and is_output.
/// Clears the module's drag state to prevent Rack from creating a CableWidget.
int cardinal_module_check_port_drag(ModuleHandle h, int* port_id, int* is_output);
```

- [ ] **Step 2: Add per-module event state to ModuleEntry in bridge.cpp**

Extend the `ModuleEntry` struct to track hover and drag state per module:

```cpp
struct ModuleEntry {
    rack::engine::Module* module = nullptr;
    rack::app::ModuleWidget* widget = nullptr;
    rack::plugin::Model* model = nullptr;

    // Per-module event state (mini EventState for direct injection)
    rack::widget::Widget* hoveredWidget = nullptr;
    rack::widget::Widget* draggedWidget = nullptr;
    int dragButton = -1;
    rack::math::Vec lastMousePos;
};
```

- [ ] **Step 3: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation.

- [ ] **Step 4: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.h cardinal-rs/crates/cardinal-core/cpp/bridge.cpp
git commit -m "feat: add event type constants and per-module event state to bridge"
```

---

### Task 4: Implement cardinal_module_event

Implement the main event forwarding function. This constructs Rack events and dispatches them into the ModuleWidget's child tree, tracking hover and drag state.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`

- [ ] **Step 1: Implement cardinal_module_event**

Add before the "Cable management" section in bridge.cpp:

```cpp
// ── Event forwarding ────────────────────────────────────────────────

int cardinal_module_event(ModuleHandle h, int event_type,
                          float x, float y,
                          int button, int action, int mods,
                          float scroll_x, float scroll_y) {
    auto it = g_modules.find(h);
    if (it == g_modules.end() || !it->second.widget) return 0;

    auto* widget = it->second.widget;
    auto& entry = it->second;
    rack::math::Vec pos(x, y);
    rack::math::Vec mouseDelta = pos.minus(entry.lastMousePos);

    switch (event_type) {
    case CARDINAL_EVENT_HOVER: {
        if (entry.draggedWidget) {
            // During drag: send DragMoveEvent to dragged widget
            rack::widget::Widget::DragMoveEvent dme;
            dme.button = entry.dragButton;
            dme.mouseDelta = mouseDelta;
            entry.draggedWidget->onDragMove(dme);
            entry.lastMousePos = pos;
            return 1;
        }

        // Normal hover: dispatch through widget tree
        rack::widget::Widget::HoverEvent he;
        he.pos = pos;
        he.mouseDelta = mouseDelta;
        widget->onHover(he);

        // Track enter/leave for hovered widget
        rack::widget::Widget* newHovered = he.isConsumed() ? he.getTarget() : nullptr;
        if (newHovered != entry.hoveredWidget) {
            if (entry.hoveredWidget) {
                rack::widget::Widget::LeaveEvent le;
                entry.hoveredWidget->onLeave(le);
            }
            if (newHovered) {
                rack::widget::Widget::EnterEvent ee;
                newHovered->onEnter(ee);
            }
            entry.hoveredWidget = newHovered;
        }

        entry.lastMousePos = pos;
        return he.isConsumed() ? 1 : 0;
    }

    case CARDINAL_EVENT_BUTTON: {
        int glfw_action = (action == 1) ? GLFW_PRESS : GLFW_RELEASE;

        if (glfw_action == GLFW_RELEASE && entry.draggedWidget) {
            // End drag first
            rack::widget::Widget::DragEndEvent dee;
            dee.button = entry.dragButton;
            entry.draggedWidget->onDragEnd(dee);
            entry.draggedWidget = nullptr;
            entry.dragButton = -1;
        }

        // Dispatch button event through widget tree
        rack::widget::Widget::ButtonEvent be;
        be.pos = pos;
        be.button = button;
        be.action = glfw_action;
        be.mods = mods;
        widget->onButton(be);

        if (glfw_action == GLFW_PRESS && be.isConsumed()) {
            rack::widget::Widget* target = be.getTarget();
            // Only track drag if a CHILD consumed it (not the ModuleWidget itself)
            if (target && target != widget) {
                entry.draggedWidget = target;
                entry.dragButton = button;

                rack::widget::Widget::DragStartEvent dse;
                dse.button = button;
                target->onDragStart(dse);
            }
            entry.lastMousePos = pos;
            return (target && target != widget) ? 1 : 0;
        }

        entry.lastMousePos = pos;
        return be.isConsumed() ? 1 : 0;
    }

    case CARDINAL_EVENT_SCROLL: {
        rack::widget::Widget::HoverScrollEvent hse;
        hse.pos = pos;
        hse.scrollDelta = rack::math::Vec(scroll_x, scroll_y);
        widget->onHoverScroll(hse);
        return hse.isConsumed() ? 1 : 0;
    }

    case CARDINAL_EVENT_LEAVE: {
        if (entry.hoveredWidget) {
            rack::widget::Widget::LeaveEvent le;
            entry.hoveredWidget->onLeave(le);
            entry.hoveredWidget = nullptr;
        }
        if (entry.draggedWidget) {
            rack::widget::Widget::DragEndEvent dee;
            dee.button = entry.dragButton;
            entry.draggedWidget->onDragEnd(dee);
            entry.draggedWidget = nullptr;
            entry.dragButton = -1;
        }
        return 0;
    }

    default:
        return 0;
    }
}
```

**Important:** The `isConsumed()` and `getTarget()` methods are on the real Rack `BaseEvent` (from `src/Rack/include/widget/event.hpp`), not the lv2export stubs. Since bridge.cpp includes the real Rack headers, these are available. If `getTarget()` isn't available in your Rack version, `BaseEvent` typically has a `Widget* target` member you can access directly after `consume()` sets it.

- [ ] **Step 2: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation. If there are errors with `isConsumed()` or `getTarget()`, check the Rack `BaseEvent` definition in `src/Rack/include/widget/event.hpp` and adjust member access accordingly (e.g., `e.target` instead of `e.getTarget()`).

- [ ] **Step 3: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.cpp
git commit -m "feat: implement cardinal_module_event for hover, button, scroll, leave"
```

---

### Task 5: Implement cardinal_module_check_port_drag

After a button-press event, check if the drag target is a PortWidget. If so, extract port info and clear the drag state to prevent Rack from creating its own cable.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/cpp/bridge.cpp`

- [ ] **Step 1: Implement cardinal_module_check_port_drag**

Add after `cardinal_module_event`:

```cpp
int cardinal_module_check_port_drag(ModuleHandle h, int* port_id, int* is_output) {
    auto it = g_modules.find(h);
    if (it == g_modules.end() || !it->second.widget) return 0;
    if (!it->second.draggedWidget) return 0;

    auto* widget = it->second.widget;

    // Check if the dragged widget is in the input or output port lists
    // (This avoids dynamic_cast and works with any PortWidget subclass)
    for (int i = 0; i < (int)widget->module->getNumInputs(); i++) {
        if (widget->getInput(i) == it->second.draggedWidget) {
            *port_id = i;
            *is_output = 0;
            goto found;
        }
    }
    for (int i = 0; i < (int)widget->module->getNumOutputs(); i++) {
        if (widget->getOutput(i) == it->second.draggedWidget) {
            *port_id = i;
            *is_output = 1;
            goto found;
        }
    }
    return 0;  // Not a port widget

found:
    // Cancel Rack's drag — send DragEnd to clean up port widget state
    {
        rack::widget::Widget::DragEndEvent dee;
        dee.button = it->second.dragButton;
        it->second.draggedWidget->onDragEnd(dee);
    }
    it->second.draggedWidget = nullptr;
    it->second.dragButton = -1;
    return 1;
}
```

- [ ] **Step 2: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation.

- [ ] **Step 3: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/cpp/bridge.cpp
git commit -m "feat: implement port drag detection for cable interception"
```

---

### Task 6: Expose Event API to Rust FFI

Add the new C functions to the Rust FFI layer and create safe wrappers.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-core/src/ffi.rs`
- Modify: `cardinal-rs/crates/cardinal-core/src/lib.rs`

- [ ] **Step 1: Add FFI declarations to ffi.rs**

Add inside the `unsafe extern "C"` block, after the `cardinal_module_render` declaration:

```rust
    // Event forwarding
    pub fn cardinal_module_event(
        h: i64, event_type: c_int,
        x: f32, y: f32,
        button: c_int, action: c_int, mods: c_int,
        scroll_x: f32, scroll_y: f32,
    ) -> c_int;

    pub fn cardinal_module_check_port_drag(
        h: i64, port_id: *mut c_int, is_output: *mut c_int,
    ) -> c_int;
```

- [ ] **Step 2: Add event type constants and safe wrappers to lib.rs**

Add after the `module_render` function:

```rust
// ── Event forwarding ────────────────────────────────────────────────

/// Event types for module_event.
pub const EVENT_HOVER: i32 = 0;
pub const EVENT_BUTTON: i32 = 1;
pub const EVENT_SCROLL: i32 = 2;
pub const EVENT_LEAVE: i32 = 3;

/// Forward a mouse event to a module's widget tree.
/// Returns true if a child widget consumed the event.
pub fn module_event(
    id: ModuleId,
    event_type: i32,
    x: f32, y: f32,
    button: i32, action: i32, mods: i32,
    scroll_x: f32, scroll_y: f32,
) -> bool {
    unsafe {
        ffi::cardinal_module_event(
            id.0, event_type, x, y,
            button, action, mods,
            scroll_x, scroll_y,
        ) != 0
    }
}

/// Port drag result from check_port_drag.
#[derive(Debug, Clone)]
pub struct PortDragInfo {
    pub port_id: i32,
    pub is_output: bool,
}

/// After a button-press event, check if a port was clicked.
/// Returns Some with port info if a port drag started.
/// Clears the module's drag state to prevent Rack cable creation.
pub fn module_check_port_drag(id: ModuleId) -> Option<PortDragInfo> {
    let mut port_id: i32 = 0;
    let mut is_output: i32 = 0;
    let result = unsafe {
        ffi::cardinal_module_check_port_drag(id.0, &mut port_id, &mut is_output)
    };
    if result != 0 {
        Some(PortDragInfo {
            port_id,
            is_output: is_output != 0,
        })
    } else {
        None
    }
}
```

- [ ] **Step 3: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation with new FFI bindings linked.

- [ ] **Step 4: Commit**

```bash
git add cardinal-rs/crates/cardinal-core/src/ffi.rs cardinal-rs/crates/cardinal-core/src/lib.rs
git commit -m "feat: expose event forwarding API to Rust FFI"
```

---

### Task 7: egui Event Translation — Route Events Through Cardinal Thread

Rewrite the egui interaction handling to forward events through the cardinal thread (for thread safety) instead of using the custom hit-testing overlay. This replaces knob interaction with native Rack widget handling, while keeping cable and module dragging in egui.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`

- [ ] **Step 1: Add event command types and result structs**

Add the event-related types near the existing `Command` enum:

```rust
struct EventResult {
    consumed: bool,
    port_drag: Option<cc::PortDragInfo>,
}
```

Add new variants to the `Command` enum:

```rust
    ModuleEvent {
        module_id: ModuleId,
        event_type: i32,
        x: f32,
        y: f32,
        button: i32,
        action: i32,
        mods: i32,
        scroll_x: f32,
        scroll_y: f32,
        reply: Option<mpsc::Sender<EventResult>>,
    },
```

- [ ] **Step 2: Handle ModuleEvent in the cardinal thread**

Add a match arm in the cardinal thread's command loop (after the existing match arms):

```rust
                    Command::ModuleEvent {
                        module_id, event_type, x, y,
                        button, action, mods, scroll_x, scroll_y,
                        reply,
                    } => {
                        let consumed = cc::module_event(
                            module_id, event_type, x, y,
                            button, action, mods, scroll_x, scroll_y,
                        );
                        if let Some(reply) = reply {
                            // For button-press, also check for port drag
                            let port_drag = if event_type == cc::EVENT_BUTTON
                                && action == 1
                                && consumed
                            {
                                cc::module_check_port_drag(module_id)
                            } else {
                                None
                            };
                            let _ = reply.send(EventResult { consumed, port_drag });
                        }
                    }
```

- [ ] **Step 3: Add active_module_drag field and event helper to App**

Add `active_module_drag` field to `App`:

```rust
struct App {
    modules: Vec<PlacedModule>,
    cables: Vec<Cable>,
    catalog: Vec<cc::CatalogEntry>,
    drag: Option<DragState>,
    active_module_drag: Option<ModuleId>,
    browser_filter: String,
    cmd_tx: mpsc::Sender<Command>,
    render_rx: mpsc::Receiver<RenderResult>,
}
```

Initialize it as `None` in `App::new`.

Add a helper method for sending events:

```rust
    /// Send a module event through the cardinal thread.
    /// Button-press events get a synchronous reply; others are fire-and-forget.
    fn send_module_event(
        &self, module_id: ModuleId, event_type: i32,
        x: f32, y: f32, button: i32, action: i32, mods: i32,
        scroll_x: f32, scroll_y: f32,
    ) -> Option<EventResult> {
        if event_type == cc::EVENT_BUTTON && action == 1 {
            // Button-press: synchronous reply needed
            let (reply_tx, reply_rx) = mpsc::channel();
            let _ = self.cmd_tx.send(Command::ModuleEvent {
                module_id, event_type, x, y, button, action, mods,
                scroll_x, scroll_y, reply: Some(reply_tx),
            });
            reply_rx.recv().ok()
        } else {
            // Hover, scroll, release: fire-and-forget
            let _ = self.cmd_tx.send(Command::ModuleEvent {
                module_id, event_type, x, y, button, action, mods,
                scroll_x, scroll_y, reply: None,
            });
            None
        }
    }
```

- [ ] **Step 4: Remove DragState::Knob and find_knob_at**

Simplify `DragState` by removing the `Knob` variant:

```rust
enum DragState {
    Cable {
        from_module: ModuleId,
        from_port: i32,
        is_output: bool,
        mouse_pos: egui::Pos2,
    },
    Module {
        module_idx: usize,
    },
}
```

Delete the `find_knob_at` method entirely.

- [ ] **Step 5: Remove Command::SetParam**

Delete the `SetParam` variant from the `Command` enum and its handler in the cardinal thread:

```rust
// DELETE from Command enum:
    SetParam {
        module: ModuleId,
        param_id: i32,
        value: f32,
    },

// DELETE from cardinal thread:
                    Command::SetParam {
                        module,
                        param_id,
                        value,
                    } => {
                        cc::module_set_param(module, param_id, value);
                    }
```

- [ ] **Step 6: Add modifier helper function**

Add this free function before the `App` impl:

```rust
fn egui_mods_to_rack(modifiers: &egui::Modifiers) -> i32 {
    let mut mods = 0i32;
    if modifiers.shift { mods |= 1; }   // GLFW_MOD_SHIFT
    if modifiers.ctrl { mods |= 2; }    // GLFW_MOD_CONTROL
    if modifiers.alt { mods |= 4; }     // GLFW_MOD_ALT
    if modifiers.mac_cmd || modifiers.command { mods |= 8; } // GLFW_MOD_SUPER
    mods
}
```

- [ ] **Step 7: Rewrite drag_started handling**

Replace the existing `drag_started` block in the central panel event loop:

```rust
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let m = &self.modules[*idx];
                        let local_x = pos.x - m.pos.x;
                        let local_y = pos.y - m.pos.y;
                        let mods = egui_mods_to_rack(&ctx.input(|i| i.modifiers));

                        // Forward button-press to Rack's widget tree
                        if let Some(result) = self.send_module_event(
                            m.id, cc::EVENT_BUTTON, local_x, local_y,
                            0, 1, mods, 0.0, 0.0,
                        ) {
                            if result.consumed {
                                if let Some(port_info) = result.port_drag {
                                    // Port was clicked — start cable drag in egui
                                    self.drag = Some(DragState::Cable {
                                        from_module: m.id,
                                        from_port: port_info.port_id,
                                        is_output: port_info.is_output,
                                        mouse_pos: pos,
                                    });
                                } else {
                                    // A widget (knob, button, switch) consumed the event
                                    self.active_module_drag = Some(m.id);
                                }
                            } else {
                                // No child widget consumed it — module drag
                                self.drag = Some(DragState::Module { module_idx: *idx });
                            }
                        } else {
                            // No reply (shouldn't happen) — fallback to module drag
                            self.drag = Some(DragState::Module { module_idx: *idx });
                        }
                    }
                }
```

- [ ] **Step 8: Rewrite dragged handling**

Replace the existing `dragged` block:

```rust
                if response.dragged() {
                    match &mut self.drag {
                        Some(DragState::Cable { mouse_pos, .. }) => {
                            if let Some(pos) = response.interact_pointer_pos() {
                                *mouse_pos = pos;
                            }
                        }
                        Some(DragState::Module { module_idx }) => {
                            self.modules[*module_idx].pos += response.drag_delta();
                        }
                        None => {
                            if let Some(active_id) = self.active_module_drag {
                                // Forward drag move to Rack's widget tree
                                if let Some(pos) = response.interact_pointer_pos() {
                                    if let Some(m) = self.modules.iter().find(|m| m.id == active_id) {
                                        let local_x = pos.x - m.pos.x;
                                        let local_y = pos.y - m.pos.y;
                                        self.send_module_event(
                                            active_id, cc::EVENT_HOVER,
                                            local_x, local_y,
                                            0, 0, 0, 0.0, 0.0,
                                        );
                                    }
                                }
                            } else {
                                self.modules[*idx].pos += response.drag_delta();
                            }
                        }
                    }
                }
```

- [ ] **Step 9: Rewrite drag_stopped handling**

Replace the existing `drag_stopped` block:

```rust
                if response.drag_stopped() {
                    if let Some(DragState::Cable {
                        from_module,
                        from_port,
                        is_output,
                        ..
                    }) = self.drag.take()
                    {
                        if let Some(pos) = response.interact_pointer_pos() {
                            drag_completed = Some((from_module, from_port, is_output, pos));
                        }
                    } else if let Some(active_id) = self.active_module_drag.take() {
                        // Forward button-release to Rack's widget tree
                        if let Some(pos) = response.interact_pointer_pos() {
                            if let Some(m) = self.modules.iter().find(|m| m.id == active_id) {
                                let local_x = pos.x - m.pos.x;
                                let local_y = pos.y - m.pos.y;
                                self.send_module_event(
                                    active_id, cc::EVENT_BUTTON,
                                    local_x, local_y,
                                    0, 0, 0, 0.0, 0.0,  // left button, release, no mods
                                );
                            }
                        }
                    } else {
                        self.drag = None;
                    }
                }
```

- [ ] **Step 10: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation. Knob interaction is now forwarded to Rack natively through the cardinal thread. Module dragging and cable dragging still work through egui.

- [ ] **Step 11: Commit**

```bash
git add cardinal-rs/crates/cardinal-egui/src/main.rs
git commit -m "feat: forward mouse events to Rack widgets via cardinal thread"
```

---

### Task 8: Add Hover and Scroll Forwarding

Forward hover events (for hover highlights, light effects) and scroll events (for fine knob adjustment) even when not dragging.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`

- [ ] **Step 1: Add hover forwarding in the central panel**

After the `for (idx, response) in &responses` loop (which handles drag events) and before the paint section, add hover and scroll forwarding:

```rust
            // Forward hover events to module widgets (when not dragging)
            if self.drag.is_none() && self.active_module_drag.is_none() {
                if let Some(hover_pos) = ctx.pointer_hover_pos() {
                    for m in &self.modules {
                        let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                        if module_rect.contains(hover_pos) {
                            let local_x = hover_pos.x - m.pos.x;
                            let local_y = hover_pos.y - m.pos.y;
                            self.send_module_event(
                                m.id, cc::EVENT_HOVER,
                                local_x, local_y,
                                0, 0, 0, 0.0, 0.0,
                            );
                            break;
                        }
                    }
                }
            }

            // Forward scroll events to module widgets
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
            if scroll_delta != egui::Vec2::ZERO {
                if let Some(hover_pos) = ctx.pointer_hover_pos() {
                    for m in &self.modules {
                        let module_rect = egui::Rect::from_min_size(m.pos, m.size);
                        if module_rect.contains(hover_pos) {
                            let local_x = hover_pos.x - m.pos.x;
                            let local_y = hover_pos.y - m.pos.y;
                            self.send_module_event(
                                m.id, cc::EVENT_SCROLL,
                                local_x, local_y,
                                0, 0, 0,
                                scroll_delta.x, scroll_delta.y,
                            );
                            break;
                        }
                    }
                }
            }
```

- [ ] **Step 2: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation.

- [ ] **Step 3: Commit**

```bash
git add cardinal-rs/crates/cardinal-egui/src/main.rs
git commit -m "feat: forward hover and scroll events to module widgets"
```

---

### Task 9: Remove Stale Interaction Overlay Code

Clean up remaining code that's been replaced by native Rack widget interaction.

**Files:**
- Modify: `cardinal-rs/crates/cardinal-egui/src/main.rs`

- [ ] **Step 1: Remove params from PlacedModule and ModuleInfo**

The `params` field is no longer used (Rack handles param interaction natively). Remove it from `PlacedModule`:

```rust
struct PlacedModule {
    id: ModuleId,
    name: String,
    pos: egui::Pos2,
    size: egui::Vec2,
    inputs: Vec<cc::PortInfo>,
    outputs: Vec<cc::PortInfo>,
    texture_id: Option<egui::TextureId>,
    render_texture: Option<wgpu::Texture>,
}
```

Remove `params` from `ModuleInfo`:

```rust
struct ModuleInfo {
    id: ModuleId,
    size: (f32, f32),
    inputs: Vec<cc::PortInfo>,
    outputs: Vec<cc::PortInfo>,
}
```

- [ ] **Step 2: Remove cc::module_params calls**

In the cardinal thread's `Command::CreateModule` handler, remove the `params` field:

```rust
                    Command::CreateModule {
                        plugin,
                        model,
                        reply,
                    } => {
                        let info = cc::module_create(&plugin, &model).map(|id| {
                            let (w, h) = cc::module_size(id);
                            ModuleInfo {
                                id,
                                size: (w.max(90.0), h.max(200.0)),
                                inputs: cc::module_inputs(id),
                                outputs: cc::module_outputs(id),
                            }
                        });
                        let _ = reply.send(info);
                    }
```

- [ ] **Step 3: Remove params from PlacedModule construction**

In `spawn_module`, remove `params`:

```rust
            self.modules.push(PlacedModule {
                id: info.id,
                name,
                pos,
                size: egui::vec2(info.size.0, info.size.1),
                inputs: info.inputs,
                outputs: info.outputs,
                texture_id: None,
                render_texture: None,
            });
```

- [ ] **Step 4: Build and verify**

Run: `nix-shell --run 'cd cardinal-rs && cargo clippy' 2>&1 | tail -20`

Expected: Successful compilation with no remaining references to `params` in the interaction code.

- [ ] **Step 5: Commit**

```bash
git add cardinal-rs/crates/cardinal-egui/src/main.rs
git commit -m "refactor: remove param metadata from egui, Rack handles interaction natively"
```

---

## Verification Checklist

After all tasks are complete, manually verify:

1. **Audio I/O renders** — spawn Audio I/O from the module browser under "Cardinal". It should show the SVG panel with labeled ports instead of a gray box.
2. **Knobs work** — spawn a module with knobs (e.g., VCV VCO-1). Click and drag up/down on a knob. The rendered knob should rotate and the parameter value should change.
3. **Port clicks start cable drag** — click on a port. A yellow cable preview line should appear and follow the mouse. Drop on another port to complete the cable.
4. **Module dragging still works** — click on the background area of a module (not on a widget) and drag to reposition.
5. **Scroll on knobs** — hover over a knob and scroll. The value should change in fine increments.
6. **Cables render correctly** — connected cables should render as bezier curves between the correct port positions.
7. **Multiple modules interact independently** — drag a knob on one module while another module's lights/meters update.
8. **Hover effects** — hover over widgets (buttons, knobs) and observe hover highlights in the rendered texture.
