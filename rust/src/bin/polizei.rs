use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Parser;

use log::{debug, info};
use script_utils::{logging, prelude::*, process::get_process_cmdlines};

#[derive(Parser, Debug)]
#[clap(
    name = "Polizei",
    about = "A little background daemon which notifies the user if they play games for too long.",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// The interval (in minutes) at which the user will be notified that they've
    /// been playing for a certain amount of time.
    #[clap(short, long, default_value = "60")]
    pub notification_interval: i64,

    /// The threshold at which the user will be notified to stop playing.
    #[clap(short, long, default_value = "120")]
    pub threshold: i64,

    /// The interval at which the user will be notified to stop playing.
    #[clap(short, long, default_value = "10")]
    pub stop_notification_interval: i64,
}

// A mapping of the game names to the substrings of the binary calls we should look for.
const GAME_LIST: &[(&str, &str)] = &[
    ("Factorio", "factorio"),
    ("Noita", "noita"),
    ("Apex Legends", "apex"),
    ("Satisfactory", "satisfactory"),
    ("Starsector", "starsector"),
];

struct RunningGame {
    start: DateTime<Local>,
    notification_count: i64,
    stop_notification_count: i64,
}

impl Default for RunningGame {
    fn default() -> Self {
        Self {
            start: Local::now(),
            notification_count: 0,
            stop_notification_count: 0,
        }
    }
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    let mut running_games: HashMap<&'static str, RunningGame> = HashMap::new();
    let current_user_id = users::get_current_uid();
    logging::init_logger(args.verbose);
    info!(
        "\n
        User will be regularily notified every {} minutes.
        After {} minutes they'll be prompted to stop.
        From then on they'll receive a notification every {} minutes\n",
        args.notification_interval, args.threshold, args.stop_notification_interval,
    );

    // Check every few minutes whether any games are up and running.
    // If they're running for the specified times, notify the user of this.
    // Get more annoying if they're running past the threshold.
    loop {
        let processes = get_process_cmdlines(current_user_id)?;

        let mut found_games: HashSet<&'static str> = HashSet::new();
        // Check all processes for the specified binaries.
        for cmdline in processes {
            debug!("Checking {cmdline}");
            for (name, binary) in GAME_LIST {
                // The cmdline doesn't contain the game just exit early.
                if !cmdline.to_lowercase().contains(binary) {
                    continue;
                }

                info!("Found running game {name}");
                found_games.insert(name);
                handle_running_game(&args, &mut running_games, *name)?;
                break;
            }
        }

        // Remove games that're no longer active.
        for key in running_games.keys().copied().collect::<Vec<&'static str>>() {
            if !found_games.contains(key) {
                info!("{key} has been closed.");
                // Remove the game from the list of running games.
                running_games.remove(key);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn handle_running_game(
    args: &CliArguments,
    running_games: &mut HashMap<&'static str, RunningGame>,
    name: &'static str,
) -> Result<()> {
    let mut running_game = running_games.entry(name).or_default();
    let now = Local::now();

    // Calculate the amount of minutes since we first saw that game running.
    let elapsed_minutes = (now - running_game.start).num_minutes();

    //
    // The user is still allowed to play. But we might notify them anyway.
    //
    if elapsed_minutes < args.threshold {
        // Calculate the current interval we're in.
        let current_interval = elapsed_minutes / args.notification_interval;
        let time_string = format_duration(elapsed_minutes);

        // Send the user a notification if we're in a new interval.
        if running_game.notification_count < current_interval {
            info!("Sending normal notification for {name} at {time_string}");
            notify(
                format!("You have been playing {name} for {time_string}"),
                120 * 1000,
                false,
            )?;
        }
        running_game.notification_count = current_interval;

        return Ok(());
    }

    //
    // The user should really stop to play now.
    //

    // Calculate the current stop interval we're in.
    let current_interval =
        ((elapsed_minutes - args.threshold) / args.stop_notification_interval) + 1;

    // Send the user a notification if we're in a new interval.
    if running_game.stop_notification_count < current_interval {
        let time_string = format_duration(elapsed_minutes);

        info!("Sending stop notification for {name} at {time_string}");
        notify(
            format!("Stop playing {name}. You are at it since {time_string}"),
            300 * 1000,
            true,
        )?;
        running_game.stop_notification_count = current_interval;
    }

    Ok(())
}

fn notify(text: String, timeout: usize, critical: bool) -> Result<()> {
    let critical = if critical { "--urgency critical" } else { "" };
    Cmd::new(format!(
        "notify-send --expire-time={timeout} '{text}' {critical}",
    ))
    .run_success()
    .context("Failed to send notification.")?;

    Ok(())
}

fn format_duration(elapsed_minutes: i64) -> String {
    let minutes = elapsed_minutes % 60;
    let hours = elapsed_minutes / 60;

    if hours == 0 {
        format!("{minutes} Minutes")
    } else {
        format!("{hours} Hours and {minutes} Minutes")
    }
}
