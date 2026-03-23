use crate::midi_loopback::MIDILoopback;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::*;
use std::sync::{Arc, Mutex};

mod midi_loopback;

#[derive(Params, Default)]
struct LoopbackMidiParams {
    #[persist = "loopback_name"]
    loopback_name: Arc<Mutex<String>>,
}

struct LoopbackMidiPlugin {
    midi_loopback: Option<MIDILoopback>,
    params: Arc<LoopbackMidiParams>,
}

impl Default for LoopbackMidiPlugin {
    fn default() -> Self {
        LoopbackMidiPlugin {
            midi_loopback: None,
            params: Arc::new(LoopbackMidiParams::default()),
        }
    }
}

impl Plugin for LoopbackMidiPlugin {
    const NAME: &'static str = "Loopback MIDI";
    const VENDOR: &'static str = "Alexandr Zyurkalov";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = "0.0.2";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.midi_loopback = match MIDILoopback::new("NewLoopback") {
            Ok(loopback) => Some(loopback),
            Err(err) => {
                nih_log!("Failed to create MIDI loopback: {}", err);
                return false;
            }
        };
        self.params.loopback_name.set("NewLoopback".to_string());
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(event) = context.next_event() {
            while let Some(event) = context.next_event() {
                match event {
                    NoteEvent::NoteOn { timing, note, velocity, channel, .. } => {
                        // timing = sample offset from start of this buffer
                        nih_log!("Note ON at sample {}: ch={} note={}", timing, channel, note);
                    }
                    _ => {}
                }
            }
        }
        ProcessStatus::KeepAlive
    }
}
impl Vst3Plugin for LoopbackMidiPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"AZLoopbackMIDI\x01\x01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Tools, Vst3SubCategory::Instrument];
}
nih_export_vst3!(LoopbackMidiPlugin);
