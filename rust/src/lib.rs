pub mod path;
pub mod process;

pub use anyhow::{anyhow, bail, Context, Result};
pub use path::{get_newest_file, path_exists, read_dir_or_fail, FileType};

/// Generic setup function that will be called in all scripts
pub fn setup() {
    // Beautify panics for better debug output.
    better_panic::install();
}
