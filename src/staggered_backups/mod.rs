//! A script used to implement staggered backups.
//!
//! It expects a folder that's full of files, each containing the timestamp of its creation in the
//! filename. It then deletes all files except:
//! - One file for each of the last 30 days
//! - One file for each week until roughly half a year is covered
//! - One file for each month until roughly 2 years are covered
//!
//! Optional sidecar files can be configured in `stagger.yml` and are kept or deleted together with
//! their primary backup file.
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
use std::{collections::BTreeMap, fs::File, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{NaiveDateTime, TimeDelta};
use log::error;
use regex::Regex;

use crate::{
    FileType,
    read_dir_or_fail,
    table::{pretty_table, print_headline_table},
};

mod bracket;
mod config;
mod entry;

use bracket::init_brackets;
use config::StaggerConfig;
use entry::Entry;

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub date_extraction_regex: Option<String>,
    pub date_format: Option<String>,
    pub execute: bool,
}

pub fn run_staggered_backups(path: &PathBuf, options: &RunOptions) -> Result<()> {
    println!("═══════════════════════════════════════════════════════════════");
    print_headline_table(format!("Checking folder: {path:?}"));

    let config_path = path.join("stagger.yml");
    let mut config = if config_path.exists() {
        println!("Found stagger config file");
        serde_yaml::from_reader(File::open(config_path)?)?
    } else {
        StaggerConfig::default()
    };
    config.validate()?;

    if let Some(regex) = &options.date_extraction_regex {
        config.regex = Some(regex.clone());
    }
    if let Some(format) = &options.date_format {
        config.format = Some(format.clone());
    }

    let files = read_dir_or_fail(path, Some(FileType::File)).context("Failed to read files")?;
    let mut files_by_name = BTreeMap::new();
    for file in files {
        let name = file
            .path()
            .file_name()
            .context(format!("Got file without filename: {:?}", file.path()))?
            .to_string_lossy()
            .to_string();

        // Ignore the stagger.yml
        if name == "stagger.yml" {
            continue;
        }

        files_by_name.insert(name, file);
    }
    let mut files_by_date = BTreeMap::new();

    // Build the regex once and ensure it actually exposes the datetime capture we rely on.
    let re = build_date_regex(&config)?;
    let date_format = config.format();

    // Go through all files and extract the datetime from its filename.
    let file_names = files_by_name.keys().cloned().collect::<Vec<_>>();
    for name in file_names {
        let Some(captures) = re.captures(&name) else {
            continue;
        };

        let datetime = NaiveDateTime::parse_from_str(&captures[1], &date_format);
        let datetime = match datetime {
            Ok(datetime) => datetime,
            Err(_) => {
                error!("Failed to parse date string. Ignoring file: {name}");
                continue;
            }
        };

        // We got a valid file. Get the file from the files_by_name list.
        let Some(file) = files_by_name.remove(&name) else {
            continue;
        };

        // Sidecars are looked up by appending each configured suffix to the primary filename.
        let sidecars = config
            .sidecar
            .iter()
            .filter_map(|sidecar| files_by_name.remove(&format!("{name}{}", sidecar.suffix)))
            .collect();

        files_by_date.insert(datetime, Entry::new(file, sidecars));
    }
    if files_by_date.is_empty() {
        println!("No files for backup found.");
        return Ok(());
    }

    // Make sure there're no superfluous files.
    if !files_by_name.is_empty() {
        error!("Found unmatched files in directory:");
        for name in files_by_name.keys() {
            error!("Didn't match name: {name}");
        }
        bail!("Aborting due to unmatched files");
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
    table.set_header(vec!["bracket", "bracket start", "files"]);
    for bracket in brackets {
        let mut entries_iter = bracket.entries.into_iter();
        // Keep the very first entry.
        if let Some((_, entry)) = entries_iter.next() {
            final_entries.push((entry, bracket.description, bracket.start_date));
        }

        for (_, entry) in entries_iter {
            table.add_row(vec![
                bracket.description.to_string(),
                format!("{:?}", bracket.start_date),
                entry.filenames().join("\n"),
            ]);
            if options.execute {
                entry.remove_files()?;
            }
        }
    }

    // Anything that's left was older than the longest configured bracket and can be fully removed.
    for (_, entry) in files_by_date {
        table.add_row(vec![
            "expired".to_string(),
            "-".to_string(),
            entry.filenames().join("\n"),
        ]);
        if options.execute {
            entry.remove_files()?;
        }
    }
    println!("{table}");

    println!("\nREMAINING FILES:");
    let mut table = pretty_table();
    table.set_header(vec!["bracket", "bracket start", "files"]);
    for (entry, desc, date) in final_entries {
        table.add_row(vec![
            desc.to_string(),
            format!("{date:?}"),
            entry.filenames().join("\n"),
        ]);
    }
    println!("{table}");

    Ok(())
}

fn build_date_regex(config: &StaggerConfig) -> Result<Regex> {
    // Build the regex once and ensure it actually exposes the datetime capture we rely on.
    let regex_pattern = config.regex();
    let re = Regex::new(&regex_pattern).context(format!(
        "Found invalid date_extraction_regex: {regex_pattern}"
    ))?;
    if re.captures_len() < 2 {
        bail!(
            "The date_extraction_regex must contain a capture group for the datetime string: {regex_pattern}"
        );
    }

    Ok(re)
}
