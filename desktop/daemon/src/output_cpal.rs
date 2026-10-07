//! Output through cpal (WASAPI on Windows).
//!
//! cpal pulls samples from a callback, so the mixer pushes blocks into a small queue
//! and blocks while it is full: the same back-pressure a blocking PulseAudio write
//! gives. The device's own rate and channel count are matched in the callback, since
//! shared-mode WASAPI only accepts its mix format.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use super::{OutputDevice, BUFFER_MS, CHANNELS, RATE};

const QUEUE_SAMPLES: usize = (RATE as u64 * BUFFER_MS / 1000) as usize * CHANNELS as usize;

#[derive(Default)]
struct Queue {
    samples: VecDeque<f32>,
    failed: bool,
}

#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    cv: Condvar,
}

pub struct Output {
    _stream: cpal::Stream,
    shared: Arc<Shared>,
}

impl Output {
    /// Queues one block of interleaved stereo at `RATE`, blocking while the queue is
    /// full. False if the device failed or stopped consuming audio.
    pub fn write(&self, block: &[f32]) -> bool {
        let limit = QUEUE_SAMPLES.max(block.len());
        let mut q = self.shared.queue.lock().unwrap();
        while !q.failed && q.samples.len() + block.len() > limit {
            let (guard, timeout) = self.shared.cv.wait_timeout(q, Duration::from_secs(2)).unwrap();
            q = guard;
            if timeout.timed_out() {
                return false; // device stalled (unplugged, endpoint changed); reconnect
            }
        }
        if q.failed {
            return false;
        }
        q.samples.extend(block);
        true
    }

    /// Waits for queued audio to finish playing.
    pub fn drain(self) {
        let wait = Duration::from_millis(BUFFER_MS * 3);
        let q = self.shared.queue.lock().unwrap();
        let _ = self.shared.cv.wait_timeout_while(q, wait, |q| !q.failed && !q.samples.is_empty());
    }
}

fn find_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    host.output_devices().ok()?.find(|d| d.name().is_ok_and(|n| n == name))
}

pub fn connect(device: Option<&str>) -> Option<Output> {
    let host = cpal::default_host();
    // A saved device may have been unplugged; fall back to the system default.
    let device = device.and_then(|name| find_device(&host, name)).or_else(|| host.default_output_device())?;
    let supported = device.default_output_config().ok()?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let shared = Arc::new(Shared::default());
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, &shared),
        SampleFormat::I16 => build::<i16>(&device, &config, &shared),
        SampleFormat::I32 => build::<i32>(&device, &config, &shared),
        SampleFormat::U16 => build::<u16>(&device, &config, &shared),
        _ => None,
    }?;
    stream.play().ok()?;
    Some(Output { _stream: stream, shared })
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: &Arc<Shared>,
) -> Option<cpal::Stream> {
    let channels = config.channels as usize;
    // Linear resampling from RATE: walk `pos` between the previous and current frame.
    let step = RATE as f64 / config.sample_rate.0 as f64;
    let (mut pos, mut prev, mut cur) = (1.0f64, [0f32; 2], [0f32; 2]);

    let feed = shared.clone();
    let on_error = shared.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut q = feed.queue.lock().unwrap();
                for frame in data.chunks_exact_mut(channels) {
                    while pos >= 1.0 {
                        prev = cur;
                        // Underrun plays silence rather than stalling the device.
                        cur = [q.samples.pop_front().unwrap_or(0.0), q.samples.pop_front().unwrap_or(0.0)];
                        pos -= 1.0;
                    }
                    let t = pos as f32;
                    let l = prev[0] + (cur[0] - prev[0]) * t;
                    let r = prev[1] + (cur[1] - prev[1]) * t;
                    pos += step;
                    match frame {
                        [mono] => *mono = T::from_sample((l + r) * 0.5),
                        [a, b, rest @ ..] => {
                            *a = T::from_sample(l);
                            *b = T::from_sample(r);
                            rest.fill(T::EQUILIBRIUM);
                        }
                        [] => {}
                    }
                }
                drop(q);
                feed.cv.notify_all();
            },
            move |_| {
                on_error.queue.lock().unwrap().failed = true;
                on_error.cv.notify_all();
            },
            None,
        )
        .ok()
}

/// Lists output devices. Names double as ids since cpal has nothing more stable.
pub fn list_devices() -> Vec<OutputDevice> {
    let Ok(devices) = cpal::default_host().output_devices() else { return vec![] };
    devices
        .filter_map(|d| d.name().ok())
        .map(|name| OutputDevice { description: name.clone(), name })
        .collect()
}
