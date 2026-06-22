use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{Days, NaiveDateTime, TimeDelta, Utc};
use tempfile::TempDir;

use super::{
    RunOptions,
    collect_entries,
    load_config,
    run_staggered_backups,
    sort_entries_into_brackets,
};

/// A created backup entry and all filenames that belong to it.
struct CreatedEntry {
    primary: String,
    sidecars: Vec<String>,
}

/// Build simple staggered-backup test directories in a temporary location.
struct ScenarioBuilder {
    tempdir: TempDir,
    sidecar_suffixes: Vec<String>,
}

impl ScenarioBuilder {
    /// Create a fresh temporary scenario without sidecars.
    fn new() -> Result<Self> {
        Ok(Self {
            tempdir: TempDir::new().context("Failed to create temporary test directory")?,
            sidecar_suffixes: Vec::new(),
        })
    }

    /// Enable one sidecar suffix and update the local `stagger.yml` file.
    fn with_sidecar_suffix(mut self, suffix: &str) -> Result<Self> {
        self.sidecar_suffixes.push(suffix.to_string());
        self.write_config()?;
        Ok(self)
    }

    /// Return the temporary directory path used by this scenario.
    fn path(&self) -> PathBuf {
        self.tempdir.path().to_path_buf()
    }

    /// Create a backup file without any sidecars.
    fn create_backup(&self, prefix: &str, datetime: NaiveDateTime) -> Result<CreatedEntry> {
        let filename = backup_filename(prefix, datetime);
        self.write_file(&filename)?;

        Ok(CreatedEntry {
            primary: filename,
            sidecars: Vec::new(),
        })
    }

    /// Create a backup file together with one file per configured sidecar suffix.
    fn create_backup_with_sidecars(
        &self,
        prefix: &str,
        datetime: NaiveDateTime,
    ) -> Result<CreatedEntry> {
        let primary = backup_filename(prefix, datetime);
        self.write_file(&primary)?;

        let mut sidecars = Vec::new();
        for suffix in &self.sidecar_suffixes {
            let filename = format!("{primary}{suffix}");
            self.write_file(&filename)?;
            sidecars.push(filename);
        }

        Ok(CreatedEntry { primary, sidecars })
    }

    /// Check whether a filename still exists in the temporary directory.
    fn exists(&self, filename: &str) -> bool {
        self.tempdir.path().join(filename).exists()
    }

    /// Write one file with placeholder test contents.
    fn write_file(&self, filename: &str) -> Result<()> {
        fs::write(self.tempdir.path().join(filename), b"test")
            .context(format!("Failed to write test file: {filename}"))
    }

    /// Write the current sidecar configuration to `stagger.yml`.
    fn write_config(&self) -> Result<()> {
        let config_path = self.tempdir.path().join("stagger.yml");
        if self.sidecar_suffixes.is_empty() {
            if config_path.exists() {
                fs::remove_file(&config_path).context("Failed to remove stale stagger.yml")?;
            }
            return Ok(());
        }

        let mut contents = String::from("sidecar:\n");
        for suffix in &self.sidecar_suffixes {
            contents.push_str(&format!("  - suffix: \"{suffix}\"\n"));
        }

        fs::write(&config_path, contents).context("Failed to write stagger.yml")
    }
}

/// Build a default backup filename for a given logical prefix and timestamp.
fn backup_filename(prefix: &str, datetime: NaiveDateTime) -> String {
    format!("{prefix}_{}.dump", datetime.format("%Y-%m-%d_%H-%M"))
}

/// Return one representative date from the newest daily, weekly, and monthly brackets.
fn representative_bracket_dates() -> Result<(NaiveDateTime, NaiveDateTime, NaiveDateTime)> {
    let (brackets, _) = sort_entries_into_brackets(Default::default())?;
    let daily = brackets
        .iter()
        .find(|bracket| bracket.description == "daily")
        .context("Failed to find daily bracket")?;
    let weekly = brackets
        .iter()
        .find(|bracket| bracket.description == "weekly")
        .context("Failed to find weekly bracket")?;
    let monthly = brackets
        .iter()
        .find(|bracket| bracket.description == "monthly")
        .context("Failed to find monthly bracket")?;

    Ok((
        daily.start_date.and_hms_opt(10, 0, 0).unwrap(),
        weekly.start_date.and_hms_opt(10, 0, 0).unwrap(),
        monthly.start_date.and_hms_opt(10, 0, 0).unwrap(),
    ))
}

