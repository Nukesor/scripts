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
    let mut device_status = headsetcontrol();

    // Check bluetoothctl next.
    if device_status == DeviceStatus::Unavailable {
        device_status = bluetoothctl();
    }

    // If we got some headphone info, format and print it.
    let i3state = state_from_battery_status(&device_status);

    let inner_text = match device_status {
        DeviceStatus::Charging { percentage } => {
            if let Some(percentage) = percentage {
                format!("{percentage}% ")
            } else {
                "".to_string()
            }
        }
        DeviceStatus::Available { percentage } => format!("{percentage}%"),
        DeviceStatus::Unavailable => {
            // We didn't get any info, return an empty response.
            println!("{}", serde_json::to_string(&CustomI3Status::default())?);
            return Ok(());
        }
    };

    let text = format!("( {inner_text})");
    let json = serde_json::to_string(&CustomI3Status::new(i3state, text))?;
    println!("{json}");

    Ok(())
}

#[derive(PartialEq)]
enum DeviceStatus {
    Charging { percentage: Option<usize> },
    Available { percentage: usize },
    Unavailable,
}

/// Determine the i3status state for this section.
/// The color will change if the battery reaches certain states.
fn state_from_battery_status(battery_status: &DeviceStatus) -> I3State {
    match battery_status {
        DeviceStatus::Charging { .. } => I3State::Idle,
        DeviceStatus::Available { percentage } => match percentage {
            0..=15 => I3State::Critical,
            16..=25 => I3State::Warning,
            26..=35 => I3State::Good,
            _ => I3State::Idle,
        },
        _ => I3State::Idle,
    }
}

// First check `headsetcontrol`.
// <https://github.com/Sapd/HeadsetControl>
fn headsetcontrol() -> DeviceStatus {
    let result = Cmd::new("headsetcontrol --battery").run_success();
    let output = match result {
        Ok(capture) => capture.stdout_str(),
        Err(err) => {
            warn!("Got error on headsetcontrol call:\n{err:#?}");
            return DeviceStatus::Unavailable;
        }
    };

    enum Availablility {
        Charging,
        Available,
        Unavailable,
    }

    // Output looks like this:
    // ```
    // Found SteelSeries Arctis Nova 7 (Arctis Nova 7)!
    //
    // Battery:
    // 	      Status: BATTERY_AVAILABLE
    // 	      Level: 100%
    // ```
    let mut availability = Availablility::Unavailable;
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Status:") {
            let parts: Vec<String> = line.split(' ').map(|s| s.to_string()).collect();
            let status_str = &parts[1];

            availability = match status_str.as_str() {
                "BATTERY_CHARGING" => Availablility::Charging,
                "BATTERY_AVAILABLE" => Availablility::Available,
                _ => Availablility::Unavailable,
            };
        }

        // Battery output of the command looks like this
        if line.starts_with("Level:") {
            let parts: Vec<String> = line.split(' ').map(|s| s.to_string()).collect();
            let battery = &parts[1];

            // Remove the percentage sign
            let Ok(percentage) = battery.trim_end_matches('%').parse() else {
                warn!("Failed to parse battery value to usize: {battery}");
                return DeviceStatus::Unavailable;
            };

            let status = match availability {
                Availablility::Charging => DeviceStatus::Charging {
                    percentage: Some(percentage),
                },
                Availablility::Available => DeviceStatus::Available { percentage },
                Availablility::Unavailable => DeviceStatus::Unavailable,
            };

            return status;
        }
    }

    DeviceStatus::Unavailable
}

// First check `bluetoothctl`.
fn bluetoothctl() -> DeviceStatus {
    let result = Cmd::new("bluetoothctl info").run_success();
    let output = match result {
        Ok(capture) => capture.stdout_str(),
        Err(err) => {
            warn!("Got error on headsetcontrol call:\n{err:#?}");
            return DeviceStatus::Unavailable;
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
            let parts: Vec<String> = line.split('(').map(|s| s.to_string()).collect();
            let battery = &parts[1];

            // Remove the closing bracket
            let Ok(percentage) = battery.trim_end_matches(')').parse() else {
                warn!("Failed to parse battery value to usize: {battery}");
                return DeviceStatus::Unavailable;
            };

            return DeviceStatus::Available { percentage };
        }
    }

    DeviceStatus::Unavailable
}
