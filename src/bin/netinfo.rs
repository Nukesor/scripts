//! A tool for use in use with status bars.
//!
//! It displays information about currently available network devices such as:
//! - IP Address
//! - Type
//! - Signal strength
use anyhow::Result;
use clap::{ArgAction, Parser};
use log::{debug, warn};
use regex::Regex;

use script_utils::exec::Cmd;
use script_utils::logging;
use script_utils::schemas::ip_addr::*;

enum NetworkType {
    Ethernet,
    Wlan,
    Vpn,
}

#[derive(Parser, Debug)]
#[clap(
    name = "netinfo",
    about = "Get network info, formatted for a status bar",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

/// Print a string, representing the current network state with IP.
fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    let capture = Cmd::new("ip -j addr").run_success()?;
    let interfaces: Vec<Interface> = serde_json::from_str(&capture.stdout_str())?;

    let mut output = Vec::new();

    for interface in interfaces {
        // We aren't interested in the loopback device
        if interface.ifname == "lo" {
            continue;
        }

        // Device doesn't have an active connection.
        if interface.addr_info.is_empty() || interface.operstate == "DOWN" {
            continue;
        }

        debug!("Interface info: {interface:#?}");

        // Try to find an ipv4 address by default.
        let addr = interface
            .addr_info
            .iter()
            .find(|addr| addr.family == "inet");

        // Search for an ipv6 address as fallback.
        let addr = if let Some(addr) = addr {
            addr
        } else {
            let ipv6_addr = interface
                .addr_info
                .iter()
                .find(|addr| addr.family == "inet6");

            match ipv6_addr {
                Some(addr) => addr,
                None => continue,
            }
        };

        let name = interface.ifname;
        let ip_addr = &addr.local;

        // Drop any container/virtual environment related networks
        if name.starts_with("docker") || name.starts_with("veth") || name.starts_with("br") {
            continue;
        }

        // Determine the network type based on the name of the interface.
        let network_type = if name.starts_with('e') {
            NetworkType::Ethernet
        } else if name.starts_with('w') {
            NetworkType::Wlan
        } else {
            NetworkType::Vpn
        };

        // Set the symbol for the current network type.
        let symbol = match network_type {
            NetworkType::Ethernet => "".into(),
            NetworkType::Wlan => format!(" {}", wifi_strength(&name)),
            NetworkType::Vpn => "".into(),
        };

        output.push(format!("{symbol} {name}: {ip_addr}"));
    }

    if output.is_empty() {
        println!("No network");
    } else {
        println!("{}", output.join(", "));
    }

    Ok(())
}

/// Determine the network strength of a given device.
/// -30 dBm Maximum signal strength, you are probably standing right next to the access point / router.
/// -50 dBm Anything down to this level can be regarded as excellent signal strength.
/// -60 dBm This is still good, reliable signal strength.
/// -67 dBm This is the minimum value for all services that require smooth and reliable data traffic.
///  VoIP/VoWi-Fi Video streaming/streaming (not the highest quality)
/// -70 dBm The signal is not very strong, but mostly sufficient. Web, email, and the like
/// -80 dBm Minimum value required to make a connection.
///  You cannot count on a reliable connection or sufficient signal strength to use services at this level.
/// -90 dBm It is very unlikely that you will be able to connect or make use of any services with this signal strength.
pub fn wifi_strength(interface: &str) -> &'static str {
    let capture_data =
        Cmd::new(format!("iw dev {interface} info | rg '^.*txpower.*'")).run_success();
    // Return an wifi error symbol if the signal strength cannot be determined.
    let capture_data = match capture_data {
        Ok(capture) => capture,
        Err(err) => {
            warn!("Got error reading interface info: {err:#?}");
            return "";
        }
    };

    let re = Regex::new(r"txpower (\d*)\.\d* dBm").unwrap();

    let output = String::from_utf8_lossy(&capture_data.stdout);

    debug!("Iw output: {output:#?}");
    let captures = match re.captures(output.trim()) {
        Some(captures) => captures,
        None => return "",
    };

    let level: usize = match captures.get(1).unwrap().as_str().parse() {
        Ok(level) => level,
        Err(_) => return "",
    };

    match level {
        10..=30 => "▇",
        51..=67 => "▅",
        68..=70 => "▃",
        71..=80 => "▁",
        81..=90 => "!",
        _ => "!",
    }
}
