use coremidi::{Client, OSStatus, VirtualSource};

pub struct MIDILoopback {
    client: Client,
    source: VirtualSource,
    name: String
}

impl MIDILoopback {
    pub fn new(name: &str) -> Result<Self, OSStatus> {
        let client = Client::new(name)?;
        let source = client.virtual_source(name)?;
        let name = name.to_string();
        Ok(MIDILoopback { client, source, name })
    }
    pub fn rename(&self, name: &str) -> Result<Self, OSStatus> {
        self.source.flush()?;
        MIDILoopback::new(name)
    }
    pub fn get_name(&self) -> &str {
       self.name.as_str()
    }
}
#[cfg(test)]
mod tests {
    use coremidi::PacketBuffer;
    use super::*;


    #[test]
    fn create_source() {
        let midi_loopback = MIDILoopback::new("track123");
        assert!(
            midi_loopback.is_ok(),
            "Failed to create MIDI loopback: {:?}",
            midi_loopback.err()
        );
        let midi_loopback = midi_loopback.unwrap();
        let note_on = create_note_on(0, 64, 127);
        assert!(midi_loopback.source.received(&note_on).is_ok());
    }

    fn create_note_on(channel: u8, note: u8, velocity: u8) -> PacketBuffer {
        let data = &[0x90 | (channel & 0x0f), note & 0x7f, velocity & 0x7f];
        PacketBuffer::new(0, data)
    }
}
