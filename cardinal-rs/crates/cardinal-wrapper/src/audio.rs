use cardinal_core as cc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn cpal_sample_rate() -> Option<f32> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(config.sample_rate().0 as f32)
}

pub fn start_audio_stream() -> Option<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;

    eprintln!(
        "Audio device: {}, config: {:?}",
        device.name().unwrap_or_default(),
        config
    );

    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &cpal::StreamConfig {
                    channels: channels as u16,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_callback(data, channels);
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .ok()?,
        _ => {
            eprintln!("Unsupported sample format: {:?}", config.sample_format());
            return None;
        }
    };

    stream.play().ok()?;
    eprintln!("Audio stream started");
    Some(stream)
}

fn audio_callback(output: &mut [f32], channels: usize) {
    let frames = output.len() / channels;
    const MAX: usize = 8192;
    let frames = frames.min(MAX);
    let mut stereo_buf = [0.0f32; MAX * 2];

    cc::audio_process(frames, None, &mut stereo_buf[..frames * 2]);

    for i in 0..frames {
        let l = stereo_buf[i * 2];
        let r = stereo_buf[i * 2 + 1];
        let base = i * channels;
        if channels >= 1 {
            output[base] = l;
        }
        if channels >= 2 {
            output[base + 1] = r;
        }
        for ch in 2..channels {
            output[base + ch] = 0.0;
        }
    }
    let written = frames * channels;
    for s in &mut output[written..] {
        *s = 0.0;
    }
}
