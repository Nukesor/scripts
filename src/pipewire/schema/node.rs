use serde_derive::Deserialize;

/// Representation of a Pipewire device
#[derive(Debug, Deserialize, Clone)]
pub struct Node {
    pub id: usize,
    #[serde(rename = "type")]
    pub node_type: String,
    pub info: NodeInfo,
}

/// Detailed info about a device
#[derive(Debug, Deserialize, Clone)]
pub struct NodeInfo {
    pub props: NodeProps,
    pub state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeProps {
    /// Info about the parent device this node belongs to
    #[serde(rename = "device.id")]
    pub device_id: usize,
    #[serde(rename = "device.api")]
    pub device_api: String,
    #[serde(rename = "device.class")]
    pub device_class: String,

    /// Info about the device profile this node belongs to
    #[serde(rename = "device.profile.description")]
    pub device_profile_description: String,
    #[serde(rename = "device.profile.name")]
    pub device_profile_name: String,

    /// Info about this very node
    #[serde(rename = "node.name")]
    pub node_name: String,
    #[serde(rename = "node.description")]
    pub node_description: String,
    #[serde(rename = "node.nick")]
    pub node_nick: String,

    /// The object properties of this node.
    #[serde(rename = "object.id")]
    pub object_id: usize,
    #[serde(rename = "object.path")]
    pub object_path: String,
    #[serde(rename = "object.serial")]
    pub object_serial: usize,

    /// The media info of this node
    #[serde(rename = "media.class")]
    pub media_class: String,

    /// The client this node belongs to
    #[serde(rename = "client.id")]
    pub client_id: usize,
}
