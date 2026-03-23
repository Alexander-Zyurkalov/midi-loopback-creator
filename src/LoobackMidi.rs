use coremidi::{Client, OSStatus, PacketBuffer, VirtualSource};

pub struct MIDILoopback {
    client: Client,
    source: VirtualSource,
}

impl MIDILoopback {
    pub fn new(name: &str) -> Result<Self, OSStatus> {
        let client = Client::new(name)?;
        let source = client.virtual_source(name)?;
        Ok(MIDILoopback { client, source })
    }
    pub fn rename(mut self, name: &str) {}
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_source() {
        let midi_loopback = MIDILoopback::new("track123");
        assert!(midi_loopback.is_ok());
        let midi_loopback = midi_loopback.unwrap();
        let note_on = create_note_on(0, 64, 127);
        assert!(midi_loopback.source.received(&note_on).is_ok());
    }

    fn create_note_on(channel: u8, note: u8, velocity: u8) -> PacketBuffer {
        let data = &[0x90 | (channel & 0x0f), note & 0x7f, velocity & 0x7f];
        PacketBuffer::new(0, data)
    }
}