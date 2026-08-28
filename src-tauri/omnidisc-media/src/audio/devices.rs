use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioDevices {
    pub inputs: Vec<AudioDevice>,
    pub outputs: Vec<AudioDevice>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Input,
    Output,
}

fn describe(device: &cpal::Device, default_id: Option<&str>) -> Option<AudioDevice> {
    let id = device.id().ok()?.to_string();
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| id.clone());
    let default = default_id == Some(id.as_str());
    Some(AudioDevice { id, name, default })
}

pub fn enumerate() -> AudioDevices {
    let host = cpal::default_host();
    let default_in = host.default_input_device().and_then(|d| d.id().ok()).map(|i| i.to_string());
    let default_out = host.default_output_device().and_then(|d| d.id().ok()).map(|i| i.to_string());
    let inputs = host
        .input_devices()
        .map(|it| it.filter_map(|d| describe(&d, default_in.as_deref())).collect())
        .unwrap_or_default();
    let outputs = host
        .output_devices()
        .map(|it| it.filter_map(|d| describe(&d, default_out.as_deref())).collect())
        .unwrap_or_default();
    AudioDevices { inputs, outputs }
}

/// Is this exact device still listed by the OS? Used to tell "unplugged" from
/// "there but refusing to open".
pub fn exists(kind: DeviceKind, id: &str) -> bool {
    let host = cpal::default_host();
    let listed = match kind {
        DeviceKind::Input => host.input_devices().ok().map(|it| it.collect::<Vec<_>>()),
        DeviceKind::Output => host.output_devices().ok().map(|it| it.collect::<Vec<_>>()),
    };
    listed
        .map(|devices| devices.iter().any(|d| d.id().map(|i| i.to_string() == id).unwrap_or(false)))
        .unwrap_or(false)
}

pub fn find(kind: DeviceKind, id: Option<&str>) -> Option<cpal::Device> {
    let host = cpal::default_host();
    let wanted = id.map(str::trim).filter(|s| !s.is_empty());
    match (kind, wanted) {
        (DeviceKind::Input, None) => host.default_input_device(),
        (DeviceKind::Output, None) => host.default_output_device(),
        (DeviceKind::Input, Some(w)) => host
            .input_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.id().map(|i| i.to_string() == w).unwrap_or(false)))
            .or_else(|| host.default_input_device()),
        (DeviceKind::Output, Some(w)) => host
            .output_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.id().map(|i| i.to_string() == w).unwrap_or(false)))
            .or_else(|| host.default_output_device()),
    }
}
