use coremidi::{Client, OSStatus, Properties, VirtualSource};

pub struct MIDILoopback {
    client: Client,
    source: VirtualSource,
    name: String,
}

impl MIDILoopback {
    pub fn new(name: &str) -> Result<Self, OSStatus> {
        let client = Client::new(name)?;
        let source = client.virtual_source(name)?;

        let id = match Self::make_unique_id() {
            Some(id) => id,
            _ => return Err(3), // replace it with some text with anyhow
        };

        source.set_property(&Properties::unique_id(), id as i32)?;
        let name = name.to_string();
        Ok(MIDILoopback {
            client,
            source,
            name,
        })
    }
    pub fn rename(&self, name: &str) -> Result<Self, OSStatus> {
        self.source.flush()?;
        MIDILoopback::new(name)
    }
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    fn make_unique_id() -> Option<u32> {
        'main_loop: for _ in  1..10000  {
            let random_num = rand::random::<u32>();
            for source in coremidi::Sources {
                match source.unique_id() {
                    Some(id) if random_num == id => continue 'main_loop,
                    _ => {}
                }
            }
            return Some(random_num);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use coremidi::PacketBuffer;
    use crate::midi_loopback::MIDILoopback;

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

    #[test]
    fn make_unique_id_test() {
        assert!(MIDILoopback::make_unique_id().is_some())
    }
}
