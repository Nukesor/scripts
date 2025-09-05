//! This script prints a minimal summary of my todo list.
//! It's designed for use in a status bar.
use std::{fs::read_to_string, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use script_utils::Context;
use serde::Serialize;

#[derive(Parser, Debug)]
pub struct CliArguments {
    /// The path to the todo markdown file.
    pub path: PathBuf,
}

#[derive(Serialize, Debug)]
pub struct Output {
    text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tooltip: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Todo {
    pub name: String,
    pub items: Vec<Item>,
}

impl Todo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            items: Vec::new(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Item {
    pub name: String,
    pub completed: bool,
}

impl Item {
    pub fn new(name: String, completed: bool) -> Self {
        Self { name, completed }
    }
}

pub fn todos_as_i3bar_output(_todos: Vec<Todo>) -> Output {
    let text = String::new();

    Output {
        text,
        tooltip: String::new(),
    }
}

pub fn todos_as_waybar_output(todos: Vec<Todo>) -> Output {
    let mut text = String::new();
    let mut tooltip = String::new();

    let todo_count = todos.len();
    if todo_count == 0 {
        text.push_str("Neat :3")
    } else {
        text = format!("{todo_count} todos")
    }

    for todo in todos {
        tooltip.push_str(" ");
        tooltip.push_str(&todo.name);
        for item in todo.items {
            tooltip.push('\r');
            if item.completed {
                tooltip.push('');
            } else {
                tooltip.push('󱘹');
            }
            tooltip.push_str(&item.name);
        }
        tooltip.push('\r');
        tooltip.push('\r');
    }

    println!("{text}");
    println!("{tooltip}");

    Output { text, tooltip }
}

/// Simply read a file and print a few lines of output
fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    if !args.path.exists() {
        println!("Nothing to do :)");
        return Ok(());
    }

    let content = read_to_string(args.path).context("Failed to read file")?;
    let todos = handle_todo_items(content);

    let output = todos_as_waybar_output(todos);

    // Send the expected json output to i3status
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

/// Go through all lines of a todo text and extract information from it.
/// For example, the amount items that were completed.
///
/// Retuns the next todo headline, we hit one.
fn handle_todo_items(content: String) -> Vec<Todo> {
    let mut todos = Vec::new();

    let mut todo: Option<Todo> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            // We found a new todo
            // If we already have one, save it to the list before starting a new one.
            if let Some(todo) = &todo {
                todos.push(todo.clone());
            }
            let name = line.strip_prefix('#').unwrap_or_default().trim();
            todo = Some(Todo::new(name.into()));
        } else if let Some(ref mut todo) = todo {
            if line.starts_with('-') && !line.starts_with("- [x]") {
                let name = line
                    .strip_prefix("- [ ]")
                    .or(line.strip_prefix("- []"))
                    .or(line.strip_prefix("-"))
                    .unwrap();
                todo.items.push(Item::new(name.to_string(), false));
            } else if line.starts_with("- [x]") {
                let name = line
                    .strip_prefix("- [x]")
                    .or(line.strip_prefix("-[x]"))
                    .unwrap();
                todo.items.push(Item::new(name.to_string(), true));
            }
        }
    }

    if let Some(todo) = todo {
        todos.push(todo);
    }

    todos
}
