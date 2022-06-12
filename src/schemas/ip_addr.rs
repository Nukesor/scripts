use serde_derive::Deserialize;

/// The entry struct for `ip -j addr` output.
#[derive(Debug, Deserialize)]
pub struct Interface {
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
pub struct AddrInfo {
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
