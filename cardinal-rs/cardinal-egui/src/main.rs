mod app;
mod wgpu_app;

use winit::event_loop::EventLoop;

fn main() {
    let sample_rate = cardinal_core::audio::cpal_sample_rate().unwrap_or(48000.0);
    eprintln!("Audio sample rate: {sample_rate} Hz");

    let (cmd_tx, render_rx) = cardinal_core::cardinal_thread::spawn_cardinal_thread(sample_rate);
    let audio_stream = cardinal_core::audio::start_audio_stream();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut wgpu_app = wgpu_app::WgpuApp::new(cmd_tx, render_rx, audio_stream);
    event_loop.run_app(&mut wgpu_app).expect("event loop error");
}
