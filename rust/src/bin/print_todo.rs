use anyhow::Result;

use script_utils::{path::read_file, path_exists};

/// Simply read a file and print a few lines of output
fn main() -> Result<()> {
    let path = "~/.local/todo";

    if !path_exists(path) {
        println!("Nothing to do :)");
        return Ok(());
    }

    let content = read_file("~/.local/todo")?;
    let concat = content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    if concat.trim().is_empty() {
        println!("Nothing to do :)");
        return Ok(());
    }

    println!("todos: {concat}");

    Ok(())
}
