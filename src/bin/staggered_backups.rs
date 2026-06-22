//! CLI entrypoint for staggered backups.
use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser};
use script_utils::{
    fs::find_leaf_dirs,
    logging,
    staggered_backups::{RunOptions, run_staggered_backups},
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
    #[clap(short = 'd', long)]
    pub date_extraction_regex: Option<String>,

    /// The date format string that's used in the filename
    /// E.g. "%Y-%m-%d_%H-%M" for "2025-04-02_00-00.dump"
    #[clap(short = 'f', long)]
    pub date_format: Option<String>,

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

    let options = RunOptions {
        date_extraction_regex: args.date_extraction_regex.clone(),
        date_format: args.date_format.clone(),
        execute: args.execute,
    };

    if !args.recursive {
        run_staggered_backups(&args.path, &options)?;
    } else {
        let leaf_dirs = find_leaf_dirs(args.path.clone())?;
        let mut leaf_dirs_iter = leaf_dirs.iter().peekable();
        while let Some(dir) = leaf_dirs_iter.next() {
            run_staggered_backups(dir, &options)?;
            if leaf_dirs_iter.peek().is_some() {
                println!("\n");
            }
        }
    }

    Ok(())
}
