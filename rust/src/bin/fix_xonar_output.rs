use std::time::Duration;

use anyhow::{Context, Result};

use script_utils::pw_dump::*;
use script_utils::{exec::Cmd, unwrap_or_continue};

/// Run at startup and set the correct expected output for the Xonar audio card.
fn main() -> Result<()> {
    let tries = 10;
    let mut current_try = 0;
    while tries > current_try {
        let success = match set_xonar_output() {
            Err(err) => {
                println!("Failed to set output with error:\n{err:?}");
                continue;
            }
            Ok(success) => success,
        };
        if success {
            return Ok(());
        }

        current_try += 1;
        println!("Didn't find Xonar STX II card yet");
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("Couldn't find specified target sink after {tries} seconds.");

    Ok(())
}

fn set_xonar_output() -> Result<bool> {
    // Get current pipewire state.
    let capture = Cmd::new("pw-dump")
        .run_success()
        .context("pw-dump execution failed.")?;

    // Run through all devices and find the one we desire.
    let devices: Vec<Device> = serde_json::from_str(&capture.stdout_str())
        .context("Failed to deserialize pw-dump output.")?;
    for device in devices {
        let info = unwrap_or_continue!(device.info);
        let props = unwrap_or_continue!(info.props);

        // Ignore any cards that aren't the Xonar card.
        let name = unwrap_or_continue!(props.api_alsa_card_name);
        if name != "Xonar STX II" {
            continue;
        }

        let card_id = unwrap_or_continue!(props.api_alsa_card);
        // Set the correct output.
        Cmd::new(format!("amixer -c {card_id} cset numid=22 'Headphones'"))
            .run_success()
            .context("Failed to set correct output via amixer")?;
        println!("Success");

        return Ok(true);
    }

    Ok(false)
}
