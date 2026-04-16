use std::sync::{mpsc, Arc};
use cardinal_core::cardinal_thread::{Command, RenderResult};
use cardinal_core::{ModuleId, CableId, CatalogEntry};
use rustc_hash::FxHashMap;
use stardust_xr_fusion::client::ClientHandle;
use stardust_xr_fusion::spatial::{Spatial, SpatialRefAspect, Transform};
use crate::dmatex;
use crate::module_panel::{DmatexState, ModulePanel};
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
    /// wgpu device for creating exportable textures.
    device: Arc<wgpu::Device>,
    /// wgpu queue for submitting GPU commands (used by CPU readback).
    queue: Arc<wgpu::Queue>,
    /// Stardust client handle for dmatex import.
    client_handle: Arc<ClientHandle>,
    /// Next dmatex ID to assign.
    next_dmatex_id: u64,
    /// DRM render node (shared across all modules).
    render_node: Option<timeline_syncobj::render_node::DrmRenderNode>,
}

impl Workspace {
    pub fn new(
        parent: &impl SpatialRefAspect,
        catalog: Vec<CatalogEntry>,
        cmd_tx: mpsc::Sender<Command>,
        render_rx: mpsc::Receiver<RenderResult>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        client_handle: Arc<ClientHandle>,
    ) -> Self {
        let root_spatial = Spatial::create(parent, Transform::identity())
            .expect("cardinal-xr: failed to create workspace root spatial");
        let render_node = dmatex::open_matching_drm_render_node(&device);
        if render_node.is_none() {
            eprintln!("cardinal-xr: WARNING: no DRM render node found, DMA-BUF textures unavailable");
        }
        Self {
            modules: FxHashMap::default(),
            cables: FxHashMap::default(),
            catalog,
            cmd_tx,
            cable_drag: CableDragState::Idle,
            root_spatial,
            render_rx,
            next_cable_color_idx: 0,
            device,
            client_handle,
            queue,
            next_dmatex_id: 1,
            render_node,
        }
    }

    pub fn spawn_module(
        &mut self,
        plugin: String,
        model: String,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: f32,
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
        let params = info.params;

        // Request initial render (texture will be provided after dmatex setup below)
        self.cmd_tx
            .send(Command::RenderModule {
                module_id: id,
                width: size.0 as i32,
                height: size.1 as i32,
                texture: None,
            })
            .ok()?;

        eprintln!("cardinal-xr: creating panel scene graph for {id:?} ({} inputs, {} outputs, {} params)", inputs.len(), outputs.len(), params.len());
        let mut panel = ModulePanel::new(&self.root_spatial, id, size, inputs, outputs, params, position, rotation, scale);

        // Set up DMA-BUF texture streaming if available.
        if let Some(render_node) = &self.render_node {
            let w = size.0 as u32;
            let h = size.1 as u32;
            if let Some(textures) = dmatex::create_exportable_texture_pair(&self.device, w, h) {
                // Create a timeline syncobj.
                if let Some(syncobj_state) = dmatex::create_timeline_syncobj(render_node) {
                    // Assign dmatex IDs.
                    let dmatex_id_0 = self.next_dmatex_id;
                    let dmatex_id_1 = self.next_dmatex_id + 1;
                    self.next_dmatex_id += 2;

                    // Import both textures into Stardust.
                    use stardust_xr_fusion::drawable::{DmatexSize, DmatexPlane};
                    use stardust_xr_wire::fd::ProtocolFd;

                    let dmatex_ids = [dmatex_id_0, dmatex_id_1];
                    for (i, tex) in textures.iter().enumerate() {
                        let dmabuf_fd_dup = tex.dmabuf.fd.try_clone()
                            .expect("failed to dup dmabuf fd");
                        let syncobj_dup_i = syncobj_state.fd.try_clone()
                            .expect("failed to dup syncobj fd");

                        let result = stardust_xr_fusion::drawable::import_dmatex(
                            &self.client_handle,
                            dmatex_ids[i],
                            DmatexSize::Dim2D(mint::Vector2 { x: w, y: h }),
                            dmatex::drm_format(),
                            tex.dmabuf.drm_format_modifier,
                            false, // not sRGB (Rgba8Unorm is linear)
                            None,  // no array layers
                            &[DmatexPlane {
                                dmabuf_fd: ProtocolFd::from(dmabuf_fd_dup),
                                offset: tex.dmabuf.offset,
                                row_size: tex.dmabuf.stride,
                                array_element_size: 0,
                                depth_slice_size: 0,
                            }],
                            ProtocolFd::from(syncobj_dup_i),
                        );

                        match result {
                            Ok(()) => eprintln!("cardinal-xr: imported dmatex {} for module {id:?}", dmatex_ids[i]),
                            Err(e) => eprintln!("cardinal-xr: failed to import dmatex {}: {e}", dmatex_ids[i]),
                        }
                    }

                    panel.dmatex_state = Some(DmatexState {
                        textures,
                        dmatex_ids: [dmatex_id_0, dmatex_id_1],
                        current_buffer: 0,
                        acquire_point: 0,
                        syncobj: syncobj_state.syncobj,
                    });

                    // TODO: Send the exportable textures to the cardinal thread
                    // so it renders to them instead of creating new ones each frame.
                    // For now, the on_render_result path handles texture streaming.

                    eprintln!("cardinal-xr: DMA-BUF textures set up for module {id:?}");
                } else {
                    eprintln!("cardinal-xr: failed to create timeline syncobj for module {id:?}");
                }
            } else {
                eprintln!("cardinal-xr: failed to create exportable textures for module {id:?}");
            }
        }

        eprintln!("cardinal-xr: panel scene graph created for {id:?}");
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

    pub fn frame_update(&mut self, dt: f32) {
        self.poll_render_results();

        // Request re-renders for all modules.
        // Using standard textures (CPU readback path) for now.
        // TODO: switch to dmatex textures once timeline syncobj issues are resolved.
        for panel in self.modules.values() {
            let _ = self.cmd_tx.send(Command::RenderModule {
                module_id: panel.id,
                width: panel.size_px.0 as i32,
                height: panel.size_px.1 as i32,
                texture: None,
            });
        }

        // Update all module panels (grab, resize, delete, widget interactions).
        for panel in self.modules.values_mut() {
            panel.frame_update(dt, &self.cmd_tx);
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
