mod constants;
mod hand_menu;
mod math;

use std::sync::mpsc;
use std::sync::Arc;

use stardust_xr_fusion::root::RootAspect as _;

fn main() {
    eprintln!("cardinal-xr: starting");

    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, _render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
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

    // --- Stardust XR connection and event loop ---
    let rt = tokio::runtime::Runtime::new().expect("cardinal-xr: failed to create tokio runtime");
    rt.block_on(async {
        let mut client = stardust_xr_fusion::client::Client::connect()
            .await
            .expect("cardinal-xr: failed to connect to Stardust XR server");

        eprintln!("cardinal-xr: connected to Stardust XR server");

        client
            .sync_event_loop(|client, _flow| {
                while let Some(root_event) = client.get_root().recv_root_event() {
                    match root_event {
                        stardust_xr_fusion::root::RootEvent::Ping { response } => {
                            response.send_ok(());
                        }
                        stardust_xr_fusion::root::RootEvent::Frame { info: _ } => {
                            // TODO: per-frame rendering logic
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
