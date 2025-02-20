//! A small helper binary to convert some raw values into human readable form.
//! For now, this includes:
//!
//! - Seconds -> Datetime
//! - Nanoseconds -> Datetime
use anyhow::{Context, Result, bail};
use chrono::TimeDelta;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    name = "Humanizer",
    about = "Bring your machine output into human readable form",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Format something time related
    Time(Time),
}

#[derive(Parser, Debug)]
pub struct Time {
    /// Convert nano seconds to human readable time
    #[clap(short, long)]
    pub nanos: Option<i64>,

    /// Convert nano seconds to human readable time
    #[clap(short, long)]
    pub seconds: Option<i64>,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    match args.cmd {
        SubCommand::Time(time) => format_time(time),
    }
}

pub fn format_time(time: Time) -> Result<()> {
    let mut duration = if let Some(nanos) = time.nanos {
        TimeDelta::nanoseconds(nanos)
    } else if let Some(seconds) = time.seconds {
        TimeDelta::try_seconds(seconds).context("Failed to convert seconds.")?
    } else {
        bail!("Either specify nanos or seconds");
    };

    let days = duration.num_days();
    duration -= TimeDelta::try_days(days).context("Failed to convert days")?;

    let hours = duration.num_hours();
    duration -= TimeDelta::try_hours(hours).context("Failed to convert hours")?;

    let minutes = duration.num_minutes();
    duration -= TimeDelta::try_minutes(minutes).context("Failed to convert minutes")?;

    let seconds = duration.num_seconds();

    let mut formatted = String::new();
    if days > 0 {
        formatted.push_str(&format!("{days} days "));
    }
    formatted.push_str(&format!("{hours:02}:{minutes:02}:{seconds:02}"));

    print!("{formatted}");

    Ok(())
}
