//! Get the output of the following command and sort them by execution time.
//! ```sh
//!     cargo +nightly test --
//!         --quiet \
//!         -Z unstable-options \
//!         --format json \
//!         --report-time > target/debug/test.json
//! ```
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser};
use comfy_table::Table;
use serde::Deserialize;

use script_utils::logging;

#[derive(Debug, Deserialize)]
enum Event {
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "ok")]
    Ok,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct TestReport {
    /// What kind of event triggered this report.
    event: Event,
    /// The name of either the suite or the name of the test.
    name: String,
    /// The execution time of this single test.
    exec_time: Option<f32>,
}

/// Main output for outpuft for
#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Report {
    /// Info about a full test suite.
    #[serde(rename = "suite")]
    Suite {
        /// What kind of event triggered this report.
        event: Event,
        /// The amount of tests that're filtered out.
        test_count: Option<usize>,
        /// The execution time of the full suite.
        exec_time: Option<f32>,
        /// The amount of tests that passed.
        passed: Option<usize>,
        /// The amount of ignored tests.
        ignored: Option<usize>,
        /// The amount of measured tests.
        measured: Option<usize>,
        /// The amount of failed tests.
        failed: Option<usize>,
        /// The amount of tests that're filtered out.
        filtered_out: Option<usize>,
    },
    /// Info about an actual test.
    #[serde(rename = "test")]
    Test(TestReport),
}

#[derive(Parser, Debug)]
#[clap(
    name = "Slow Rust Test Finder",
    about = "Sort and format a list of test execution time so we can easily find slow tests.",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// The path to the json test result file.
    pub path: PathBuf,

    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// Any tests below this value in 'ms' won't be shown in the resulting table.
    #[clap(short, long, default_value = "500")]
    pub threshold: usize,
}

/// Print a string, representing the current network state with IP.
fn main() -> Result<()> {
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    let file = std::fs::read_to_string(&args.path).context("Failed to read test state file:")?;

    // Collect all reports of finished successful tests.
    let mut tests = Vec::new();

    // Each line in this document is a full
    for line in file.lines() {
        let report: Report =
            serde_json::from_str(line).context(format!("Failed to parse line: {line}"))?;
        match report {
            Report::Suite { .. } => continue,
            Report::Test(test) => {
                // Don't add non-successful tests to the list.
                if !matches!(test.event, Event::Ok) {
                    continue;
                }
                if let Some(exec_time) = test.exec_time {
                    // Don't display tests that're below the minimum thresold.
                    if args.threshold as f32 / 1000.0 > exec_time {
                        continue;
                    }
                    tests.push(test);
                }
            }
        }
    }

    tests.sort_by(|a, b| a.exec_time.partial_cmp(&b.exec_time).unwrap());

    let mut table = Table::new();
    table.set_header(vec!["Exec time", "name"]);
    table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
    for test in tests {
        table.add_row(vec![
            format!("{:.2}", test.exec_time.unwrap()),
            test.name.to_string(),
        ]);
    }

    println!("{table}");

    Ok(())
}
