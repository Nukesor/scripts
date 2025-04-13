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
use log::{debug, error, info};
use regex::Regex;
use script_utils::{FileType, logging, read_dir_or_fail};

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
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    // Min log level INFO
    logging::init_logger(args.verbose + 2);

    let files =
        read_dir_or_fail(&args.path, Some(FileType::File)).context("Failed to read files")?;
    let mut files_by_date = BTreeMap::new();

    println!(
        "Running staggered backup cleanup for folder: {:?}",
        args.path
    );
    if !args.execute {
        println!("--- DRY RUN MODE ---");
    }

    // Go through all files and extract the datetime from its filename.
    for file in files {
        let name = file
            .path()
            .file_stem()
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
                        "Encountered file that's somehow in the future: {entry_date:?} > {end_of_bracket:?}"
                    )
                }
            }

            let (datetime, entry) = files_by_date.pop_last().unwrap();
            bracket.entries.insert(datetime, entry);
        }
    }

    // Now delete all but the very first entry on each bracket.
    // So we keep
    // - One backup per day for the first 30 days
    // - One backup per week (usually monday) for 26 weeks.
    // - One backup per month (usually the 1st) for 2 years.
    for bracket in brackets.into_iter() {
        let mut entries_iter = bracket.entries.into_iter();
        // Keep the very first entry.
        if let Some((_, entry)) = entries_iter.next() {
            debug!(
                "Keeping backup {:?} for {} bracket {:?}",
                entry.file_name(),
                bracket.description,
                bracket.start_date,
            )
        }

        for (_, entry) in entries_iter {
            info!(
                "Removing file {:?} for {} bracket {:?}",
                entry.file_name(),
                bracket.description,
                bracket.start_date,
            );
            if args.execute {
                remove_file(entry.path())
                    .context(format!("Failed to remove file: {:?}", entry.path()))?;
            }
        }
    }

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

fn init_brackets() -> Result<Vec<Bracket>> {
    let mut brackets = Vec::new();
    let today = Utc::now().date_naive();
    // Create daily brackets for the last 30 days
    for days_back in 0..30 {
        let bracket_start = today
            .checked_sub_days(Days::new(days_back))
            .context(format!("Failed to subtract {days_back} days from today"))?;

        brackets.push(Bracket::new(bracket_start, 0, "daily"));
    }

    // Create weekly brackets for the last 22 weeks (half a year - ~30 days)
    let one_month_back = today
        .checked_sub_days(Days::new(30))
        .context("Failed to subtract 30 days from today")?;
    let monday_of_first_week = one_month_back
        .checked_sub_days(Days::new(
            one_month_back.weekday().number_from_monday().into(),
        ))
        .context("Failed to get start of week")?;
    for weeks_back in 0..22 {
        let bracket_start = monday_of_first_week
            .checked_sub_days(Days::new(7 * weeks_back))
            .context("Failed to subtract several weeks back")?;

        brackets.push(Bracket::new(bracket_start, 7, "weekly"));
    }

    // Create monthly brackets for 19 months (2 years - ~22 weeks. A bit of overlap to be safe).
    // This is a bit more involved as months differ in length.
    // For this, we save the start of the last month in each iteration.
    let half_year_back = today
        .checked_sub_days(Days::new(26 * 7))
        .context("Failed to go half a year back")?;

    let mut start_of_month = half_year_back
        .checked_sub_days(Days::new(half_year_back.day0().into()))
        .context(format!("Failed to get start of month for {half_year_back}"))?;

    for _ in 0..19 {
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
