use std::fs::{DirEntry, remove_file};

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct Entry {
    pub dir_entry: DirEntry,
    pub sidecars: Vec<DirEntry>,
}

impl Entry {
    /// Create a logical backup entry from a primary file and its sidecars.
    pub fn new(dir_entry: DirEntry, sidecars: Vec<DirEntry>) -> Self {
        Self {
            dir_entry,
            sidecars,
        }
    }

    /// Return the filenames that belong to this logical backup entry.
    pub fn filenames(&self) -> Vec<String> {
        let mut filenames = vec![self.dir_entry.file_name().to_string_lossy().to_string()];
        for sidecar in &self.sidecars {
            filenames.push(sidecar.file_name().to_string_lossy().to_string());
        }

        filenames
    }

    /// Remove the primary file and all sidecars that belong to this entry.
    pub fn remove_files(&self) -> Result<()> {
        remove_file(self.dir_entry.path()).context(format!(
            "Failed to remove file: {:?}",
            self.dir_entry.path()
        ))?;

        for sidecar in &self.sidecars {
            remove_file(sidecar.path())
                .context(format!("Failed to remove file: {:?}", sidecar.path()))?;
        }

        Ok(())
    }
}
