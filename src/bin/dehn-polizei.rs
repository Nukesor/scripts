use std::{
    fs::{File, remove_file},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{ArgAction, Parser};
use dirs::runtime_dir;
use log::info;
use script_utils::{logging, notify::*};

#[derive(Parser, Debug)]
#[clap(
    name = "Dehn-Polizei",
    about = "A little background daemon which notifies the user that they should do some stretching",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    #[clap(subcommand)]
    cmd: SubCommand,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Start the daemon.
    Start {
        /// The interval (in minutes) at which the user will be notified that they should stretch
        #[clap(short, long, default_value = "90")]
        interval: usize,

        /// The interval at which the user will be reminded if they didn't stretch yet.
        #[clap(short, long, default_value = "10")]
        reminder_interval: usize,
    },

    /// Signal that you've stretched
    Ack {},
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    match args.cmd {
        SubCommand::Start {
            interval,
            reminder_interval,
        } => start(interval, reminder_interval),
        SubCommand::Ack {} => {
            // Touch an ack file to indicate that the user has stretched.
            File::create(ack_file_path()?)?;
            Ok(())
        }
    }
}

fn ack_file_path() -> Result<PathBuf> {
    Ok(runtime_dir()
        .ok_or(anyhow!("Couldn't find runtime dir"))?
        .join("dehn-polizei-ack"))
}

fn start(start: usize, reminder_interval: usize) -> Result<()> {
    info!(
        "\n
        User will be regularily notified every {start} minutes.
        They'll receive a follow-up notification every {reminder_interval} minutes\n",
    );

    let mut last_stretch: DateTime<Utc> = Utc::now();
    // Save the time in minutes since last_stretch when we sent a notification.
    let mut last_notify_minutes = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let now: DateTime<Utc> = Utc::now();
        let minutes = (now - last_stretch).num_minutes() as usize;

        // Search for the ack file, if it exists, the user has stretched.
        // Reset the timer and remove the file.
        if ack_file_path()?.exists() {
            remove_file(ack_file_path()?)?;
            last_stretch = Utc::now();
            last_notify_minutes = 0;
        }

        // We're still in good territory
        if minutes < start {
            continue;
        }

        // Handle the first notification
        if last_notify_minutes == 0 && minutes < (start + reminder_interval) {
            info!("Notify for stretching");
            notify(
                20 * 1000,
                format!(
                    "You have have been working for {start} minutes.\nTime for a stretch\\!\\!"
                ),
            )?;
            last_notify_minutes = minutes;
            continue;
        }

        // Now we handle follow-up notifications.

        // We're still in the same reminder interval
        if minutes < (last_notify_minutes + reminder_interval) {
            continue;
        }

        // We hit the next interval. Push a critical notification.
        info!("Sending reminder for {minutes} minutes.");
        critical_notify(
            20 * 1000,
            format!(
                "You are {} minutes overdue! Go stretch!",
                minutes - last_notify_minutes
            ),
        )?;
        last_notify_minutes = minutes;
    }
}
