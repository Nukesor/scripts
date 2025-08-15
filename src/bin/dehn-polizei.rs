use std::{
    fs::{File, remove_file},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use clap::{ArgAction, Parser};
use dirs::runtime_dir;
use log::info;
use script_utils::{
    logging,
    notify::*,
    timer::{Phase, PhaseTimer},
};

#[derive(Debug, Clone, PartialEq)]
pub enum StretchAction {
    Initial { stretch_interval: usize },
    Reminder { reminder_interval: usize },
}

#[derive(Parser, Debug)]
#[clap(
    name = "Dehn-Polizei",
    about = "A little background daemon which notifies me that I should do some stretching",
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

fn start(stretch_interval: usize, reminder_interval: usize) -> Result<()> {
    info!(
        "\n
        User will be regularly notified every {stretch_interval} minutes.
        They'll receive a follow-up notification every {reminder_interval} minutes\n",
    );

    let phases = vec![
        Phase::one_time(
            stretch_interval,
            StretchAction::Initial { stretch_interval },
        ),
        Phase::recurring(
            stretch_interval,
            reminder_interval,
            StretchAction::Reminder { reminder_interval },
        ),
    ];
    let mut timer = PhaseTimer::new(phases);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));

        // Search for the ack file, if it exists, the user has stretched.
        // Reset the timer and remove the file.
        if ack_file_path()?.exists() {
            remove_file(ack_file_path()?)?;
            timer.reset();
            info!("Timer reset - user acknowledged stretch");
            continue;
        }

        if let Some(action) = timer.check() {
            match action {
                StretchAction::Initial { stretch_interval } => {
                    info!("Sending initial stretch notification");
                    let message = format!(
                        "You have been working for {stretch_interval} minutes.\nTime for a stretch\\!\\!",
                    );
                    notify(20 * 1000, message)?;
                }
                StretchAction::Reminder {
                    reminder_interval: _,
                } => {
                    info!("Sending stretch reminder");
                    let overdue_minutes = timer.elapsed_minutes() - stretch_interval;
                    let message = format!("You are {overdue_minutes} minutes overdue! Go stretch!");
                    critical_notify(40 * 1000, message)?;
                }
            }
        }
    }
}
