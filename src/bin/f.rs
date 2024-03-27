//! A collection of helpful file system operations.
//!
//! - Get a list of all top-level git repositories
use std::{fs::read_dir, path::PathBuf};

use anyhow::Result;
use clap::{ArgAction, Parser};
use log::debug;

use script_utils::logging;

#[derive(Parser, Debug)]
#[clap(
    name = "File helpers",
    about = "A collection of helpful file system operations.",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Find all git repos in a given directory
    FindRepos {
        paths: Vec<PathBuf>,
        #[clap(short, long, default_value = "5")]
        max_depth: usize,
    },
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    match args.cmd {
        SubCommand::FindRepos { paths, max_depth } => {
            // Find repos up to a depth of 5 directories.
            let mut repos = Vec::new();
            for path in paths {
                discover_repos(&path, 0, max_depth, &mut repos);
            }

            // Print the list of repos to stdout with newlines
            println!(
                "{}",
                repos
                    .into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            );
        }
    }

    Ok(())
}

/// Discover repositories inside a given folder.
///
/// This function is copy-pasted from `geil`.
/// If anything changes, consider backporting.
pub fn discover_repos(
    path: &PathBuf,
    depths: usize,
    max_depth: usize,
    new_repos: &mut Vec<PathBuf>,
) {
    // Check if a .git directory exists.
    // If it does, always stop searching.
    let git_dir = path.join(".git");
    debug!("{} Looking at folder {:?}", depths, path);
    if git_dir.exists() {
        debug!("Found .git folder");
        // Add the repository, if we don't know it yet.
        new_repos.push(path.to_owned());
        return;
    }

    // Recursion stop. Only check up to a dephts of 5
    if depths == max_depth {
        debug!("Max depth reached");
        return;
    }

    let current_dir = match read_dir(path) {
        Ok(current_dir) => current_dir,
        Err(err) => {
            debug!(
                "Couldn't read directory at {:?} with error: {:?}",
                path, err
            );
            return;
        }
    };

    // The current path is no repository, search it's subdirectories
    for entry_result in current_dir {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                discover_repos(&path, depths + 1, max_depth, new_repos);
            }
            Err(err) => {
                debug!(
                    "Couldn't read directory path {:?} with error: {:?}",
                    path, err
                );
                continue;
            }
        }
    }
}
