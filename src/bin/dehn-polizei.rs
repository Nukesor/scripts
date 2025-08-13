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

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationAction {
    None,
    Initial(String),
    Reminder(String),
}

#[derive(Debug, Clone)]
pub struct Timer {
    /// The interval at which the user will be reminded if they didn't stretch yet.
    reminder_interval: usize,
    /// The interval (in minutes) at which the user will be notified that they should stretch
    stretch_interval: usize,
    /// When the timer was started/last reset
    start_time: DateTime<Utc>,
    /// Minutes since start_time when we last sent a notification
    pub last_notification_minutes: usize,
}

impl Timer {
    pub fn new(interval: usize, reminder_interval: usize) -> Self {
        Self {
            stretch_interval: interval,
            reminder_interval,
            start_time: Utc::now(),
            last_notification_minutes: 0,
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Utc::now();
        self.last_notification_minutes = 0;
    }

    pub fn calculate_notification(&mut self) -> NotificationAction {
        let current_minutes = (Utc::now() - self.start_time).num_minutes() as usize;
        self.calculate_notification_at_time(current_minutes)
    }

    fn calculate_notification_at_time(&mut self, current_minutes: usize) -> NotificationAction {
        // Still within the normal stretch interval
        if current_minutes < self.stretch_interval {
            return NotificationAction::None;
        }

        let minutes_since_last_notification = current_minutes - self.last_notification_minutes;

        // First notification after stretch interval is reached
        if self.last_notification_minutes == 0 {
            self.last_notification_minutes = current_minutes;
            return NotificationAction::Initial(format!(
                "You have been working for {} minutes.\nTime for a stretch\\!\\!",
                self.stretch_interval
            ));
        }

        // Check if it's time for a reminder
        if minutes_since_last_notification >= self.reminder_interval {
            let overdue_minutes = current_minutes - self.stretch_interval;
            self.last_notification_minutes = current_minutes;
            return NotificationAction::Reminder(format!(
                "You are {} minutes overdue! Go stretch!",
                overdue_minutes
            ));
        }

        NotificationAction::None
    }
}

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

fn start(stretch_interval: usize, reminder_interval: usize) -> Result<()> {
    info!(
        "\n
        User will be regularly notified every {} minutes.
        They'll receive a follow-up notification every {} minutes\n",
        stretch_interval, reminder_interval
    );

    let mut timer = Timer::new(stretch_interval, reminder_interval);

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

        let action = timer.calculate_notification();

        match action {
            NotificationAction::None => continue,
            NotificationAction::Initial(message) => {
                info!("Sending initial stretch notification");
                notify(20 * 1000, message)?;
            }
            NotificationAction::Reminder(message) => {
                info!("Sending stretch reminder");
                critical_notify(20 * 1000, message)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn creates_correct_intervals() {
        let timer = Timer::new(90, 10);

        assert_eq!(timer.stretch_interval, 90);
        assert_eq!(timer.reminder_interval, 10);
        assert_eq!(timer.last_notification_minutes, 0);
    }

    #[test]
    fn sets_start_time_to_now() {
        let before = Utc::now();
        let timer = Timer::new(90, 10);
        let after = Utc::now();

        assert!(timer.start_time >= before);
        assert!(timer.start_time <= after);
    }

    #[test]
    fn resets_all_values() {
        let mut timer = Timer::new(90, 10);
        timer.last_notification_minutes = 25;

        let before = Utc::now();
        timer.reset();
        let after = Utc::now();

        assert_eq!(timer.last_notification_minutes, 0);
        assert!(timer.start_time >= before);
        assert!(timer.start_time <= after);
    }

    #[test]
    fn no_notification_within_interval() {
        let mut timer = Timer::new(90, 10);
        let action = timer.calculate_notification_at_time(45);
        assert_eq!(action, NotificationAction::None);
    }

    #[test]
    fn no_notification_at_exact_interval() {
        let mut timer = Timer::new(90, 10);
        let action = timer.calculate_notification_at_time(89);
        assert_eq!(action, NotificationAction::None);
    }

    #[rstest]
    #[case::at_interval(90)]
    #[case::after_interval(95)]
    fn sends_initial_notification(#[case] minutes: usize) {
        let mut timer = Timer::new(90, 10);
        let action = timer.calculate_notification_at_time(minutes);

        match action {
            NotificationAction::Initial(message) => {
                assert!(message.contains("90 minutes"));
                assert!(message.contains("stretch"));
            }
            _ => panic!("Expected Initial notification"),
        }

        assert_eq!(timer.last_notification_minutes, minutes);
    }

    #[test]
    fn sends_reminder_after_reminder_interval() {
        let mut timer = Timer::new(90, 10);

        // Simulate initial notification at 90 minutes
        timer.calculate_notification_at_time(90);

        // Check reminder at 100 minutes (10 minutes after initial)
        let action = timer.calculate_notification_at_time(100);

        match action {
            NotificationAction::Reminder(message) => {
                assert!(message.contains("10 minutes overdue"));
            }
            _ => panic!("Expected Reminder notification"),
        }

        assert_eq!(timer.last_notification_minutes, 100);
    }

    #[test]
    fn no_reminder_before_interval() {
        let mut timer = Timer::new(90, 10);

        // Simulate initial notification at 90 minutes
        timer.calculate_notification_at_time(90);

        // Check at 95 minutes (5 minutes after initial, less than reminder interval)
        let action = timer.calculate_notification_at_time(95);
        assert_eq!(action, NotificationAction::None);
    }

    #[test]
    fn sends_multiple_reminders() {
        let mut timer = Timer::new(90, 10);

        // Initial notification
        timer.calculate_notification_at_time(90);

        // First reminder at 100 minutes
        let action1 = timer.calculate_notification_at_time(100);
        match action1 {
            NotificationAction::Reminder(message) => {
                assert!(message.contains("10 minutes overdue"));
            }
            _ => panic!("Expected first Reminder notification"),
        }

        // Second reminder at 110 minutes
        let action2 = timer.calculate_notification_at_time(110);
        match action2 {
            NotificationAction::Reminder(message) => {
                assert!(message.contains("20 minutes overdue"));
            }
            _ => panic!("Expected second Reminder notification"),
        }
    }
}
