//! Small convenience script to quickly change the output sink.
//! It also moves all current outputs to that sink as well.
//!
//! This is currently used by me via shortcuts.
//! Needed binaries:
//! - pw-dump
//! - pactl
use anyhow::Result;
use clap::{ArgAction, Parser};
use script_utils::{exec::Cmd, logging, notify::*, pipewire::*};
use strum::Display;

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
    Previous,
    // Switch to the default built-in device.
    BuiltIn,
    // Switch to a specific target
    Target { target: String },
    // List all devices
    List,
}
fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    let device = match args.command {
        Command::Next => rotate_sink(Direction::Next)?,
        Command::Previous => rotate_sink(Direction::Previous)?,
        Command::BuiltIn => get_sinks()?
            .into_iter()
            .find(|device| device.info.props.node_description.starts_with("Built-in")),
        Command::Target { ref target } => get_sinks()?
            .into_iter()
            .find(|device| device.info.props.node_description.starts_with(target)),
        Command::List => {
            list_sinks()?;
            return Ok(());
        }
    };

    let Some(device) = device else {
        critical_notify(
            1500,
            format!(
                "Could not find target sink for command: {:#?}",
                &args.command
            ),
        )?;

        return Ok(());
    };

    switch_device(&device)?;

    Ok(())
}

/// Set the target device as the default sink.
/// Also take all inputs that're currently open and move them over to the target device.
/// This allows for a clean transition of any active streams when switching devices.
fn switch_device(node: &Node) -> Result<()> {
    let props = &node.info.props;
    // Set the default sink.
    Cmd::new(format!("wpctl set-default {}", props.object_id)).run_success()?;

    move_inputs_to_sink(props.object_serial)?;

    // Inform the user about the sink we just switched to.
    notify(1500, format!("Changed sink to {}", props.node_description))?;

    Ok(())
}

/// Get the list of all active sinks and print them to the commandline.
fn list_sinks() -> Result<()> {
    let nodes = get_sinks()?;

    if nodes.is_empty() {
        println!("Found no sinks");
        return Ok(());
    }

    println!("Found the following sinks:");
    for node in nodes.iter() {
        let props = &node.info.props;
        println!(
            "{}:\n \
            Serial: {}\n \
            Description: {}\n \
            Name: {}\n \
            ",
            props.object_id, props.object_serial, props.node_description, props.node_name,
        );
    }

    Ok(())
}
