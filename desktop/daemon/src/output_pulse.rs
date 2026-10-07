//! Output to PulseAudio, which on current desktops is PipeWire's pulse server.

use libpulse_binding as pulse;
use libpulse_simple_binding::Simple;
use pulse::sample::{Format, Spec};
use pulse::stream::Direction;

use super::{OutputDevice, BUFFER_MS, CHANNELS, RATE};

pub struct Output(Simple);

impl Output {
    /// Queues one block, blocking while the server's buffer is full. False if the
    /// server went away.
    pub fn write(&self, block: &[f32]) -> bool {
        // SAFETY: f32 has no invalid bit patterns and the slice covers exactly `block`.
        let bytes = unsafe {
            std::slice::from_raw_parts(block.as_ptr() as *const u8, std::mem::size_of_val(block))
        };
        self.0.write(bytes).is_ok()
    }

    /// Waits for queued audio to finish playing.
    pub fn drain(self) {
        let _ = self.0.drain();
    }
}

pub fn connect(device: Option<&str>) -> Option<Output> {
    let spec = Spec { format: Format::FLOAT32NE, rate: RATE, channels: CHANNELS as u8 };
    let bytes = |ms: u64| spec.usec_to_bytes(pulse::time::MicroSeconds(ms * 1000)) as u32;
    let attr = pulse::def::BufferAttr {
        maxlength: u32::MAX,
        tlength: bytes(BUFFER_MS),
        prebuf: u32::MAX,
        minreq: bytes(BUFFER_MS / 3),
        fragsize: u32::MAX,
    };
    let open = |dev| Simple::new(None, "Moodist", Direction::Playback, dev, "Ambient sounds", &spec, None, Some(&attr)).ok();
    // A saved device may have been unplugged; fall back to the system default.
    open(device).or_else(|| device.and_then(|_| open(None))).map(Output)
}

/// Lists PulseAudio/PipeWire sinks. Returns an empty list if no server is reachable.
pub fn list_devices() -> Vec<OutputDevice> {
    use pulse::callbacks::ListResult;
    use pulse::context::{Context, FlagSet, State};
    use pulse::mainloop::standard::{IterateResult, Mainloop};
    use pulse::operation::State as OpState;
    use std::cell::RefCell;
    use std::rc::Rc;

    let Some(mut mainloop) = Mainloop::new() else { return vec![] };
    let Some(mut context) = Context::new(&mainloop, "Moodist") else { return vec![] };
    if context.connect(None, FlagSet::NOAUTOSPAWN, None).is_err() {
        return vec![];
    }
    loop {
        if let IterateResult::Quit(_) | IterateResult::Err(_) = mainloop.iterate(true) {
            return vec![];
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => return vec![],
            _ => {}
        }
    }

    let devices = Rc::new(RefCell::new(Vec::new()));
    let sink = devices.clone();
    let op = context.introspect().get_sink_info_list(move |res| {
        if let ListResult::Item(info) = res {
            if let Some(name) = &info.name {
                sink.borrow_mut().push(OutputDevice {
                    name: name.to_string(),
                    description: info.description.as_deref().unwrap_or(name).to_string(),
                });
            }
        }
    });
    while op.get_state() == OpState::Running {
        if let IterateResult::Quit(_) | IterateResult::Err(_) = mainloop.iterate(true) {
            break;
        }
    }
    context.disconnect();
    devices.take()
}
