//! A script used to implement staggered backups.
//!
//! It expects a folder that's full of files, each containing the timestamp of its creation in the
//! filename. It then deletes all files except:
//! - 1 file for each of the last 7 days
//! - 1 file for each of the last 26 weeks
//! - 1 file for each month of the last 2 years
//!
//! The file that's kept is always the oldest file that can be found for the given timespan.
//!
//! Example:
//! The current date is 2025-04-02
//! There're two files:
//! - mydb_2025-04-01_10-00.dump
//! - mydb_2025-04-01_23-00.dump
//!
//! In this case, the second file will be deleted, as it's newer than the first one.
use std::{
    collections::BTreeMap,
    fs::{DirEntry, remove_file},
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, TimeDelta, Utc};
use clap::{ArgAction, Parser};
use log::error;
use regex::Regex;
use script_utils::{
    FileType,
    fs::find_leaf_dirs,
    logging,
    read_dir_or_fail,
    table::{pretty_table, print_headline_table},
};

#[derive(Parser, Debug)]
#[clap(
    name = "Staggered backups",
    about = "Execute in a directory to ",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// The path that contains the backup files.
    pub path: PathBuf,

    /// Regex that extracts the matching `date_format` string from a filename.
    /// The default extracts
    /// "2025-04-02_00-00" from something like
    /// "some_game_name_2025-04-02_00-00.tar.zst"
    #[clap(
        short,
        long,
        default_value = r"[a-z_]*_([0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}-[0-9]{2})\..*"
    )]
    pub date_extraction_regex: String,

    /// The date format string that's used in the filename
    /// E.g. "%Y-%m-%d_%H-%M" for "2025-04-02_00-00.dump"
    #[clap(short, long, default_value = "%Y-%m-%d_%H-%M")]
    pub date_format: String,

    /// Don't do any operations unless this flag is set
    #[clap(short, long)]
    pub execute: bool,

    /// If this is set, recursively search for folders with backups from the given path.
    /// This will run the staggered backups for each directory that is found.
    #[clap(short, long)]
    pub recursive: bool,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    // Min log level INFO
    logging::init_logger(args.verbose + 2);

    if !args.execute {
        println!("--- DRY RUN MODE ---");
    }
    if args.recursive {
        println!("WARNING: Running in recursive mode.");
    }
    println!();

    if !args.recursive {
        run_staggered_backup(&args.path, &args)?;
    } else {
        let leaf_dirs = find_leaf_dirs(args.path.clone())?;
        let mut leaf_dirs_iter = leaf_dirs.iter().peekable();
        while let Some(dir) = leaf_dirs_iter.next() {
            run_staggered_backup(dir, &args)?;
            if leaf_dirs_iter.peek().is_some() {
                println!("\n");
            }
        }
    }

    Ok(())
}

pub fn run_staggered_backup(path: &PathBuf, args: &CliArguments) -> Result<()> {
    let files = read_dir_or_fail(path, Some(FileType::File)).context("Failed to read files")?;
    let mut files_by_date = BTreeMap::new();
    println!("═══════════════════════════════════════════════════════════════");
    print_headline_table(format!("Checking folder: {path:?}"));
    // Go through all files and extract the datetime from its filename.
    for file in files {
        let name = file
            .path()
            .file_name()
            .context(format!("Got file without filename: {:?}", file.path()))?
            .to_string_lossy()
            .to_string();

        // Run the date extraction regex
        let re = Regex::new(&args.date_extraction_regex).context(format!(
            "Found invalid date_extraction_regex: {}",
            args.date_extraction_regex
        ))?;
        let Some(captures) = re.captures(&name) else {
            error!("Date extraction regex didn't match name. Ignoring file: {name}");
            continue;
        };

        let datetime = NaiveDateTime::parse_from_str(&captures[1], &args.date_format);
        let datetime = match datetime {
            Ok(datetime) => datetime,
            Err(_) => {
                error!("Failed to parse date string. Ignoring file: {name}");
                continue;
            }
        };

        files_by_date.insert(datetime, file);
    }
    if files_by_date.is_empty() {
        println!("No files for backup found.");
        return Ok(());
    }

    let mut brackets = init_brackets()?;

    // Now we sort all entries into their brackets.
    //
    // The brackets are ordered in a way that the smaller brackets come first.
    // So even if there's some overlap, entries will be sorted into the smaller brackets
    // (i.e days instead of weeks).
    //
    // The backup files themselves are ordered from oldest to newest.
    // We now check for each bracket whether the newest backup matches the given bracket.
    // This is done until an entry is hit that is older than the current bracket.
    // In that case, we continue with the next bracket.
    for bracket in brackets.iter_mut() {
        'inner: loop {
            {
                let entry = files_by_date.last_key_value();
                // We hit the last entry, nothing to do.
                let Some((datetime, _)) = entry else {
                    break;
                };

                let start_of_bracket = &bracket.start_date;
                let end_of_bracket = start_of_bracket
                    .checked_add_signed(TimeDelta::days(bracket.days.into()))
                    .context("Couldn't calculate bracket length")?;

                // This entry is before the bracket, continue with the next one.
                let entry_date = datetime.date();
                if entry_date < *start_of_bracket {
                    break 'inner;
                } else if entry_date > end_of_bracket {
                    bail!(
                        "Encountered file that's somehow in the future for {} bracket ({:?} - {:?}):\n Entry date: {:?}",
                        bracket.description,
                        bracket.start_date,
                        end_of_bracket,
                        entry_date
                    )
                }
            }

            let (datetime, entry) = files_by_date.pop_last().unwrap();
            bracket.entries.insert(datetime, entry);
        }
    }

    // Now delete all but the very first entry on each bracket.
    // So we keep
    // - One backup per day
    // - One backup per week
    // - One backup per month
    let mut final_entries = Vec::new();
    println!("\nREMOVED FILES:");
    let mut table = pretty_table();
    table.set_header(vec!["bracket", "bracket start", "filename"]);
    for bracket in brackets.into_iter() {
        let mut entries_iter = bracket.entries.into_iter();
        // Keep the very first entry.
        if let Some((_, entry)) = entries_iter.next() {
            final_entries.push((entry, bracket.description, bracket.start_date));
        }

        for (_, entry) in entries_iter {
            table.add_row(vec![
                bracket.description.to_string(),
                format!("{:?}", bracket.start_date),
                entry.file_name().to_string_lossy().to_string(),
            ]);
            if args.execute {
                remove_file(entry.path())
                    .context(format!("Failed to remove file: {:?}", entry.path()))?;
            }
        }
    }
    println!("{table}");

    println!("\nREMAINING FILES:");
    let mut table = pretty_table();
    table.set_header(vec!["bracket", "bracket start", "filename"]);
    for (entry, desc, date) in final_entries {
        table.add_row(vec![
            desc.to_string(),
            format!("{date:?}"),
            entry.file_name().to_string_lossy().to_string(),
        ]);
    }
    println!("{table}");

    Ok(())
}

