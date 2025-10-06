pub mod exec;
pub mod fs;
pub mod i3status;
pub mod ip_addr;
pub mod logging;
pub mod notify;
pub mod pipewire;
pub mod process;
pub mod ring;
pub mod table;
pub mod timer;

pub use anyhow::{Context, Result, anyhow, bail};
pub use fs::{FileType, get_newest_file, path_exists, read_dir_or_fail};

pub mod prelude {
    pub use super::{exec::*, fs::*};
}

/// Generic setup function that will be called in all scripts
pub fn setup() {
    // Beautify panics for better debug output.
    better_panic::install();
}

pub fn sleep_seconds(seconds: u64) {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}

#[macro_export]
macro_rules! some_or_continue {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => {
                continue;
            }
        }
    };
}
