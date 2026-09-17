//! Input device listing.

use cpal::traits::{DeviceTrait, HostTrait};

pub struct InputDevice {
    pub name: String,
    pub device: cpal::Device,
}

pub fn list_input_devices() -> Result<Vec<InputDevice>, cpal::DevicesError> {
    Ok(cpal::default_host()
        .input_devices()?
        .map(|device| InputDevice {
            name: device.name().unwrap_or_else(|_| "Unknown".into()),
            device,
        })
        .collect())
}

/// The named device, or the system default when `name` is `None` or the
/// device is gone.
pub fn find_input_device(name: Option<&str>) -> Option<cpal::Device> {
    match name {
        Some(name) => list_input_devices()
            .ok()?
            .into_iter()
            .find(|d| d.name == name)
            .map(|d| d.device),
        None => cpal::default_host().default_input_device(),
    }
}
