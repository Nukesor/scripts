use anyhow::Result;

use script_utils::{path::read_file, path_exists};

/// Simply read a file and print a few lines of output
fn main() -> Result<()> {
    let path = "~/Syncthing/Transfer/todo";

    if !path_exists(path) {
        println!("Nothing to do :)");
        return Ok(());
    }

    let content = read_file(path)?;
    let concat = content
        .lines()
        .filter(|line| line.starts_with("#"))
        .map(|line| {
            let line = line.clone().strip_prefix("#");
            let line = line.unwrap().trim();
            line.to_string()
        })
        .collect::<Vec<String>>()
        .join(", ");

    if concat.trim().is_empty() {
        println!("Nothing to do :) |");
        return Ok(());
    }

    println!("todos: {concat} |");

    Ok(())
}
