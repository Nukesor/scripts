//! A tool to uniformly name audio releases.
use std::{env::current_dir, fs::DirEntry, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crossterm::style::{style, Attribute, Color, Stylize};
use script_utils::prelude::*;

#[derive(Parser, Debug)]
#[clap(name = "Renamer", about = "", author = "Arne Beer <contact@arne.beer>")]
struct CliArguments {}

fn main() -> Result<()> {
    // Parse commandline options.
    let _args = CliArguments::parse();

    let cwd = current_dir()?;
    let entries = read_dir_or_fail(&cwd, Some(FileType::Directory))?;

    for entry in entries {
        handle_entry(&cwd, entry)?;
    }

    Ok(())
}

#[derive(Default)]
struct Metadata {
    artist: Option<String>,
    year: Option<usize>,
    catalogue_no: Option<String>,
    source: Option<String>,
    format: Option<String>,
}

fn handle_entry(cwd: &PathBuf, entry: DirEntry) -> Result<()> {
    let name = entry.file_name().to_string_lossy().to_string();
    let path = entry.path();
}

fn metadata_from_name(name: &str) -> Result<Metadata> {
    let metadata = Metadata::default();

    Ok(metadata)
}
