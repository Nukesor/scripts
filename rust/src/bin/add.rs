use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crossterm::style::{style, Attribute, Color, Stylize};
use script_utils::prelude::*;

#[derive(Parser, Debug)]
#[clap(
    name = "Add",
    about = "Add a package to your package list",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// The packages that should be uninstalled.
    pub packages: Vec<String>,

    #[clap(short, long)]
    pub pkglist_file: Option<PathBuf>,

    /// For AUR operations, another manager and pkglist will be used by default.
    #[clap(short, long)]
    pub aur: bool,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    let pkglist_path = if let Some(path) = &args.pkglist_file {
        expand(path)
    } else if args.aur {
        expand(&PathBuf::from("~/.setup/aur-pkglist"))
    } else {
        expand(&PathBuf::from("~/.setup/pkglist"))
    };

    let mut pkglist: Vec<String> =
        read_file_lines(&pkglist_path).context("Failed to read pkglist file.")?;

    let mut results = Vec::new();

    // Install the packages
    for package in args.packages.iter() {
        results.push((package.to_string(), install_package(args.aur, package)?));
    }

    for (name, result) in results {
        handle_result(&mut pkglist, &name, result);
    }

    // Write the packagelist
    sort_and_write(pkglist, &pkglist_path)?;

    Ok(())
}

enum InstallResult {
    Success,
    Installed,
    Failed(String),
}

fn handle_result(pkglist: &mut Vec<String>, name: &str, result: InstallResult) {
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
            let added_text = if add_to_list(pkglist, name) {
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
            let added_text = if add_to_list(pkglist, name) {
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

fn install_package(aur: bool, name: &str) -> Result<InstallResult> {
    let manager = if aur { "paru" } else { "pacman" };
    let sudo = if aur { "" } else { "sudo " };

    // Check if the package is already installed
    let capture = Cmd::new(format!("{sudo}{manager} -Qi {name}")).run()?;
    let is_installed = capture.success();

    if !is_installed {
        let capture = Cmd::new(format!("{sudo}{manager} -S {name} --noconfirm --needed")).run()?;

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

    list.push(name);

    true
}
