use anyhow::Result;
use clap::{ArgEnum, Parser};

use script_utils::pw_dump::*;
use script_utils::{process::Cmd, unwrap_or_continue};

#[derive(Parser, Debug)]
#[clap(
    name = "change_sink",
    about = "Change the current sink to the specified device",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// The audio sink that should be switched to.
    #[clap(arg_enum)]
    pub target: Target,
}

#[derive(Parser, ArgEnum, Copy, Clone, Debug)]
enum Target {
    HDMI,
    BuiltIn,
    Xonar,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    // Get current pipewire state.
    let capture = Cmd::new("pw-dump").run_success()?;
    let devices: Vec<Device> = serde_json::from_str(&capture.stdout_str())?;

    // Run through all devices and find the one we desire.
    for device in devices {
        let info = unwrap_or_continue!(device.info);
        let props = unwrap_or_continue!(info.props);
        let device_id = props.object_serial;
        // We are only interested in Audio/Sink type devices.
        match props.media_class {
            None => continue,
            Some(class) => {
                if class != "Audio/Sink" {
                    continue;
                }
            }
        }

        let description = unwrap_or_continue!(props.node_description);
        //println!("Device {device_id}: {description}");

        // Check if we find a device for the given name.
        let device_found = match args.target {
            Target::HDMI => description.contains("HDMI"),
            Target::BuiltIn => description.starts_with("Built-in"),
            Target::Xonar => description.contains("Xonar"),
        };

        if !device_found {
            continue;
        }

        // Set the default sink.
        Cmd::new(format!("pactl set-default-sink {device_id}")).run_success()?;

        // Get all currently active sink inputs.
        // Output format looks like this:
        //
        // 188 56 187 PipeWire float32le 2ch 48000Hz
        //
        // We're interested in the first number.
        let capture = Cmd::new(format!("pactl list short sink-inputs")).run_success()?;

        let input_ids: Vec<String> = capture
            .stdout_str()
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| line.split('\t').nth(0).map(|id| id.to_string()))
            .collect();

        //println!("{input_ids:?}");

        for id in input_ids {
            Cmd::new(format!("pactl move-sink-input {id} {device_id}")).run_success()?;
        }

        return Ok(());
    }

    println!("Couldn't find specified target sink.");

    Ok(())
}
