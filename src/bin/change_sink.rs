//! Small convenience script to quickly change the output sink.
//! It also moves all current outputs to that sink as well.
//!
//! This is currently used by me via shortcuts.
//! Needed binaries:
//! - pw-dump
//! - pactl
use anyhow::{Context, Result};
use clap::{ArgAction, Parser};

use log::{debug, info, warn};
use script_utils::logging;
use script_utils::notify::*;
use script_utils::ring::Ring;
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

    /// The command to execute.
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Display, Clone, Debug, PartialEq)]
pub enum Command {
    // Go to the next device
    Next,
    // Go to the previous device
    Prev,
    // Switch to the default built-in device.
    BuiltIn,
    // Switch to a specific target
    Target { target: String },
    // List all devices
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilteredDevice {
    id: usize,
    description: String,
    /// The raw node name, which is also used by alsa.
    node_name: String,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    let devices = get_sink_list()?;

    // Only print the device list and return early.
    if args.command == Command::List {
        if devices.is_empty() {
            println!("Found no sinks");
        } else {
            println!("Found the following sinks:");
            for device in devices.iter() {
                println!("{}: {}", device.id, device.description);
            }
        }

        return Ok(());
    }

    // Get the target sink, return if we cannot find any.
    let Some(device) = get_target_sink(devices, &args.command)? else {
        critical_notify(
            2000,
            format!(
                "Could not find target sink for command: {:#?}",
                &args.command
            ),
        )?;
        return Ok(());
    };

    // Set the default sink.
    Cmd::new(format!("pactl set-default-sink {}", device.id)).run_success()?;

    move_inputs_to_sink(device.id)?;

    // Inform the user about the sink we just switched to.
    notify(2000, format!("Changed sink to {}", device.description))?;

    Ok(())
}

/// Try to determine the id and description of the targeted sink.
/// May return None if the target sink cannot be found.
pub fn get_target_sink(
    devices: Vec<FilteredDevice>,
    command: &Command,
) -> Result<Option<FilteredDevice>> {
    // Determine the current sink.
    let output = Cmd::new("pactl get-default-sink")
        .run_success()
        .context("Failed to find default sink")?;
    let current_sink_name = output.stdout_str().trim().to_owned();

    // Initialize the device ring for easy iteration to the next/previous device.
    let mut ring = Ring::new(devices.clone())?;
    // Move the cursor to the current device.
    // If `None` is found, the device falls back to `0`.
    // TODO: Handle this?
    ring.find(|(_, device)| device.node_name == current_sink_name);

    // Check if we find a device for the given name.
    let device = match command {
        Command::BuiltIn => devices
            .iter()
            .find(|device| device.description.starts_with("Built-in")),
        Command::Target { target } => devices
            .iter()
            .find(|device| device.description.starts_with(target)),
        Command::Prev => Some(ring.prev()),
        Command::Next => Some(ring.next()),
        _ => return Ok(None),
    };

    Ok(device.cloned())
}

/// Get a map of all input devices.
/// Map has structure `device_id -> device description`.
pub fn get_sink_list() -> Result<Vec<FilteredDevice>> {
    // Get current pipewire state.
    let capture = Cmd::new("pw-dump").run_success()?;
    let devices: Vec<Device> = serde_json::from_str(&capture.stdout_str())?;

    let mut input_devices = Vec::new();

    // Run through all devices and find the one we desire.
    for device in devices.iter() {
        let info = some_or_continue!(&device.info);
        let props = some_or_continue!(&info.props);
        // We are only interested in devices of Audio/Sink type.
        if props.media_class != Some("Audio/Sink".to_string()) {
            continue;
        }

        let device_id = props.object_serial;
        let description = some_or_continue!(&props.node_description);
        let node_name = some_or_continue!(&props.node_name);

        info!("Found device {device_id}: {description}");
        debug!("Raw: {:#?}", device);

        let filtered_device = FilteredDevice {
            id: device_id,
            node_name: node_name.clone(),
            description: description.clone(),
        };
        input_devices.push(filtered_device);
    }

    Ok(input_devices)
}

/// Search all inputs and switch them over to the new device.
pub fn move_inputs_to_sink(device_id: usize) -> Result<()> {
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

    Ok(())
}
