use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crossterm::style::{style, Attribute, Color, Stylize};
use script_utils::prelude::*;

#[derive(Parser, Debug)]
#[clap(
    name = "Remove",
    about = "Remove a package from your package list",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    pub packages: Vec<String>,

    #[clap(short, long, default_value = "~/.setup/pkglist")]
    pub pkglist_file: PathBuf,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    let pkglist_path = expand(&args.pkglist_file);
    let mut pkglist: Vec<String> =
        read_file_lines(&pkglist_path).context("Failed to read pkglist file.")?;

    let mut results = Vec::new();

    // Install the packages
    for package in args.packages.iter() {
        results.push((package.to_string(), uninstall_package(package)?));
    }

    for (name, result) in results {
        match result {
            InstallResult::Failed(output) => {
                println!(
                    "{} to uninstall {} with error:\n{}",
                    style("Failed").with(Color::Red),
                    style(name).attribute(Attribute::Bold),
                    output
                );
            }
            InstallResult::Success => {
                let removed_text = if removed_from_list(&mut pkglist, &name) {
                    style(" and removed from to the pkglist")
                } else {
                    style(", but it wasn't in the pkglist.").with(Color::Yellow)
                };

                println!(
                    " {} {}{}",
                    style(name).attribute(Attribute::Bold),
                    style("has been uninstalled").with(Color::Green),
                    removed_text,
                );
            }
            InstallResult::NotInstalled => {
                let removed_text = if removed_from_list(&mut pkglist, &name) {
                    style(", but it was in the pkglist.").with(Color::Yellow)
                } else {
                    style(" and not in the pkglist")
                };

                println!(
                    " {} {}{}",
                    style(name).attribute(Attribute::Bold),
                    style("was not installed").with(Color::Green),
                    removed_text,
                );
            }
        }
    }

    // Write the packagelist
    pkglist.sort();
    pkglist.retain(|name| !name.trim().is_empty());

    std::fs::write(pkglist_path, pkglist.join("\n")).context("Failed to write pkglist file")?;

    Ok(())
}

enum InstallResult {
    Success,
    NotInstalled,
    Failed(String),
}

fn uninstall_package(name: &str) -> Result<InstallResult> {
    // Check if the package is already installed
    let capture = Cmd::new(format!("sudo pacman -Qi {name}")).run()?;
    let is_installed = capture.success();

    if is_installed {
        let capture = Cmd::new(format!("sudo pacman -Rns {name} --noconfirm")).run()?;

        if !capture.exit_status.success() {
            return Ok(InstallResult::Failed(capture.stdout_str()));
        } else {
            return Ok(InstallResult::Success);
        }
    }

    Ok(InstallResult::NotInstalled)
}

fn removed_from_list(list: &mut Vec<String>, name: &str) -> bool {
    let name = name.to_string();
    let index = list.iter().position(|n| n == &name);
    match index {
        Some(index) => {
            list.remove(index);
            true
        }
        None => false,
    }
}
