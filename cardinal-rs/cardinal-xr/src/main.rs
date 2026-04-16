mod cable;
mod constants;
mod dmatex;
mod hand_menu;
mod interaction;
mod math;
mod module_panel;
mod workspace;

use std::sync::mpsc;
use std::sync::Arc;

use stardust_xr_fusion::root::RootAspect as _;

use crate::hand_menu::HandMenuState;
use crate::workspace::Workspace;

fn main() {
    eprintln!("cardinal-xr: starting");

    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
    let _audio_stream = cardinal_core::audio::start_audio_stream();

    // --- GPU initialisation (Vulkan) ---
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("cardinal-xr: failed to find a Vulkan adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("cardinal_xr_device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("cardinal-xr: failed to create wgpu device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    // Send GPU handles to the cardinal thread.
    cmd_tx
        .send(cardinal_core::cardinal_thread::Command::InitGpu {
            device: device.clone(),
            queue: queue.clone(),
        })
        .expect("cardinal-xr: failed to send InitGpu command");

    // Fetch the module catalog.
    let (catalog_tx, catalog_rx) = mpsc::channel();
    cmd_tx
        .send(cardinal_core::cardinal_thread::Command::GetCatalog(catalog_tx))
        .expect("cardinal-xr: failed to send GetCatalog command");
    let catalog = catalog_rx.recv().expect("cardinal-xr: failed to receive catalog");
    eprintln!("cardinal-xr: catalog has {} entries", catalog.len());

    // Build hand menu state from catalog.
    let mut hand_menu = HandMenuState::from_catalog(&catalog);

    // --- Stardust XR connection and event loop ---
    let rt = tokio::runtime::Runtime::new().expect("cardinal-xr: failed to create tokio runtime");
    rt.block_on(async {
        let mut client = stardust_xr_fusion::client::Client::connect()
            .await
            .expect("cardinal-xr: failed to connect to Stardust XR server");

        eprintln!("cardinal-xr: connected to Stardust XR server");

        // Create workspace parented to the Stardust client root.
        let mut workspace = Workspace::new(
            client.get_root(),
            catalog,
            cmd_tx,
            render_rx,
        );

        client
            .sync_event_loop(|client, _flow| {
                while let Some(root_event) = client.get_root().recv_root_event() {
                    match root_event {
                        stardust_xr_fusion::root::RootEvent::Ping { response } => {
                            response.send_ok(());
                        }
                        stardust_xr_fusion::root::RootEvent::Frame { info } => {
                            workspace.frame_update(info.delta);

                            // Drain spawn requests from hand menu and spawn modules.
                            for (plugin_slug, model_slug) in hand_menu.spawn_requests.drain(..) {
                                // TODO: use right hand's pointing ray for spawn position
                                let spawn_pos = glam::Vec3::new(0.0, 0.0, -crate::constants::MODULE_SPAWN_DISTANCE_M);
                                workspace.spawn_module(
                                    plugin_slug,
                                    model_slug,
                                    spawn_pos,
                                    glam::Quat::IDENTITY,
                                );
                            }

                            // TODO: detect palm-up gesture and call
                            //       hand_menu.update_palm_visibility(palm_up_amount)
                            // TODO: update projectors with latest render textures
                        }
                        stardust_xr_fusion::root::RootEvent::SaveState { response } => {
                            response.send_ok(stardust_xr_fusion::root::ClientState::default());
                        }
                    }
                }
            })
            .await
            .expect("cardinal-xr: event loop error");
    });
}
