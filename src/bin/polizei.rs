use std::collections::{HashMap, HashSet};

use anyhow::Result;
use clap::{ArgAction, Parser};
use log::{debug, info};
use script_utils::{
    logging,
    notify::*,
    process::get_process_cmdlines,
    timer::{Phase, PhaseTimer},
};

#[derive(Parser, Debug)]
#[clap(
    name = "Polizei",
    about = "A little background daemon which notifies the user if they play games for too long.",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
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

// A mapping of the games to watch
// 1. Names of the game.
// 2. Substrings of the binary we should look for.
// 3. Whether we should warn the user if the threshold was exceeded.
const GAME_LIST: &[(&str, &str, bool)] = &[
    ("Oxygen Not Included", "OxygenNotIncluded", true),
    ("Factorio", "factorio", true),
    ("Noita", "noita", true),
    ("Apex Legends", "apex", false),
    ("Satisfactory", "satisfactory", true),
    ("Starsector", "starsector", true),
    ("Terraria", "terraria", false),
    ("Necesse", "necesse", true),
    ("some game", "streaming_client", true),
    ("Minecraft", "atlauncher.jar", true),
    ("Zero Sievert", "zero sievert.exe", true),
];

#[derive(Debug, Clone, PartialEq)]
pub enum GameAction {
    RegularNotification,
    StopNotification,
}

struct RunningGame {
    timer: PhaseTimer<GameAction>,
}

impl RunningGame {
    fn new(
        notification_interval: i64,
        threshold: i64,
        stop_notification_interval: i64,
        strict: bool,
    ) -> Self {
        let mut phases = vec![];

        // Add regular notification phase (recurring from start if interval > 0)
        if notification_interval > 0 {
            phases.push(Phase::recurring(
                notification_interval as usize,
                notification_interval as usize,
                GameAction::RegularNotification,
            ));
        }

        // Add stop notification phase (recurring from threshold if strict)
        if strict && stop_notification_interval > 0 {
            phases.push(Phase::recurring(
                threshold as usize,
                stop_notification_interval as usize,
                GameAction::StopNotification,
            ));
        }

        Self {
            timer: PhaseTimer::new(phases),
        }
    }

    fn elapsed_minutes(&self) -> usize {
        self.timer.elapsed_minutes()
    }

    fn calculate_action(&mut self) -> Option<GameAction> {
        self.timer.calculate_action()
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
            debug!("Looking at process: {cmdline}");
            for (name, binary, strict) in GAME_LIST {
                // The cmdline doesn't contain the game just exit early.
                if !cmdline.to_lowercase().contains(binary) {
                    continue;
                }

                info!("Found running game {name}");
                found_games.insert(name);
                handle_running_game(&args, &mut running_games, name, *strict)?;
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
    strict: bool,
) -> Result<()> {
    let running_game = running_games.entry(name).or_insert_with(|| {
        RunningGame::new(
            args.notification_interval,
            args.threshold,
            args.stop_notification_interval,
            strict,
        )
    });

    if let Some(action) = running_game.calculate_action() {
        let elapsed_minutes = running_game.elapsed_minutes() as i64;
        let time_string = format_duration(elapsed_minutes);

        match action {
            GameAction::RegularNotification => {
                info!("Sending normal notification for {name} at {time_string}");
                notify(
                    10 * 1000,
                    format!("You have been playing {name} for {time_string}"),
                )?;
            }
            GameAction::StopNotification => {
                info!("Sending stop notification for {name} at {time_string}");
                critical_notify(
                    300 * 1000,
                    format!("Stop playing {name}. You are at it since {time_string}"),
                )?;
            }
        }
    }

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
