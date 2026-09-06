//! Serial device discovery and byte transport adapter.

#[cfg(windows)]
mod windows_discovery;

#[cfg(not(windows))]
use serialport::SerialPortType;
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::fmt;
use std::time::Duration;
use transport_api::{ByteTransport, TransportError};

pub const MONGOOSE_JLR_USB_VID: u16 = 0x18E1;
pub const MONGOOSE_JLR_USB_PID: u16 = 0x0104;
pub const MONGOOSE_JLR_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialDevice {
    pub port_name: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub usb_serial_number: Option<String>,
    pub driver_service: Option<String>,
}

#[derive(Debug)]
pub enum DiscoveryError {
    Enumeration(String),
    DeviceNotFound { vid: u16, pid: u16 },
    AmbiguousDevices(Vec<SerialDevice>),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enumeration(message) => write!(formatter, "serial enumeration failed: {message}"),
            Self::DeviceNotFound { vid, pid } => {
                write!(formatter, "device {vid:04X}:{pid:04X} was not found")
            }
            Self::AmbiguousDevices(devices) => write!(
                formatter,
                "multiple matching devices found: {}",
                devices
                    .iter()
                    .map(|device| device.port_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

pub trait SerialDeviceEnumerator {
    fn enumerate(&self) -> Result<Vec<SerialDevice>, DiscoveryError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemSerialDeviceEnumerator;

impl SerialDeviceEnumerator for SystemSerialDeviceEnumerator {
    fn enumerate(&self) -> Result<Vec<SerialDevice>, DiscoveryError> {
        #[cfg(windows)]
        {
            windows_discovery::enumerate()
        }
        #[cfg(not(windows))]
        {
            let ports = serialport::available_ports()
                .map_err(|error| DiscoveryError::Enumeration(error.to_string()))?;
            Ok(ports
                .into_iter()
                .filter_map(|port| match port.port_type {
                    SerialPortType::UsbPort(info) => Some(SerialDevice {
                        port_name: port.port_name,
                        usb_vid: info.vid,
                        usb_pid: info.pid,
                        usb_serial_number: info.serial_number,
                        driver_service: None,
                    }),
                    _ => None,
                })
                .collect())
        }
    }
}

pub fn matching_devices<E: SerialDeviceEnumerator>(
    enumerator: &E,
    vid: u16,
    pid: u16,
) -> Result<Vec<SerialDevice>, DiscoveryError> {
    Ok(enumerator
        .enumerate()?
        .into_iter()
        .filter(|device| device.usb_vid == vid && device.usb_pid == pid)
        .collect())
}

pub fn discover_unique<E: SerialDeviceEnumerator>(
    enumerator: &E,
    vid: u16,
    pid: u16,
) -> Result<SerialDevice, DiscoveryError> {
    let matches = matching_devices(enumerator, vid, pid)?;
    match matches.as_slice() {
        [] => Err(DiscoveryError::DeviceNotFound { vid, pid }),
        [device] => Ok(device.clone()),
        _ => Err(DiscoveryError::AmbiguousDevices(matches)),
    }
}

pub fn discover_mongoose_jlr() -> Result<SerialDevice, DiscoveryError> {
    discover_unique(
        &SystemSerialDeviceEnumerator,
        MONGOOSE_JLR_USB_VID,
        MONGOOSE_JLR_USB_PID,
    )
}

pub struct SerialTransport {
    port: Option<Box<dyn SerialPort>>,
}

impl SerialTransport {
    pub fn open(device: &SerialDevice) -> Result<Self, TransportError> {
        let port = serialport::new(&device.port_name, MONGOOSE_JLR_BAUD_RATE)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(DEFAULT_READ_TIMEOUT)
            .open()
            .map_err(map_serial_error)?;
        Ok(Self { port: Some(port) })
    }

    fn port(&mut self) -> Result<&mut (dyn SerialPort + 'static), TransportError> {
        self.port.as_deref_mut().ok_or(TransportError::Closed)
    }
}

impl ByteTransport for SerialTransport {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        std::io::Write::write_all(self.port()?, bytes).map_err(map_io_error)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        std::io::Read::read(self.port()?, buffer).map_err(map_io_error)
    }

    fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
        self.port()?.set_timeout(timeout).map_err(map_serial_error)
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.port.take();
        Ok(())
    }
}

fn map_serial_error(error: serialport::Error) -> TransportError {
    if error.kind() == serialport::ErrorKind::Io(std::io::ErrorKind::TimedOut) {
        TransportError::Timeout
    } else {
        TransportError::Io(error.to_string())
    }
}

fn map_io_error(error: std::io::Error) -> TransportError {
    if error.kind() == std::io::ErrorKind::TimedOut {
        TransportError::Timeout
    } else {
        TransportError::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedEnumerator(Vec<SerialDevice>);

    impl SerialDeviceEnumerator for FixedEnumerator {
        fn enumerate(&self) -> Result<Vec<SerialDevice>, DiscoveryError> {
            Ok(self.0.clone())
        }
    }

    fn device(port_name: &str, vid: u16, pid: u16) -> SerialDevice {
        SerialDevice {
            port_name: port_name.into(),
            usb_vid: vid,
            usb_pid: pid,
            usb_serial_number: None,
            driver_service: None,
        }
    }

    #[test]
    fn discovery_filters_by_vid_and_pid() {
        let enumerator = FixedEnumerator(vec![
            device("COM1", 0x1234, 0x5678),
            device("COM9", MONGOOSE_JLR_USB_VID, MONGOOSE_JLR_USB_PID),
        ]);
        assert_eq!(
            discover_unique(&enumerator, MONGOOSE_JLR_USB_VID, MONGOOSE_JLR_USB_PID)
                .unwrap()
                .port_name,
            "COM9"
        );
    }

    #[test]
    fn no_device_fails_closed() {
        let result = discover_unique(&FixedEnumerator(vec![]), 0x18E1, 0x0104);
        assert!(matches!(result, Err(DiscoveryError::DeviceNotFound { .. })));
    }

    #[test]
    fn ambiguity_fails_closed_and_preserves_candidates() {
        let result = discover_unique(
            &FixedEnumerator(vec![
                device("COM7", 0x18E1, 0x0104),
                device("COM8", 0x18E1, 0x0104),
            ]),
            0x18E1,
            0x0104,
        );
        match result {
            Err(DiscoveryError::AmbiguousDevices(devices)) => {
                assert_eq!(devices.len(), 2);
                assert_eq!(devices[0].port_name, "COM7");
                assert_eq!(devices[1].port_name, "COM8");
            }
            _ => panic!("expected ambiguity"),
        }
    }
}
