use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, Utc};

use super::entry::Entry;

pub struct Bracket {
    pub start_date: NaiveDate,
    /// How many days the bracket encompasses.
    pub days: u32,
    pub description: &'static str,
    /// The sorted list of all entries that're in a given bracket.
    pub entries: BTreeMap<NaiveDateTime, Entry>,
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

pub fn init_brackets() -> Result<Vec<Bracket>> {
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
