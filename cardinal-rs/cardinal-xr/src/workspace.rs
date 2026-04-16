use std::sync::mpsc;
use cardinal_core::cardinal_thread::{Command, RenderResult};
use cardinal_core::{ModuleId, CableId, CatalogEntry};
use rustc_hash::FxHashMap;
use stardust_xr_fusion::spatial::{Spatial, SpatialRefAspect, Transform};
use crate::module_panel::ModulePanel;
use crate::cable::{Cable, CableDragState};

pub struct Workspace {
    pub modules: FxHashMap<ModuleId, ModulePanel>,
    pub cables: FxHashMap<CableId, Cable>,
    pub catalog: Vec<CatalogEntry>,
    pub cmd_tx: mpsc::Sender<Command>,
    pub cable_drag: CableDragState,
    /// Root spatial node for the entire workspace, parented to the Stardust client root.
    pub root_spatial: Spatial,
    render_rx: mpsc::Receiver<RenderResult>,
    next_cable_color_idx: usize,
}

impl Workspace {
    pub fn new(
        parent: &impl SpatialRefAspect,
        catalog: Vec<CatalogEntry>,
        cmd_tx: mpsc::Sender<Command>,
        render_rx: mpsc::Receiver<RenderResult>,
    ) -> Self {
        let root_spatial = Spatial::create(parent, Transform::identity())
            .expect("cardinal-xr: failed to create workspace root spatial");
        Self {
            modules: FxHashMap::default(),
            cables: FxHashMap::default(),
            catalog,
            cmd_tx,
            cable_drag: CableDragState::Idle,
            root_spatial,
            render_rx,
            next_cable_color_idx: 0,
        }
    }

    pub fn spawn_module(
        &mut self,
        plugin: String,
        model: String,
        position: glam::Vec3,
        rotation: glam::Quat,
    ) -> Option<ModuleId> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::CreateModule {
                plugin: plugin.clone(),
                model: model.clone(),
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

        let panel = ModulePanel::new(&self.root_spatial, id, size, inputs, outputs, position, rotation);
        self.modules.insert(id, panel);

        eprintln!("cardinal-xr: spawned module {plugin}/{model} -> {id:?} at {position:?}");

        Some(id)
    }

    pub fn destroy_module(&mut self, id: ModuleId) {
        // Destroy all cables connected to this module first
        let connected_cables: Vec<CableId> = self
            .cables
            .values()
            .filter(|c| c.out_module == id || c.in_module == id)
            .map(|c| c.id)
            .collect();

        for cable_id in connected_cables {
            self.destroy_cable(cable_id);
        }

        // Remove the module panel
        self.modules.remove(&id);

        // Send destroy command to cardinal thread
        let _ = self.cmd_tx.send(Command::DestroyModule(id));
    }

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

        let id = reply_rx.recv().ok()??;

        let color_idx = self.next_cable_color_idx;
        self.next_cable_color_idx =
            (self.next_cable_color_idx + 1) % crate::constants::CABLE_COLORS.len();

        let cable = Cable::new(id, out_mod, out_port, in_mod, in_port, color_idx);
        self.cables.insert(id, cable);

        Some(id)
    }

    pub fn destroy_cable(&mut self, id: CableId) {
        self.cables.remove(&id);
        cardinal_core::cable_destroy(id);
    }

    fn poll_render_results(&mut self) {
        loop {
            match self.render_rx.try_recv() {
                Ok(result) => {
                    let module_id = result.module_id;
                    if let Some(panel) = self.modules.get_mut(&module_id) {
                        panel.on_render_result(result);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn frame_update(&mut self) {
        self.poll_render_results();

        // Update all module panels (grab, resize, delete interactions).
        for panel in self.modules.values_mut() {
            panel.frame_update();
        }

        // Collect modules flagged for deletion.
        let to_delete: Vec<ModuleId> = self
            .modules
            .values()
            .filter(|p| p.pending_delete)
            .map(|p| p.id)
            .collect();
        for id in to_delete {
            self.destroy_module(id);
        }

        // Cable geometry updates come later
    }
}
