use serde::{de, Deserialize, Deserializer};
use serde_derive::Deserialize;

/// Representation of a Pipewire device
#[derive(Debug, Deserialize, Clone)]
pub struct Device {
    pub id: usize,
    pub info: DeviceInfo,
    #[serde(rename = "type")]
    pub device_type: String,
}

/// Detailed info about a device
#[derive(Debug, Deserialize, Clone)]
pub struct DeviceInfo {
    pub props: DeviceProps,
    #[serde(rename = "params", deserialize_with = "extract_routes")]
    pub routes: Vec<Route>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceProps {
    #[serde(rename = "device.api")]
    pub device_api: String,
    #[serde(rename = "device.description")]
    pub device_description: String,
    #[serde(rename = "device.name")]
    pub device_name: String,

    /// The object properties of this device.
    #[serde(rename = "object.id")]
    pub object_id: usize,
    #[serde(rename = "object.path")]
    pub object_path: Option<String>,
    #[serde(rename = "object.serial")]
    pub object_serial: usize,

    /// The media info of this node
    #[serde(rename = "media.class")]
    pub media_class: String,

    /// The client this device belongs to
    #[serde(rename = "client.id")]
    pub client_id: usize,
}

/// A device can have multiple in-/outgoing routes.
/// Each has their own profile
///
/// This profile info contains some interesting data, such as, whether a cable is
/// plugged in or not.
#[derive(Debug, Deserialize, Clone)]
pub struct Route {
    pub index: usize,
    pub direction: String,
    pub name: String,
    pub description: String,
    // "yes"|"no"|"unknown"
    pub available: String,
}

/// The routes are deep inside a bunch of other irrelevant info.
/// To be specific in `info.params["Routes"]`
/// Since we're not interested in the rest of `params`, we just extract that single object from that
/// list of object.
fn extract_routes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Route>, D::Error> {
    let map = serde_json::map::Map::deserialize(deserializer)?;

    let Some(routes) = map.get("EnumRoute") else {
        return Ok(Vec::new());
    };

    serde_json::from_value(routes.clone()).map_err(de::Error::custom)
}
