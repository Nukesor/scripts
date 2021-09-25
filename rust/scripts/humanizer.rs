use chrono::Duration;
use clap::Clap;

#[derive(Clap, Debug)]
#[clap(
    name = "Humanizer",
    about = "Bring your machine output into human readable form",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    /// Format something time related
    Time(Time),
}

#[derive(Clap, Debug)]
pub struct Time {
    /// Convert nano seconds to human readable time
    #[clap(short, long)]
    pub nanos: Option<i64>,

    /// Convert nano seconds to human readable time
    #[clap(short, long)]
    pub seconds: Option<i64>,
}

fn main() {
    // Parse commandline options.
    let args = CliArguments::parse();

    match args.cmd {
        SubCommand::Time(time) => format_time(time),
    }
}

pub fn format_time(time: Time) {
    let mut duration = if let Some(nanos) = time.nanos {
        Duration::nanoseconds(nanos)
    } else if let Some(seconds) = time.seconds {
        Duration::seconds(seconds)
    } else {
        println!("Either specify nanos or seconds");
        std::process::exit(1);
    };

    let days = duration.num_days();
    duration = duration - Duration::days(days);

    let hours = duration.num_hours();
    duration = duration - Duration::hours(hours);

    let minutes = duration.num_minutes();
    duration = duration - Duration::minutes(minutes);

    let seconds = duration.num_seconds();

    let mut formatted = String::new();
    if days > 0 {
        formatted.push_str(&format!("{} days ", days));
    }
    formatted.push_str(&format!("{:02}:{:02}:{:02}", hours, minutes, seconds));

    print!("{}", formatted);
}
