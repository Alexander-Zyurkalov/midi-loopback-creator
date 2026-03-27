use anyhow::{anyhow, Result};
use coremidi::{Client, OSStatus, Properties, VirtualDestination};

pub struct MIDILoopback {
    client: Client,
    destination: Option<VirtualDestination>,
    name: String,
    unique_id: u32,
    instrument_id: u8,
}

fn describe_os_status(status: OSStatus) -> String {
    let description = match status {
        -10830 => "Invalid client (kMIDIInvalidClient)",
        -10831 => "Invalid port (kMIDIInvalidPort)",
        -10832 => "Wrong endpoint type (kMIDIWrongEndpointType)",
        -10833 => "No connection (kMIDINoConnection)",
        -10834 => "Unknown endpoint (kMIDIUnknownEndpoint)",
        -10835 => "Unknown property (kMIDIUnknownProperty)",
        -10836 => "Wrong property type (kMIDIWrongPropertyType)",
        -10837 => "No current MIDI setup (kMIDINoCurrentSetup)",
        -10838 => "Message send failed (kMIDIMessageSendErr)",
        -10839 => "MIDI server failed to start (kMIDIServerStartErr)",
        -10840 => "MIDI setup format error (kMIDISetupFormatErr)",
        -10841 => "Called from wrong thread (kMIDIWrongThread)",
        -10842 => "Object not found (kMIDIObjectNotFound)",
        -10843 => "Unique ID already taken (kMIDIIDNotUnique)",
        -10844 => "Operation not permitted (kMIDINotPermitted)",
        _ => "Unknown error",
    };
    format!("CoreMIDI error {}: {}", status, description)
}

fn midi_err(status: OSStatus) -> anyhow::Error {
    anyhow!(describe_os_status(status))
}

impl MIDILoopback {
    pub fn new(name: &str, instrument_id: u8) -> Result<(Self, u32)> {
        let client = Client::new(name).map_err(midi_err)?;

        let unique_id = Self::make_unique_id().ok_or_else(|| {
            anyhow!(
                "Failed to generate a unique MIDI ID after 10000 attempts — \
                 too many MIDI sources registered in the system"
            )
        })?;

        let destination = Some(Self::make_loopback(
            &client,
            name,
            unique_id,
            instrument_id,
        )?);
        let name = name.to_string();
        Ok((
            MIDILoopback {
                client,
                destination,
                name,
                unique_id,
                instrument_id,
            },
            unique_id,
        ))
    }

    pub fn rename(&mut self, name: &str) -> Result<()> {
        // TODO also rename destination
        // TODO also apply a unique ID for it
        // https://claude.ai/chat/debac70c-e5d5-4c24-b16c-1cd958f06df8
        self.destination = None;
        self.destination = Some(Self::make_loopback(
            &self.client,
            name,
            self.unique_id,
            self.instrument_id,
        )?);
        self.name = name.to_string();
        Ok(())
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_unique_id(&self) -> u32 {
        self.unique_id
    }

    fn make_loopback(
        client: &Client,
        name: &str,
        source_id: u32,
        instrument_id: u8,
    ) -> Result<VirtualDestination> {
        let source = client.virtual_source(name).map_err(midi_err)?;
        source
            .set_property(&Properties::unique_id(), source_id as i32)
            .map_err(midi_err)?;

        let destination = client
            .virtual_destination(name, move |packet_list| {
                let _ = source.received(packet_list);
            })
            .map_err(midi_err)?;
        let magic_number_for_a_destination_id = 58828300;
        let destination_id = magic_number_for_a_destination_id + instrument_id as i32;
        destination
            .set_property(&Properties::unique_id(), destination_id)
            .map_err(midi_err)?;

        Ok(destination)
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
            for destination in coremidi::Destinations {
                match destination.unique_id() {
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
    use super::*;

    #[test]
    fn create_destination() {
        let midi_loopback = MIDILoopback::new("Instrument1", 1);
        assert!(
            midi_loopback.is_ok(),
            "Failed to create MIDI loopback: {:?}",
            midi_loopback.err()
        );
        let (mut loopback, unique_id) = midi_loopback.unwrap();

        assert_eq!(loopback.unique_id, unique_id);
        assert!(loopback.rename("new_name").is_ok());
        assert_eq!(loopback.unique_id, unique_id);
    }

    #[test]
    fn make_unique_id_test() {
        assert!(MIDILoopback::make_unique_id().is_some())
    }
}
