#!/bin/python3
use anyhow::Result;
use serde_derive::Deserialize;

use script_utils::process::Cmd;

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
        let symbol = if name.starts_with('e') {
            ""
        } else if name.starts_with('w') {
            ""
        } else {
            "🖧"
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
