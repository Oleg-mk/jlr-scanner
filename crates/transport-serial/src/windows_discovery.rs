use crate::{DiscoveryError, SerialDevice};
use std::collections::HashSet;
use windows::core::HSTRING;
use windows::Win32::Devices::Communication::GetCommPorts;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Locate_DevNodeW, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

const USB_ENUM_PATH: &str = r"SYSTEM\CurrentControlSet\Enum\USB";

pub(super) fn enumerate() -> Result<Vec<SerialDevice>, DiscoveryError> {
    let present_ports = present_port_names()?;
    let registry = RegKey::predef(HKEY_LOCAL_MACHINE);
    let usb = registry
        .open_subkey(USB_ENUM_PATH)
        .map_err(|error| DiscoveryError::Enumeration(error.to_string()))?;
    let mut devices = Vec::new();

    for hardware_key_name in usb.enum_keys().filter_map(Result::ok) {
        let Some((vid, pid)) = parse_usb_identity(&hardware_key_name) else {
            continue;
        };
        let Ok(hardware_key) = usb.open_subkey(&hardware_key_name) else {
            continue;
        };
        for instance_name in hardware_key.enum_keys().filter_map(Result::ok) {
            let device_id = device_instance_id(&hardware_key_name, &instance_name);
            if !is_present(&device_id) {
                continue;
            }
            let Ok(instance) = hardware_key.open_subkey(&instance_name) else {
                continue;
            };
            let Ok(parameters) = instance.open_subkey("Device Parameters") else {
                continue;
            };
            let Ok(port_name) = parameters.get_value::<String, _>("PortName") else {
                continue;
            };
            if !present_ports.contains(&port_name.to_ascii_uppercase()) {
                continue;
            }
            devices.push(SerialDevice {
                port_name,
                usb_vid: vid,
                usb_pid: pid,
                usb_serial_number: serial_number(&instance_name),
                driver_service: instance.get_value::<String, _>("Service").ok(),
            });
        }
    }
    devices.sort_by(|left, right| left.port_name.cmp(&right.port_name));
    devices.dedup();
    Ok(devices)
}

fn device_instance_id(hardware_key_name: &str, instance_name: &str) -> String {
    format!(r"USB\{hardware_key_name}\{instance_name}")
}

fn is_present(device_instance_id: &str) -> bool {
    let device_id = HSTRING::from(device_instance_id);
    let mut device_node = 0_u32;
    // SAFETY: device_node is a valid output pointer and the HSTRING is NUL-terminated.
    unsafe {
        CM_Locate_DevNodeW(&mut device_node, &device_id, CM_LOCATE_DEVNODE_NORMAL) == CR_SUCCESS
    }
}
fn present_port_names() -> Result<HashSet<String>, DiscoveryError> {
    let mut port_numbers = [0_u32; 256];
    let mut found = 0_u32;
    // SAFETY: GetCommPorts receives a valid mutable slice and count output.
    let status = unsafe { GetCommPorts(&mut port_numbers, &mut found) };
    if status != 0 {
        return Err(DiscoveryError::Enumeration(format!(
            "GetCommPorts failed with Windows error {status}"
        )));
    }
    Ok(
        port_numbers[..usize::try_from(found).unwrap_or(port_numbers.len())]
            .iter()
            .map(|port| format!("COM{port}"))
            .collect(),
    )
}

fn parse_usb_identity(hardware_key_name: &str) -> Option<(u16, u16)> {
    let uppercase = hardware_key_name.to_ascii_uppercase();
    let vid_index = uppercase.find("VID_")? + 4;
    let pid_index = uppercase.find("PID_")? + 4;
    let vid = u16::from_str_radix(uppercase.get(vid_index..vid_index + 4)?, 16).ok()?;
    let pid = u16::from_str_radix(uppercase.get(pid_index..pid_index + 4)?, 16).ok()?;
    Some((vid, pid))
}

fn serial_number(instance_name: &str) -> Option<String> {
    if instance_name.is_empty() || instance_name.contains('&') {
        None
    } else {
        Some(instance_name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitive_usb_identity() {
        assert_eq!(
            parse_usb_identity("Vid_18e1&Pid_0104"),
            Some((0x18E1, 0x0104))
        );
        assert_eq!(
            parse_usb_identity("VID_18E1&PID_0104&MI_00"),
            Some((0x18E1, 0x0104))
        );
    }

    #[test]
    fn only_stable_instance_ids_are_reported_as_serial_numbers() {
        assert_eq!(
            serial_number("AOLHE00000000001"),
            Some("AOLHE00000000001".into())
        );
        assert_eq!(serial_number("6&ABC&0&1"), None);
    }
}

#[test]
fn builds_windows_device_instance_id() {
    assert_eq!(
        device_instance_id("VID_18E1&PID_0104", "AOLHE00000000001"),
        r"USB\VID_18E1&PID_0104\AOLHE00000000001"
    );
}
