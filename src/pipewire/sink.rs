use anyhow::{Context, Result, bail};
use log::{debug, error, info, trace, warn};
use strum::Display;

use super::{
    Device,
    schema::{node::Node, parse_pw_dump},
};
use crate::{exec::Cmd, notify::*, ring::Ring};

// Some sinks are just uninteresting for me.
const IGNORED_SINKS: &[&str] = &[
    // Sony/Philips Digital Interface
    // Coaxial/Optical input
    "USB Audio S/PDIF Output",
];

/// Get a map of all audio sink noes.
pub fn get_sinks() -> Result<Vec<Node>> {
    let (devices, nodes) = parse_pw_dump()?;

    let mut valid_nodes = Vec::new();

    // Run through all devices and find the one we desire.
    for node in nodes.into_iter() {
        let props = &node.info.props;
        // We are only interested in devices of Audio/Sink type.
        if &props.media_class != "Audio/Sink" {
            continue;
        }

        // Skip all ignored sinks
        if IGNORED_SINKS.contains(&props.node_description.as_str()) {
            continue;
        }

        // Ignore any sinks where we can safely say that they aren't plugged in.
        if is_not_plugged_in(&node, &devices) {
            continue;
        }

        info!(
            "Found device {}: {} ({})",
            props.object_serial, props.node_description, props.node_name
        );
        trace!("Raw: {:#?}", node);

        valid_nodes.push(node);
    }

    Ok(valid_nodes)
}

/// Check whether the physical connection for a node is actually plugged in or not.
/// To check this, we have to search the associated device for the node, go through its profiles
/// and get the profile for that node.
///
/// The way we're currently doing this is by comparing the node's profile description
/// and the actual profile description.
/// However, this doesn't always work, as these descriptions seem to sometimes differ.
///
/// We handle any error graceful and return `false`. I.e. if:
/// - No device is found
/// - No matching profile is found
/// - The status of the profile is `unknown`.
fn is_not_plugged_in(node: &Node, devices: &[Device]) -> bool {
    let device_id = &node.info.props.device_id;

    // Ensure that there's a device profile description and name.
    // Without this, we cannot check whether the node is actually plugged in.
    //
    // For now, this was only the case for devices like Bluetooth Headsets,
    // which are only present if connected.
    let Some(profile_description) = &node.info.props.device_profile_description else {
        return false;
    };
    let Some(profile_name) = &node.info.props.device_profile_name else {
        return false;
    };

    // Get the device
    let Some(device) = devices.iter().find(|device| device.id == *device_id) else {
        return false;
    };

    // Go through all profiles
    for profile in &device.info.params.profiles {
        // There's a bit of inconsistency over here.
        // From what I've seen, there're several possible ways of finding the matching route.
        //
        // - The description matches perfectly
        // - The profile name matches the node's profile name prefixed with `output`
        if !(&profile.description == profile_description
            || profile.name == format!("output:{profile_name}"))
        {
            continue;
        }

        // If we found a matching route, check if it's not plugged in
        return profile.available == "no";
    }

    // Check all routes.
    // Some profile-names seem to reference routes, which is pretty confusing
    for route in &device.info.params.routes {
        // There's a bit of inconsistency over here.
        // From what I've seen, there're several possible ways of finding the matching route.
        //
        // - The description matches perfectly
        // - The profile name matches the node's profile name prefixed with `output`
        if !(&route.description == profile_description
            || route.name == format!("output:{profile_name}"))
        {
            continue;
        }

        // If we found a matching route, check if it's not plugged in
        return route.available == "no";
    }

    false
}

#[derive(Display)]
pub enum Direction {
    Next,
    Previous,
}

/// Try to determine the id and description of the targeted sink.
/// May return None if the target sink cannot be found.
pub fn rotate_sink(direction: Direction) -> Result<Option<Node>> {
    // Determine the current sink.
    let output = Cmd::new("pactl get-default-sink")
        .run_success()
        .context("Failed to find default sink")?;
    let current_sink_name = output.stdout_str().trim().to_owned();
    debug!("Current sink name: {current_sink_name}");

    // Initialize the device ring for easy iteration to the next/previous device.
    let nodes = get_sinks()?;
    let mut ring = Ring::new(nodes.clone())?;

    // Move the cursor to the current node.
    // If `None` is found, return an error as we cannot determine the current sink.
    let current_device = ring.find(|(_, node)| node.info.props.node_name == current_sink_name);
    if current_device.is_none() {
        error!("Could not determine current sink: {current_sink_name}");
        critical_notify(
            1500,
            format!("Could not determine current sink: {current_sink_name}"),
        )?;
        bail!("Failed to determine current sink");
    }

    // Check if we find a node for the given name.
    let node = match direction {
        Direction::Next => Some((ring.next()).clone()),
        Direction::Previous => Some((ring.prev()).clone()),
    };

    if let Some(ref node) = node {
        debug!(
            "{direction} sink name: {}",
            node.info.props.node_description
        );
    }

    Ok(node.clone())
}

/// Search all inputs and switch them over to the given device.
pub fn move_inputs_to_sink(node_object_serial: usize) -> Result<()> {
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
        let result =
            Cmd::new(format!("pactl move-sink-input {id} {node_object_serial}")).run_success();
        if let Err(err) = result {
            warn!("Failed to switch input {id} to new sink: {err:?}");
        };
    }

    Ok(())
}
