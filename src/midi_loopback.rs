use anyhow::{anyhow, Result};
use coremidi::{
    Client, Destination, OSStatus, OutputPort, Properties, Protocol, VirtualDestination,
};

pub struct MIDILoopback {
    client: Client,
    destination: Option<VirtualDestination>,
    name: String,
    source_id: u32,
    destination_id: u32,
    additional_dest_ids: (Option<u32>, Option<u32>),
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
    pub fn new(
        name: &str,
        source_id: Option<u32>,
        destination_id: Option<u32>,
    ) -> Result<(Self, u32, u32)> {
        let client = Client::new(name).map_err(midi_err)?;

        let source_id = match source_id {
            Some(id) => id,
            None => Self::make_unique_id().ok_or_else(|| {
                anyhow!(
                    "Failed to generate a unique MIDI source ID after 10000 attempts — \
                     too many MIDI endpoints registered in the system"
                )
            })?,
        };

        let destination_id = match destination_id {
            Some(id) => id,
            None => Self::make_unique_id().ok_or_else(|| {
                anyhow!(
                    "Failed to generate a unique MIDI destination ID after 10000 attempts — \
                     too many MIDI endpoints registered in the system"
                )
            })?,
        };

        let destination = Some(Self::make_loopback(
            &client,
            name,
            source_id,
            destination_id,
            (None, None),
        )?);
        Ok((
            MIDILoopback {
                client,
                destination,
                name: name.to_string(),
                source_id,
                destination_id,
                additional_dest_ids: (None, None),
            },
            source_id,
            destination_id,
        ))
    }

    pub fn rename(&mut self, name: &str) -> Result<()> {
        self.destination = None;
        self.destination = Some(Self::make_loopback(
            &self.client,
            name,
            self.source_id,
            self.destination_id,
            self.additional_dest_ids,
        )?);
        self.name = name.to_string();
        Ok(())
    }

    pub fn set_additional_destinations(&mut self, id1: Option<u32>, id2: Option<u32>) -> Result<()> {
        self.additional_dest_ids = (id1, id2);
        self.destination = None;
        self.destination = Some(Self::make_loopback(
            &self.client,
            &self.name.clone(),
            self.source_id,
            self.destination_id,
            self.additional_dest_ids,
        )?);
        Ok(())
    }

    pub fn find_destination_id_by_name(name: &str) -> Option<u32> {
        coremidi::Destinations
            .into_iter()
            .find(|d| d.display_name().as_deref() == Some(name))
            .and_then(|d| d.unique_id())
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_unique_id(&self) -> u32 {
        self.source_id
    }

    fn make_loopback(
        client: &Client,
        name: &str,
        source_id: u32,
        destination_id: u32,
        additional_dest_ids: (Option<u32>, Option<u32>),
    ) -> Result<VirtualDestination> {
        let source = client.virtual_source(name).map_err(midi_err)?;
        source
            .set_property(&Properties::unique_id(), source_id as i32)
            .map_err(midi_err)?;

        let mut additional: Vec<(OutputPort, Destination)> = Vec::new();
        for id_opt in [additional_dest_ids.0, additional_dest_ids.1] {
            if let Some(id) = id_opt {
                let endpoint = coremidi::Destinations
                    .into_iter()
                    .find(|d| d.unique_id() == Some(id))
                    .ok_or_else(|| anyhow!("Additional destination with ID {} not found", id))?;
                let port = client.output_port(name).map_err(midi_err)?;
                additional.push((port, endpoint));
            }
        }

        let destination = client
            .virtual_destination_with_protocol(name, Protocol::Midi10, move |packet_list| {
                let _ = source.received(packet_list);
                for (port, endpoint) in &additional {
                    let _ = port.send(endpoint, packet_list);
                }
            })
            .map_err(midi_err)?;
        destination
            .set_property(&Properties::unique_id(), destination_id as i32)
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
        let midi_loopback = MIDILoopback::new("Instrument1", None, None);
        assert!(
            midi_loopback.is_ok(),
            "Failed to create MIDI loopback: {:?}",
            midi_loopback.err()
        );
        let (mut loopback, source_id, _destination_id) = midi_loopback.unwrap();

        assert_eq!(loopback.source_id, source_id);
        assert!(loopback.rename("new_name").is_ok());
        assert_eq!(loopback.source_id, source_id);
    }

    #[test]
    fn make_unique_id_test() {
        assert!(MIDILoopback::make_unique_id().is_some())
    }
}
