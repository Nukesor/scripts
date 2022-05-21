//! This script prints a minimal summary of my todo list.
//! It's designed for use in a status bar.
use std::{fs::read_to_string, path::PathBuf, str::Lines};

use anyhow::Result;
use clap::Parser;
use script_utils::Context;

#[derive(Parser, Debug)]
pub struct CliArguments {
    /// The path to the todo markdown file.
    pub path: PathBuf,
}

/// Simply read a file and print a few lines of output
fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    if !args.path.exists() {
        println!("Nothing to do :)");
        return Ok(());
    }

    let mut output = String::new();
    let content = read_to_string(args.path).context("Failed to read file")?;
    let mut lines = content.lines();

    let mut next_todo = lines
        .find(|line| line.starts_with('#'))
        .map(|line| line.to_string());

    while let Some(headline) = next_todo {
        next_todo = handle_todo_items(&headline, &mut lines, &mut output);
    }

    if output.trim().is_empty() {
        println!("Nothing to do :) |");
        return Ok(());
    }

    println!("{output} |");

    Ok(())
}

/// Go through all lines of a todo text and extract information from it.
/// For example, the amount items that were completed.
///
/// Retuns the next todo headline, we hit one.
fn handle_todo_items(headline: &str, lines: &mut Lines, output: &mut String) -> Option<String> {
    // First things first, append the name of the todo.
    if let Some(headline) = headline.strip_prefix('#') {
        output.push_str(headline.trim());
    }

    let mut items = 0;
    let mut completed_items = 0;
    for line in lines {
        // We found the next todo. Abort.
        if line.starts_with('#') {
            // Add the current item counter and a comma for todo separation.
            if items > 0 {
                output.push_str(&format!(" ({completed_items}/{items})"))
            }
            output.push_str(", ");
            return Some(line.to_string());
        }

        // We found an unfinished item
        if line.trim().starts_with("- [ ]") || line.trim().starts_with("- []") {
            items += 1;
            continue;
        }

        // We found a finished item
        if line.trim().starts_with("- [x]") {
            items += 1;
            completed_items += 1;
        }
    }

    // Add the current item counter.
    if items > 0 {
        output.push_str(&format!(" ({completed_items}/{items})"))
    }

    None
}
