use std::sync::mpsc;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowAttributes};

use crate::app::App;
use cardinal_core::cardinal_thread::{Command, RenderResult};

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
}

pub struct WgpuApp {
    gpu: Option<GpuState>,
    app: Option<App>,
    cmd_tx: mpsc::Sender<Command>,
    render_rx: Option<mpsc::Receiver<RenderResult>>,
    _audio_stream: Option<cpal::Stream>,
}

impl WgpuApp {
    pub fn new(
        cmd_tx: mpsc::Sender<Command>,
        render_rx: mpsc::Receiver<RenderResult>,
        audio_stream: Option<cpal::Stream>,
    ) -> Self {
        Self {
            gpu: None,
            app: None,
            cmd_tx,
            render_rx: Some(render_rx),
            _audio_stream: audio_stream,
        }
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("Cardinal")
            .with_inner_size(winit::dpi::LogicalSize::new(1400.0, 800.0));

        let window = Arc::new(event_loop.create_window(window_attrs).expect("failed to create window"));

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("failed to find a suitable adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("cardinal_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .expect("failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let _ = self.cmd_tx.send(Command::InitGpu {
            device: device.clone(),
            queue: queue.clone(),
        });

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        let render_rx = self.render_rx.take().expect("render_rx already taken");
        let app = App::new(self.cmd_tx.clone(), render_rx);

        self.gpu = Some(GpuState {
            window,
            surface,
            surface_config,
            device,
            queue,
            egui_renderer,
            egui_state,
            egui_ctx,
        });
        self.app = Some(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(gpu), Some(app)) = (self.gpu.as_mut(), self.app.as_mut()) else {
            return;
        };

        let response = gpu.egui_state.on_window_event(&gpu.window, &event);
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    gpu.surface_config.width = new_size.width;
                    gpu.surface_config.height = new_size.height;
                    gpu.surface.configure(&gpu.device, &gpu.surface_config);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                app.poll_render_results(&mut gpu.egui_renderer, &gpu.device);
                app.request_renders();

                let raw_input = gpu.egui_state.take_egui_input(&gpu.window);
                #[allow(deprecated)]
                let full_output = gpu.egui_ctx.run(raw_input, |ctx| {
                    app.ui(ctx);
                });

                gpu.egui_state.handle_platform_output(&gpu.window, full_output.platform_output);

                let paint_jobs = gpu.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [gpu.surface_config.width, gpu.surface_config.height],
                    pixels_per_point: gpu.egui_ctx.pixels_per_point(),
                };

                let surface_texture = match gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(tex)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        gpu.surface.configure(&gpu.device, &gpu.surface_config);
                        gpu.window.request_redraw();
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        eprintln!("wgpu surface validation error");
                        return;
                    }
                };

                let surface_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_encoder"),
                });

                for (id, delta) in &full_output.textures_delta.set {
                    gpu.egui_renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
                }

                gpu.egui_renderer.update_buffers(
                    &gpu.device,
                    &gpu.queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                {
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui_render"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &surface_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.1,
                                    b: 0.12,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        ..Default::default()
                    });
                    let mut render_pass = render_pass.forget_lifetime();
                    gpu.egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
                }

                gpu.queue.submit(std::iter::once(encoder.finish()));
                surface_texture.present();

                for id in &full_output.textures_delta.free {
                    gpu.egui_renderer.free_texture(id);
                }

                gpu.window.request_redraw();
            }
            _ => {}
        }
    }
}
