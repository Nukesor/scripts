use serde::Deserialize;

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
    pub params: Params,
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

#[derive(Debug, Deserialize, Clone)]
pub struct Params {
    #[serde(rename = "EnumProfile", default)]
    pub profiles: Vec<Profile>,
    #[serde(rename = "EnumRoute", default)]
    pub routes: Vec<Profile>,
}

/// A device can have multiple in-/outgoing routes.
/// Each has their own profile
///
/// This profile info contains some interesting data, such as, whether a cable is
/// plugged in or not.
#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    pub index: usize,
    pub name: String,
    pub description: String,
    // "yes"|"no"|"unknown"
    pub available: String,
}
