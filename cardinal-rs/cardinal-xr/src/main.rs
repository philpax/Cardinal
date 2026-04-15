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
