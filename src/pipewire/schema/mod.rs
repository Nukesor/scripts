use anyhow::{Context, Result};
use serde_json::Value;

use crate::prelude::Cmd;
pub use device::*;
pub use node::*;

pub mod device;
pub mod node;

/// Parse the output of `pw_dump` and return a list of devices and nodes.
pub fn parse_pw_dump() -> Result<(Vec<Device>, Vec<Node>)> {
    let mut devices = Vec::new();
    let mut nodes = Vec::new();

    // First off, get the raw serde json representation.
    // There're many pipewire object types in the output we aren't interested in.
    // We're going to filter out only those we want.
    let capture = Cmd::new("pw-dump").run_success()?;
    let objects: Vec<Value> = serde_json::from_str(&capture.stdout_str())?;

    for object in objects {
        // The output should only contain objects. Everything else could be ignored anyway.
        let Value::Object(values) = &object else {
            continue;
        };

        match values.get("type").unwrap() {
            Value::String(object_type) => match object_type.as_str() {
                "PipeWire:Interface:Node" => {
                    // There are a few default drivers we're really not interested in.
                    // We're only interested in those nodes that have a valid associated device.
                    // Such nodes have a `node.info.props.device.id` property.
                    let Some(Value::Object(infos)) = values.get("info") else {
                        continue;
                    };
                    let Some(Value::Object(props)) = infos.get("props") else {
                        continue;
                    };
                    let Some(Value::Number(_device_id)) = props.get("device.id") else {
                        continue;
                    };

                    // On top of that, we must have a media_class, otherwise we cannot do anything
                    // with it anyway.
                    let Some(Value::String(media_class)) = props.get("media.class") else {
                        continue;
                    };
                    // Furthermore, we're only interested in audio sinks.
                    if !media_class.starts_with("Audio") {
                        continue;
                    }

                    // We now know that there's a device id in there, so let's include that node.
                    let node: Node = serde_json::from_value(object.clone())
                        .context(format!("Failed to parse node: {:#?}", object.clone()))?;
                    nodes.push(node);
                }
                "PipeWire:Interface:Device" => {
                    let device: Device = serde_json::from_value(object.clone())
                        .context(format!("Failed to parse device: {:#?}", object.clone()))?;
                    devices.push(device);
                }
                _ => continue,
            },
            _ => continue,
        }
    }

    Ok((devices, nodes))
}
