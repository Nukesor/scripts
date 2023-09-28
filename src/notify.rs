use anyhow::{Context, Result};

use crate::exec::Cmd;

/// Send an urgent notification to the notification daemon.
pub fn critical_notify(display_time: usize, message: String) -> Result<()> {
    // Inform the user about the sink we just switched to.
    Cmd::new(format!(
        "notify-send --urgency=critical --expire-time={display_time} '{message}'",
    ))
    .run_success()
    .context("Failed to send notification.")?;

    Ok(())
}

/// Send a notification to the notification daemon.
pub fn notify(display_time: usize, message: String) -> Result<()> {
    // Inform the user about the sink we just switched to.
    Cmd::new(format!(
        "notify-send --expire-time={display_time} '{message}'",
    ))
    .run_success()
    .context("Failed to send notification.")?;

    Ok(())
}
