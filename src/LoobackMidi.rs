use coremidi::{Client, OSStatus, PacketBuffer};

pub struct MIDILoopback {
    name: String,
    client: Client,
}

impl MIDILoopback {

    pub fn new (name: impl Into<String>) -> Result<Self, OSStatus> {
        let name = name.into();
        let client = Client::new(name.as_str())?;
        Ok(   MIDILoopback { name, client}  )
    }
    pub fn rename(mut self, name: impl Into<String> ) {
        self.name = name.into();
    }

}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate() {
        let midi_loopback = MIDILoopback::new("track123");
        assert!(midi_loopback.is_ok());
    }

}