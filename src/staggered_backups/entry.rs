use std::fs::{DirEntry, remove_file};

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct Entry {
    pub dir_entry: DirEntry,
    pub sidecars: Vec<DirEntry>,
}

impl Entry {
    pub fn new(dir_entry: DirEntry, sidecars: Vec<DirEntry>) -> Self {
        Self {
            dir_entry,
            sidecars,
        }
    }

    pub fn filenames(&self) -> Vec<String> {
        let mut filenames = vec![self.dir_entry.file_name().to_string_lossy().to_string()];
        for sidecar in &self.sidecars {
            filenames.push(sidecar.file_name().to_string_lossy().to_string());
        }

        filenames
    }

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
