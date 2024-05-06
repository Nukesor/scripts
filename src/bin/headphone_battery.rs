//! Small helper script to get the battery status of my various wireless headphones.

use anyhow::Result;
use clap::{ArgAction, Parser};

use log::warn;
use script_utils::{
    exec::Cmd,
    i3status::{CustomI3Status, I3State},
    logging,
};

#[derive(Parser, Debug)]
#[clap(
    name = "headphone_battery",
    about = "Get the battery status of various headphones.",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    // Check headsetcontrol first
    let mut battery_status = headsetcontrol();

    // Check bluetoothctl next.
    if battery_status.is_none() {
        battery_status = bluetoothctl();
    }

    // If we got some headphone info, format and print it.
    if let Some(battery_status) = battery_status {
        let i3state = state_from_battery_status(&battery_status);

        let inner_text = match battery_status {
            BatteryStatus::Charging => "Charging".to_string(),
            BatteryStatus::Percentage(percent) => format!("{percent}%"),
        };
        let text = format!("( {inner_text})");

        let json = serde_json::to_string(&CustomI3Status::new(i3state, text))?;
        println!("{json}");
        return Ok(());
    }

    // We didn't get any info, return an empty response.
    println!("{}", serde_json::to_string(&CustomI3Status::default())?);

    Ok(())
}

enum BatteryStatus {
    Charging,
    Percentage(usize),
}

/// Determine the i3status state for this section.
/// The color will change if the battery reaches certain states.
fn state_from_battery_status(battery_status: &BatteryStatus) -> I3State {
    match battery_status {
        BatteryStatus::Charging => I3State::Idle,
        BatteryStatus::Percentage(percent) => match percent {
            0..=15 => I3State::Critical,
            16..=25 => I3State::Warning,
            26..=35 => I3State::Good,
            _ => I3State::Idle,
        },
    }
}

// First check `headsetcontrol`.
// <https://github.com/Sapd/HeadsetControl>
fn headsetcontrol() -> Option<BatteryStatus> {
    let result = Cmd::new("headsetcontrol --battery").run_success();
    let output = match result {
        Ok(capture) => capture.stdout_str(),
        Err(err) => {
            warn!("Got error on headsetcontrol call:\n{err:#?}");
            return None;
        }
    };

    // Output of the command looks like this:
    // ```
    // Found SteelSeries Arctis Nova 7!
    // Battery: 25%
    // Success!
    // ```
    for line in output.lines() {
        // Battery output of the command looks like this
        if line.starts_with("Battery:") {
            let parts: Vec<String> = line.split(" ").map(|s| s.to_string()).collect();
            let battery = &parts[1];

            // The dongle will be detected even if the headset isn't visible.
            // In that case we get an "Unavailable" battery status.
            if battery == "Unavailable" {
                return None;
            }

            if battery == "Charging" {
                return Some(BatteryStatus::Charging);
            }

            // Remove the percentage sign
            let Ok(battery) = battery.trim_end_matches("%").parse() else {
                warn!("Failed to parse battery value to usize: {battery}");
                return None;
            };

            return Some(BatteryStatus::Percentage(battery));
        }
    }

    return None;
}

// First check `bluetoothctl`.
fn bluetoothctl() -> Option<BatteryStatus> {
    let result = Cmd::new("bluetoothctl info").run_success();
    let output = match result {
        Ok(capture) => capture.stdout_str(),
        Err(err) => {
            warn!("Got error on headsetcontrol call:\n{err:#?}");
            return None;
        }
    };

    // Output of the command looks like this:
    // ```
    // ...
    // RSSI: 0xffffffc2 (-62)
    // TxPower: 0xfffffff9 (-7)
    // Battery Percentage: 0x64 (100)
    // ```
    for line in output.lines() {
        // Battery output of the command looks like this
        if line.trim().starts_with("Battery Percentage:") {
            // Split at `(` to get the last part: `100)`
            let parts: Vec<String> = line.split("(").map(|s| s.to_string()).collect();
            let battery = &parts[1];

            // Remove the closing bracket
            let Ok(battery) = battery.trim_end_matches(")").parse() else {
                warn!("Failed to parse battery value to usize: {battery}");
                return None;
            };

            return Some(BatteryStatus::Percentage(battery));
        }
    }

    return None;
}