struct Bracket {
    pub start_date: NaiveDate,
    /// How many days the bracket encompasses.
    pub days: u32,
    pub description: &'static str,
    /// The sorted list of all entries that're in a given bracket.
    pub entries: BTreeMap<NaiveDateTime, DirEntry>,
}

impl Bracket {
    pub fn new(start_date: NaiveDate, days: u32, description: &'static str) -> Self {
        Self {
            start_date,
            days,
            description,
            entries: BTreeMap::new(),
        }
    }
}

// The amount of days/weeks/months that should be tracked.
// There's an overlap of these brackets.
// For 30 days, 26 weeks and 24 months it would look roughly like this:
// 30 daily brackets (smallest unit)
// 26 - floor(30 / 7) = 22 weekly brackets
// 24 - floor(26 * 7 / 30) = 18 monthly brackets
const DAY_BRACKETS: u64 = 30;
const WEEK_BRACKETS: u64 = 26;
const MONTH_BRACKETS: u64 = 24;

fn init_brackets() -> Result<Vec<Bracket>> {
    let mut brackets = Vec::new();
    let mut last_daily_bracket = Utc::now().date_naive();
    // Create daily brackets
    for _ in 0..DAY_BRACKETS {
        brackets.push(Bracket::new(last_daily_bracket, 0, "daily"));
        last_daily_bracket = last_daily_bracket
            .checked_sub_days(Days::new(1))
            .context(format!(
                "Failed to go back one day from {last_daily_bracket:?}"
            ))?;
    }

    // Create weekly brackets for half a year. Start where the daily brackets end.
    let mut last_weekly_bracket = last_daily_bracket
        .checked_sub_days(Days::new(
            last_daily_bracket.weekday().num_days_from_monday().into(),
        ))
        .context("Failed to get start of week")?;

    let weekly_brackets = WEEK_BRACKETS - (DAY_BRACKETS as f64 / 7.0).floor() as u64;
    for _ in 0..weekly_brackets {
        brackets.push(Bracket::new(last_weekly_bracket, 6, "weekly"));
        last_weekly_bracket = last_weekly_bracket
            .checked_sub_days(Days::new(7))
            .context("Failed to subtract several weeks back")?;
    }

    // Create monthly brackets for 24 months and start in the month the weekly brackets end.
    // This whole thing is a bit more involved as months differ in length.
    // We save the start of the last month in each iteration, subtract a day
    let mut start_of_month = last_weekly_bracket
        .checked_sub_days(Days::new(last_weekly_bracket.day0().into()))
        .context(format!(
            "Failed to get start of month for {last_weekly_bracket}"
        ))?;

    let monthly_brackets = MONTH_BRACKETS - (WEEK_BRACKETS as f64 * 7.0 / 30.0).floor() as u64;
    for _ in 0..monthly_brackets {
        // Go one month in future and one day back to get last day of current month.
        let last_day_of_month = start_of_month
            .checked_add_months(Months::new(1))
            .unwrap()
            .checked_sub_days(Days::new(1))
            .unwrap();

        brackets.push(Bracket::new(
            start_of_month,
            last_day_of_month.day0(),
            "monthly",
        ));

        // Set the start of the month to the previous month.
        let previous_month = start_of_month
            .checked_sub_days(Days::new(20))
            .context(format!("Failed to subtract 20 days for {start_of_month}"))?;
        start_of_month = previous_month
            .checked_sub_days(Days::new(previous_month.day0().into()))
            .context(format!("Failed to get start of month for {previous_month}"))?;
    }

    Ok(brackets)
}
