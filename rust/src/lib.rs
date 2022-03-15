pub mod file;
pub mod path;
pub mod process;

pub use anyhow::{anyhow, bail, Context, Result};
pub use path::{get_newest_file, path_exists, read_dir_or_fail, FileType};

pub mod prelude {
    pub use super::file::*;
    pub use super::path::*;
    pub use super::process::*;
}

/// Generic setup function that will be called in all scripts
pub fn setup() {
    // Beautify panics for better debug output.
    better_panic::install();
}

#[macro_export]
macro_rules! unwrap_or_continue {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => {
                continue;
            }
        }
    };
}
