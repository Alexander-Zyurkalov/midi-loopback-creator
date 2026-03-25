use coremidi::{Client, OSStatus, Properties, VirtualSource};

pub struct MIDILoopback {
    client: Client,
    source: Option<VirtualSource>,
    name: String,
    unique_id: u32,
}

impl MIDILoopback {
    pub fn new(name: &str) -> Result<Self, OSStatus> {
        let client = Client::new(name)?;

        let unique_id = match Self::make_unique_id() {
            Some(id) => id,
            _ => return Err(3), // replace it with some text with anyhow
        };
        let source = Some(Self::make_source(&client, name, unique_id)?);
        let name = name.to_string();
        Ok(MIDILoopback {
            client,
            source,
            name,
            unique_id,
        })
    }

    pub fn rename(&mut self, name: &str) -> Result<(), OSStatus> {
        self.source = None;
        self.source = Some(Self::make_source(&self.client, name, self.unique_id)?);
        Ok(())
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_unique_id(&self) -> u32 {
        self.unique_id
    }

    fn make_source(client: &Client, name: &str, unique_id: u32) -> Result<VirtualSource, OSStatus> {
        let source = client.virtual_source(name)?;
        source.set_property(&Properties::unique_id(), unique_id as i32)?;
        Ok(source)
    }

    fn make_unique_id() -> Option<u32> {
        'main_loop: for _ in 1..10000 {
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
    use crate::midi_loopback::MIDILoopback;
    use coremidi::PacketBuffer;

    #[test]
    fn create_source() {
        let midi_loopback = MIDILoopback::new("track123");
        assert!(
            midi_loopback.is_ok(),
            "Failed to create MIDI loopback: {:?}",
            midi_loopback.err()
        );
        let mut midi_loopback = midi_loopback.unwrap();

        let unique_id = midi_loopback.unique_id;
        assert!(midi_loopback.rename("new_name").is_ok());
        assert_eq!(midi_loopback.unique_id, unique_id);
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