/// Return one timestamp that is older than the oldest configured bracket.
fn expired_date() -> Result<NaiveDateTime> {
    let (brackets, _) = sort_entries_into_brackets(Default::default())?;
    let oldest_bracket = brackets.last().context("Failed to find oldest bracket")?;
    let expired_date = oldest_bracket
        .start_date
        .checked_sub_days(Days::new(1))
        .context("Failed to derive expired date")?;

    Ok(expired_date.and_hms_opt(10, 0, 0).unwrap())
}

#[test]
fn sidecars_are_correctly_detected() -> Result<()> {
    let builder = ScenarioBuilder::new()?.with_sidecar_suffix("-journal")?;
    let daily_date = Utc::now().date_naive().and_hms_opt(10, 0, 0).unwrap();
    let created = builder.create_backup_with_sidecars("backup", daily_date)?;

    let config = load_config(&builder.path(), &RunOptions::default())?;
    let entries = collect_entries(&builder.path(), &config)?;

    assert_eq!(entries.len(), 1);
    let entry = entries.values().next().unwrap();
    assert_eq!(
        entry.dir_entry.file_name().to_string_lossy(),
        created.primary
    );
    assert_eq!(entry.sidecars.len(), 1);
    assert_eq!(
        entry.sidecars[0].file_name().to_string_lossy(),
        created.sidecars[0]
    );

    Ok(())
}

#[test]
fn files_for_each_bracket_are_correctly_removed() -> Result<()> {
    let builder = ScenarioBuilder::new()?;
    let (daily_date, weekly_date, monthly_date) = representative_bracket_dates()?;
    let expired = expired_date()?;

    let daily_old = builder.create_backup("daily", daily_date)?;
    let daily_new = builder.create_backup("daily", daily_date + TimeDelta::hours(10))?;
    let weekly_old = builder.create_backup("weekly", weekly_date)?;
    let weekly_new = builder.create_backup("weekly", weekly_date + TimeDelta::hours(10))?;
    let monthly_old = builder.create_backup("monthly", monthly_date)?;
    let monthly_new = builder.create_backup("monthly", monthly_date + TimeDelta::hours(10))?;
    let expired_entry = builder.create_backup("expired", expired)?;

    run_staggered_backups(
        &builder.path(),
        &RunOptions {
            execute: true,
            ..Default::default()
        },
    )?;

    assert!(builder.exists(&daily_old.primary));
    assert!(!builder.exists(&daily_new.primary));
    assert!(builder.exists(&weekly_old.primary));
    assert!(!builder.exists(&weekly_new.primary));
    assert!(builder.exists(&monthly_old.primary));
    assert!(!builder.exists(&monthly_new.primary));
    assert!(!builder.exists(&expired_entry.primary));

    Ok(())
}

#[test]
fn duplicates_inside_a_bracket_are_correctly_removed() -> Result<()> {
    let builder = ScenarioBuilder::new()?;
    let daily_date = Utc::now().date_naive().and_hms_opt(10, 0, 0).unwrap();

    let older = builder.create_backup("duplicate", daily_date)?;
    let newer = builder.create_backup("duplicate", daily_date + TimeDelta::hours(5))?;

    run_staggered_backups(
        &builder.path(),
        &RunOptions {
            execute: true,
            ..Default::default()
        },
    )?;

    assert!(builder.exists(&older.primary));
    assert!(!builder.exists(&newer.primary));

    Ok(())
}

#[test]
fn sidecars_are_also_removed() -> Result<()> {
    let builder = ScenarioBuilder::new()?.with_sidecar_suffix("-journal")?;
    let daily_date = Utc::now().date_naive().and_hms_opt(10, 0, 0).unwrap();

    let retained = builder.create_backup_with_sidecars("retained", daily_date)?;
    let removed =
        builder.create_backup_with_sidecars("retained", daily_date + TimeDelta::hours(5))?;

    run_staggered_backups(
        &builder.path(),
        &RunOptions {
            execute: true,
            ..Default::default()
        },
    )?;

    assert!(builder.exists(&retained.primary));
    assert!(builder.exists(&retained.sidecars[0]));
    assert!(!builder.exists(&removed.primary));
    assert!(!builder.exists(&removed.sidecars[0]));

    Ok(())
}
