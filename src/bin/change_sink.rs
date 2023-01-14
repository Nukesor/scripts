//! Small convenience script to quickly change the output sink.
//! It also moves all current outputs to that sink as well.
//!
//! This is currently used by me via shortcuts.
use anyhow::Result;
use clap::{ArgAction, Parser, ValueEnum};

use log::{debug, warn};
use script_utils::logging;
use script_utils::schemas::pw_dump::*;
use script_utils::{exec::Cmd, some_or_continue};
use strum_macros::Display;

#[derive(Parser, Debug)]
#[clap(
    name = "change_sink",
    about = "Change the current sink to the specified device",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// The audio sink that should be switched to.
    #[clap(value_enum)]
    pub target: Target,
}

#[derive(Parser, ValueEnum, Display, Copy, Clone, Debug)]
enum Target {
    Hdmi,
    BuiltIn,
    Xonar,
    Unitek,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    // Get current pipewire state.
    let capture = Cmd::new("pw-dump").run_success()?;
    let devices: Vec<Device> = serde_json::from_str(&capture.stdout_str())?;

    logging::init_logger(args.verbose);

    // Run through all devices and find the one we desire.
    for device in devices {
        let info = some_or_continue!(device.info);
        let props = some_or_continue!(info.props);
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

        let description = some_or_continue!(props.node_description);
        debug!("Found device {device_id}: {description}");

        // Check if we find a device for the given name.
        let device_found = match args.target {
            Target::Hdmi => description.contains("HDMI"),
            Target::BuiltIn => description.starts_with("Built-in"),
            Target::Xonar => description.contains("Xonar Essence STX II"),
            Target::Unitek => description.contains("Unitek Y-247A"),
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
        let capture = Cmd::new("pactl list short sink-inputs").run_success()?;

        let input_ids: Vec<String> = capture
            .stdout_str()
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| line.split('\t').next().map(|id| id.to_string()))
            .collect();

        debug!("Input Ids: {input_ids:?}");

        for id in input_ids {
            let result = Cmd::new(format!("pactl move-sink-input {id} {device_id}")).run_success();
            if let Err(err) = result {
                warn!("Failed to switch input {id} to new sink: {err:?}");
            };
        }

        Cmd::new(format!(
            "notify-send --expire-time=2000 'Changed sink to {}'",
            args.target
        ))
        .run_success()?;

        return Ok(());
    }

    println!("Couldn't find specified target sink.");

    Ok(())
}
