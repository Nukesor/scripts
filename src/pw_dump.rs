//! Pipewire (pw_dump) specific datastructures.
use serde_derive::Deserialize;

/// Representation of a Pipewire device
#[derive(Debug, Deserialize)]
pub struct Device {
    pub id: usize,
    pub info: Option<Info>,
    #[serde(rename = "type")]
    pub device_type: String,
    pub version: usize,
    pub permissions: Vec<String>,
}

/// Detailed info about a device
#[derive(Debug, Deserialize)]
pub struct Info {
    pub props: Option<Props>,
    pub error: Option<String>,
    #[serde(rename = "max-input-ports")]
    pub max_input_ports: Option<usize>,
    #[serde(rename = "max-output-ports")]
    pub max_output_ports: Option<usize>,
    /// Looks like this:
    /// [ "input-ports", "output-ports", "state", "props", "params" ]
    #[serde(rename = "change-mask")]
    pub change_mask: Vec<String>,
    #[serde(rename = "n-input-ports")]
    pub n_input_ports: Option<usize>,
    #[serde(rename = "n-output-ports")]
    pub n_output_ports: Option<usize>,
    //pub state: "suspended",
}

#[derive(Debug, Deserialize)]
pub struct Props {
    #[serde(rename = "media.class")]
    pub media_class: Option<String>,
    #[serde(rename = "device.id")]
    pub device_id: Option<usize>,
    #[serde(rename = "node.description")]
    pub node_description: Option<String>,
    #[serde(rename = "object.serial")]
    pub object_serial: usize,
    #[serde(rename = "api.alsa.path")]
    pub api_alsa_path: Option<String>,
    #[serde(rename = "api.alsa.card")]
    pub api_alsa_card: Option<usize>,
    #[serde(rename = "api.alsa.card.name")]
    pub api_alsa_card_name: Option<String>,
    #[serde(rename = "api.alsa.card.longname")]
    pub api_alsa_card_longname: Option<String>,
    //#[serde(rename="object.path")]
    //object_path: "v4l2:/dev/video2",
    //#[serde(rename="device.api")]
    //device_api: "v4l2",
    //#[serde(rename="node.name")]
    //node_name: "v4l2_input_pci-0000_00_14_0-usb-0_8_1_0",
    //#[serde(rename="factory.name")]
    //factory_name: "api_v4l2_source",
    //#[serde(rename="node.pause-on-idle")]
    //node_pause-on-idle: false,
    //#[serde(rename="client.id")]
    //client_id: 32,
    //#[serde(rename="media.role")]
    //media_role: "Camera",
    //#[serde(rename="node.driver")]
    //node_driver: true,
    //#[serde(rename="object.id")]
    //object_id: 49,
}
