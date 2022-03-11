use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crossterm::style::{style, Attribute, Color, Stylize};
use script_utils::{path::*, process::Cmd};

#[derive(Parser, Debug)]
#[clap(
    name = "Add",
    about = "Add a package to your package list",
    author = "Arne Beer <contact@arne.beer>"
)]
pub struct CliArguments {
    pub packages: Vec<String>,

    #[clap(short, long)]
    pub pkglist_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    let pkglist_path = args
        .pkglist_file
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "~/.setup/pkglist".to_string());

    let pkglist_path = expand(pkglist_path);

    let mut pkglist: Vec<String> = read_file(&pkglist_path)
        .context("Failed to read pkglist file.")?
        .split("\n")
        .map(|name| name.to_string())
        .collect();

    let mut results = Vec::new();

    // Install the packages
    for package in args.packages.iter() {
        results.push((package.to_string(), install_package(package)?));
    }

    for (name, result) in results {
        match result {
            InstallResult::Failed(output) => {
                println!(
                    "{} to install {} with error:\n{}",
                    style("Failed").with(Color::Red),
                    style(name).attribute(Attribute::Bold),
                    output
                );
            }
            InstallResult::Success => {
                let added_text = if add_to_list(&mut pkglist, &name) {
                    style(" and added it to the pkglist")
                } else {
                    style(", but it was already in the pkglist.").with(Color::Yellow)
                };

                println!(
                    " {} {}{}",
                    style(name).attribute(Attribute::Bold),
                    style("has been installed").with(Color::Green),
                    added_text,
                );
            }
            InstallResult::Installed => {
                let added_text = if add_to_list(&mut pkglist, &name) {
                    style(", but it wasn't in the pkglist yet.").with(Color::Yellow)
                } else {
                    style(" and in the pkglist")
                };

                println!(
                    " {} is {}{}",
                    style(name).attribute(Attribute::Bold),
                    style("already installed").with(Color::Green),
                    added_text,
                );
            }
        }
    }

    // Write the packagelist
    pkglist.sort();
    std::fs::write(pkglist_path, pkglist.join("\n")).context("Failed to write pkglist file")?;

    Ok(())
}

enum InstallResult {
    Success,
    Installed,
    Failed(String),
}

fn install_package(name: &str) -> Result<InstallResult> {
    // Check if the package is already installed
    let capture = Cmd::new(format!("sudo pacman -Qi {name}")).run()?;
    let is_installed = capture.success();

    if !is_installed {
        let capture = Cmd::new(format!("sudo pacman -S {name} --noconfirm --needed")).run()?;

        if !capture.exit_status.success() {
            return Ok(InstallResult::Failed(capture.stdout_str()));
        } else {
            return Ok(InstallResult::Success);
        }
    }

    Ok(InstallResult::Installed)
}

fn add_to_list(list: &mut Vec<String>, name: &str) -> bool {
    let name = name.to_string();
    if list.contains(&name) {
        return false;
    }

    list.push(name.to_string());

    true
}
