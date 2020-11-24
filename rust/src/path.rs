use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use shellexpand::tilde;

pub fn path_exists<T: ToString>(path: T) -> bool {
    Path::new(&tilde(&path.to_string()).to_string()).exists()
}

pub fn get_newest_file(path: PathBuf) -> Result<Option<PathBuf>> {
    let dir = std::fs::read_dir(path)?;

    let mut path: Option<PathBuf> = None;
    let mut modified = SystemTime::UNIX_EPOCH;

    for entry_result in dir {
        let entry = entry_result?;
        let metadata = entry.metadata()?;

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
