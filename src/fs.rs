//! All file system related helper functions.
use std::fs::{read_to_string, DirEntry};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use shellexpand::tilde;

pub use file::*;
pub use path::*;

pub mod path {
    use super::*;
    /// Expand the tilde and return a valid PathBuf.
    pub fn expand(path: &Path) -> PathBuf {
        let path = tilde(&path.to_string_lossy()).to_string();
        PathBuf::from(&path)
    }

    /// Check if a file exists.
    pub fn path_exists<T: ToString>(path: T) -> bool {
        Path::new(&tilde(&path.to_string()).to_string()).exists()
    }
}

pub mod file {
    use super::*;

    pub enum FileType {
        Directory,
        File,
    }

    pub fn read_file_lines(path: &PathBuf) -> Result<Vec<String>> {
        let content = read_file(path)?;
        Ok(content.split('\n').map(|name| name.to_string()).collect())
    }

    /// Read the contents of a file.
    pub fn read_file(path: &PathBuf) -> Result<String> {
        read_to_string(path).context(format!("Failed to read file {path:?}"))
    }

    /// Read the contents of a file.
    pub fn sort_and_write(mut strings: Vec<String>, path: &PathBuf) -> Result<()> {
        strings.sort();
        strings.retain(|name| !name.trim().is_empty());
        std::fs::write(path, strings.join("\n"))
            .context(format!("Failed to write to file {path:?}"))
    }

    /// Read all entries of a directory and return them.
    /// If a FileType is specified, only files with that type will be returned.
    pub fn read_dir_or_fail(path: PathBuf, file_type: Option<FileType>) -> Result<Vec<DirEntry>> {
        let dir = std::fs::read_dir(path)?;

        let mut entries: Vec<DirEntry> = Vec::new();
        for entry_result in dir {
            let entry = entry_result?;

            // Filter not matching file types
            if let Some(file_type) = &file_type {
                match file_type {
                    FileType::Directory => {
                        if !entry.file_type()?.is_dir() {
                            continue;
                        }
                    }
                    FileType::File => {
                        if !entry.file_type()?.is_file() {
                            continue;
                        }
                    }
                }
            }

            entries.push(entry);
        }

        Ok(entries)
    }

    /// Return the file with the newest 'modified' date in a directory.
    pub fn get_newest_file(path: PathBuf) -> Result<Option<PathBuf>> {
        let dir = std::fs::read_dir(path)?;

        let mut path: Option<PathBuf> = None;
        let mut modified = SystemTime::UNIX_EPOCH;

        for entry_result in dir {
            let entry = entry_result?;
            let metadata = entry.metadata()?;

            // We're looking at the first file. Use it as a base-line.
            if path.is_none() {
                path = Some(entry.path());
                modified = metadata.modified()?;
                continue;
            }

            let last_modified = metadata.modified()?;
            if last_modified > modified {
                modified = last_modified;
                path = Some(entry.path());
            }
        }

        Ok(path)
    }
}
