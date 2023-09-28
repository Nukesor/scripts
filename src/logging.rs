use simplelog::{Config, ConfigBuilder, LevelFilter, SimpleLogger};

/// Initialize the logger with the specified verbosity level.
pub fn init_logger(level: u8) {
    let level = match level {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    // Try to initialize the logger with the timezone set to the Local time of the machine.
    let mut builder = ConfigBuilder::new();
    let logger_config = match builder.set_time_offset_to_local() {
        Err(_) => {
            println!("Failed to determine the local time of this machine. Fallback to UTC.");
            Config::default()
        }
        Ok(builder) => builder.build(),
    };

    SimpleLogger::init(level, logger_config).expect("Failed to init SimpleLogger");
}
