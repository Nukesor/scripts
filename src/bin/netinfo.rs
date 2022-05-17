#!/bin/python3
use anyhow::Result;
use regex::Regex;
use serde_derive::Deserialize;

use script_utils::exec::Cmd;

/// Main output for outpuft for
#[derive(Debug, Deserialize)]
struct Interface {
    pub ifname: String,
    pub addr_info: Vec<AddrInfo>,
    //    pub ifindex: usize,
    //    pub flags: Vec<String>,
    //    pub mtu: usize,
    //    pub qdisc: String,
    pub operstate: String,
    //    pub group: Option<String>,
    //    pub txqlen: usize,
    //    pub link_type: String,
    //    pub address: Option<String>,
    //    pub broadcast: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddrInfo {
    pub family: String,
    pub local: String,
    //    pub prefixlen: usize,
    //    pub metric: Option<usize>,
    //    pub broadcast: Option<String>,
    //    pub scope: String,
    //    pub dynamic: Option<bool>,
    //    pub label: Option<String>,
    //    pub valid_life_time: usize,
    //    pub preferred_life_time: usize,
}

enum NetworkType {
    Ethernet,
    Wlan,
    Vpn,
}

/// Print a string, representing the current network state with IP.
fn main() -> Result<()> {
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
            NetworkType::Ethernet => '',
            NetworkType::Wlan => '',
            NetworkType::Vpn => '',
        };

        let strength = match network_type {
            NetworkType::Ethernet => "",
            NetworkType::Wlan => wifi_strength(&name),
            NetworkType::Vpn => "",
        };

        output.push(format!("{symbol}{strength} {name}: {ip_addr}"));
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
        Cmd::new(format!("iwconfig {interface} | rg '^.*Signal level=.*'")).run_success();
    // Return an wifi error symbol if the signal strength cannot be determined.
    let capture_data = match capture_data {
        Ok(capture) => capture,
        Err(_) => return " ❌",
    };

    let re = Regex::new(r".*Signal level=-(\d*) dBm").unwrap();

    let output = String::from_utf8_lossy(&capture_data.stdout);
    let captures = match re.captures(&output) {
        Some(captures) => captures,
        None => return " ❌",
    };

    let level: usize = match captures.get(1).unwrap().as_str().parse() {
        Ok(level) => level,
        Err(_) => return " ❌",
    };

    match level {
        20..=60 => " ▇",
        61..=67 => " ▅",
        68..=70 => " ▃",
        71..=80 => " ▁",
        81..=90 => " !",
        _ => " ❌",
    }
}
