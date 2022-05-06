use simplelog::{Config, LevelFilter, SimpleLogger};

/// Initialize the logger with the specified verbosity level.
pub fn init_logger(level: u8) {
    let level = match level {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };
    SimpleLogger::init(level, Config::default()).expect("Failed to init SimpleLogger");
}
