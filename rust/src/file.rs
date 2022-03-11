use anyhow::{Context, Result};
use std::{fs::read_to_string, path::PathBuf};

pub fn read_file_lines(path: &PathBuf) -> Result<Vec<String>> {
    let content = read_file(&path)?;
    Ok(content.split("\n").map(|name| name.to_string()).collect())
}

/// Read the contents of a file.
pub fn read_file(path: &PathBuf) -> Result<String> {
    read_to_string(path).context(format!("Failed to read file {:?}", path))
}

/// Read the contents of a file.
pub fn sort_and_write(mut strings: Vec<String>, path: &PathBuf) -> Result<()> {
    strings.sort();
    strings.retain(|name| !name.trim().is_empty());
    std::fs::write(path, strings.join("\n")).context(format!("Failed to write to file {:?}", path))
}
