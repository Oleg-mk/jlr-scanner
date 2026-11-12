//! The MongoosePro JLR on SDD's own driver (ADR-0041).
//!
//! SDD's MongoosePro package binds the cable to Drew Technologies' kernel
//! driver `dtmonpro.sys`, which creates no COM port, so the serial path
//! cannot see such a cable. The driver does expose a device interface, and
//! SDD's own library reads and writes the adapter's byte stream through it.
//! This crate does the same and nothing more: it finds that interface, opens
//! it, moves bytes, and calls none of the driver's IOCTLs. Nothing of the
//! package is loaded, shipped or changed, and no driver is ever rebound.
//!
//! The Windows calls live in `windows_interface`; everything here — the
//! identity parsing, the reader thread and the transport's timing — is
//! platform-independent and tested against a fake pipe.

#[cfg(windows)]
mod windows_interface;

use std::collections::VecDeque;
use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use transport_api::{ByteTransport, TransportError};

pub const MONGOOSE_JLR_USB_VID: u16 = 0x18E1;
pub const MONGOOSE_JLR_USB_PID: u16 = 0x0104;
/// The Windows service SDD's package installs for the cable; what the
/// adapter summary's `driver` field carries for a cable found this way.
pub const DRIVER_SERVICE: &str = "dtmonpro";
/// The device interface `dtmonpro.sys` creates for the cable, as recorded
/// in `docs/research/mongoose/MONGOOSE_USB_PROTOCOL.md`.
pub const DEVICE_INTERFACE_GUID: u128 = 0xF99F6FCE_03F4_4817_8A03_8CFDE4743B92;
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(250);
/// How long one write may take before it is cancelled and reported.
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
/// The reader's buffer, comfortably above any one transfer the adapter sends.
pub const READ_CHUNK: usize = 4096;
/// How long `close` waits for the reader to let go after cancelling its read.
const CLOSE_WAIT: Duration = Duration::from_secs(1);

/// A cable found on the package's driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassThruDevice {
    /// The device interface path the handle is opened on; opaque above here.
    pub device_path: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub usb_serial_number: Option<String>,
}

#[derive(Debug)]
pub enum DiscoveryError {
    Enumeration(String),
    DeviceNotFound { vid: u16, pid: u16 },
    AmbiguousDevices(Vec<PassThruDevice>),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enumeration(message) => {
                write!(formatter, "device interface enumeration failed: {message}")
            }
            Self::DeviceNotFound { vid, pid } => {
                write!(formatter, "device {vid:04X}:{pid:04X} was not found")
            }
            Self::AmbiguousDevices(devices) => write!(
                formatter,
                "multiple matching devices found: {}",
                devices
                    .iter()
                    .map(|device| device.device_path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// What a Windows device instance id says about a USB device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsbIdentity {
    pub vid: u16,
    pub pid: u16,
    /// The instance name when it is the device's own serial number; a
    /// Windows-made name (`6&ABC&0&1`, for a device without one) is not.
    pub serial_number: Option<String>,
}

/// `USB\VID_18E1&PID_0104\AOLHE0000003666A` and the like. The identity comes
/// from here and never from the interface GUID alone: the GUID says where to
/// look, the instance id says what was found.
pub fn parse_instance_id(instance_id: &str) -> Option<UsbIdentity> {
    let mut parts = instance_id.split('\\');
    let enumerator = parts.next()?;
    if !enumerator.eq_ignore_ascii_case("USB") {
        return None;
    }
    let hardware = parts.next()?.to_ascii_uppercase();
    let instance = parts.next().unwrap_or("");
    let vid_index = hardware.find("VID_")? + 4;
    let pid_index = hardware.find("PID_")? + 4;
    let vid = u16::from_str_radix(hardware.get(vid_index..vid_index + 4)?, 16).ok()?;
    let pid = u16::from_str_radix(hardware.get(pid_index..pid_index + 4)?, 16).ok()?;
    let serial_number = if instance.is_empty() || instance.contains('&') {
        None
    } else {
        Some(instance.to_owned())
    };
    Some(UsbIdentity {
        vid,
        pid,
        serial_number,
    })
}

pub trait PassThruEnumerator {
    fn enumerate(&self) -> Result<Vec<PassThruDevice>, DiscoveryError>;
}

/// The cables the installed `dtmonpro.sys` exposes right now. On anything
/// but Windows there is no such driver and the list is empty.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemPassThruEnumerator;

impl PassThruEnumerator for SystemPassThruEnumerator {
    fn enumerate(&self) -> Result<Vec<PassThruDevice>, DiscoveryError> {
        #[cfg(windows)]
        {
            windows_interface::enumerate()
        }
        #[cfg(not(windows))]
        {
            Ok(Vec::new())
        }
    }
}

pub fn matching_devices<E: PassThruEnumerator>(
    enumerator: &E,
    vid: u16,
    pid: u16,
) -> Result<Vec<PassThruDevice>, DiscoveryError> {
    Ok(enumerator
        .enumerate()?
        .into_iter()
        .filter(|device| device.usb_vid == vid && device.usb_pid == pid)
        .collect())
}

/// Every device of one vendor on this driver, whatever its product: named to
/// the person rather than passed over, never opened.
pub fn vendor_devices<E: PassThruEnumerator>(
    enumerator: &E,
    vid: u16,
) -> Result<Vec<PassThruDevice>, DiscoveryError> {
    Ok(enumerator
        .enumerate()?
        .into_iter()
        .filter(|device| device.usb_vid == vid)
        .collect())
}

pub fn discover_unique<E: PassThruEnumerator>(
    enumerator: &E,
    vid: u16,
    pid: u16,
) -> Result<PassThruDevice, DiscoveryError> {
    let matches = matching_devices(enumerator, vid, pid)?;
    match matches.as_slice() {
        [] => Err(DiscoveryError::DeviceNotFound { vid, pid }),
        [device] => Ok(device.clone()),
        _ => Err(DiscoveryError::AmbiguousDevices(matches)),
    }
}

pub fn discover_mongoose_jlr() -> Result<PassThruDevice, DiscoveryError> {
    discover_unique(
        &SystemPassThruEnumerator,
        MONGOOSE_JLR_USB_VID,
        MONGOOSE_JLR_USB_PID,
    )
}

/// Why a transfer on the pipe did not deliver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipeError {
    /// The transfer was cancelled: the transport is closing.
    Cancelled,
    /// A write did not complete within `WRITE_TIMEOUT`.
    Timeout,
    /// The device went away, or the driver refused the transfer.
    Io(String),
}

/// One open handle on the device interface, used from two threads at once:
/// the reader's blocking read and the caller's writes. The Windows pipe is
/// in `windows_interface`; tests bring their own.
pub trait Pipe: Send + Sync {
    /// Blocks until the device sends something, the pipe is cancelled, or
    /// the transfer fails. A zero-length completion is a transfer that
    /// carried nothing, not the end of the stream.
    fn read(&self, buffer: &mut [u8]) -> Result<usize, PipeError>;
    fn write(&self, bytes: &[u8]) -> Result<(), PipeError>;
    /// Cancels the pending read (and a pending write, if any); the reader
    /// then gets `Cancelled` and stops.
    fn cancel(&self);
}

/// The adapter's byte stream through SDD's driver, as `ByteTransport`.
///
/// A reader thread owns the one pending read — the driver's read queue is
/// sequential — and hands each completion to a channel; `read` takes from
/// the channel within the read timeout and keeps what the caller's buffer
/// did not take. Writes go straight to the pipe from the caller's thread.
pub struct DtmonproTransport {
    pipe: Option<Arc<dyn Pipe>>,
    incoming: Receiver<Result<Vec<u8>, PipeError>>,
    pending: VecDeque<u8>,
    reader: Option<JoinHandle<()>>,
    read_timeout: Duration,
}

impl DtmonproTransport {
    /// Opens the cable on the package's driver. Fails with the adapter "in
    /// use" when another program — SDD — holds it.
    #[cfg(windows)]
    pub fn open(device: &PassThruDevice) -> Result<Self, TransportError> {
        let pipe =
            windows_interface::OverlappedPipe::open(&device.device_path).map_err(map_pipe_error)?;
        Ok(Self::over(Arc::new(pipe)))
    }

    #[cfg(not(windows))]
    pub fn open(_device: &PassThruDevice) -> Result<Self, TransportError> {
        Err(TransportError::Io(
            "the MongoosePro driver's device interface exists on Windows only".to_owned(),
        ))
    }

    /// A transport over any pipe: the Windows one, or a fake under test.
    pub fn over(pipe: Arc<dyn Pipe>) -> Self {
        let (sender, receiver) = mpsc::sync_channel(256);
        let reader = spawn_reader(Arc::clone(&pipe), sender);
        Self {
            pipe: Some(pipe),
            incoming: receiver,
            pending: VecDeque::new(),
            reader: Some(reader),
            read_timeout: DEFAULT_READ_TIMEOUT,
        }
    }

    fn take_pending(&mut self, buffer: &mut [u8]) -> usize {
        let count = buffer.len().min(self.pending.len());
        for slot in buffer.iter_mut().take(count) {
            *slot = self.pending.pop_front().unwrap_or_default();
        }
        count
    }
}

fn spawn_reader(
    pipe: Arc<dyn Pipe>,
    sender: SyncSender<Result<Vec<u8>, PipeError>>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("dtmonpro-reader".to_owned())
        .spawn(move || {
            let mut chunk = vec![0_u8; READ_CHUNK];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => continue,
                    Ok(read) => {
                        if sender.send(Ok(chunk[..read].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        })
        .expect("the dtmonpro reader thread could not be started")
}

fn map_pipe_error(error: PipeError) -> TransportError {
    match error {
        PipeError::Timeout => TransportError::Timeout,
        PipeError::Cancelled => TransportError::Closed,
        PipeError::Io(message) => TransportError::Io(message),
    }
}

impl ByteTransport for DtmonproTransport {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        let pipe = self.pipe.as_ref().ok_or(TransportError::Closed)?;
        pipe.write(bytes).map_err(map_pipe_error)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        if self.pipe.is_none() {
            return Err(TransportError::Closed);
        }
        if buffer.is_empty() {
            return Ok(0);
        }
        if !self.pending.is_empty() {
            return Ok(self.take_pending(buffer));
        }
        match self.incoming.recv_timeout(self.read_timeout) {
            Ok(Ok(chunk)) => {
                self.pending.extend(chunk);
                Ok(self.take_pending(buffer))
            }
            Ok(Err(PipeError::Cancelled)) | Err(RecvTimeoutError::Disconnected) => {
                Err(TransportError::Closed)
            }
            Ok(Err(error)) => Err(map_pipe_error(error)),
            Err(RecvTimeoutError::Timeout) => Err(TransportError::Timeout),
        }
    }

    fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
        self.read_timeout = timeout;
        Ok(())
    }

    /// Cancels the pending read, waits for the reader to let go, and drops
    /// the pipe; the handle closes with the last reference to it. A reader
    /// that does not answer the cancellation within `CLOSE_WAIT` is left to
    /// finish on its own rather than hang the caller.
    fn close(&mut self) -> Result<(), TransportError> {
        let Some(pipe) = self.pipe.take() else {
            return Ok(());
        };
        pipe.cancel();
        let deadline = Instant::now() + CLOSE_WAIT;
        let reader_finished = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.incoming.recv_timeout(remaining) {
                Ok(_) => continue,
                Err(RecvTimeoutError::Disconnected) => break true,
                Err(RecvTimeoutError::Timeout) => break false,
            }
        };
        if reader_finished {
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
        self.pending.clear();
        drop(pipe);
        Ok(())
    }
}

impl Drop for DtmonproTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Condvar, Mutex};

    #[test]
    fn the_instance_id_names_the_device_and_its_serial() {
        assert_eq!(
            parse_instance_id(r"USB\VID_18E1&PID_0104\AOLHE0000003666A"),
            Some(UsbIdentity {
                vid: 0x18E1,
                pid: 0x0104,
                serial_number: Some("AOLHE0000003666A".to_owned()),
            })
        );
        assert_eq!(
            parse_instance_id(r"usb\vid_18e1&pid_0104\aolhe0000003666a"),
            Some(UsbIdentity {
                vid: 0x18E1,
                pid: 0x0104,
                serial_number: Some("aolhe0000003666a".to_owned()),
            })
        );
    }

    #[test]
    fn a_windows_made_instance_name_is_not_a_serial_number() {
        assert_eq!(
            parse_instance_id(r"USB\VID_18E1&PID_0104\6&ABC&0&1"),
            Some(UsbIdentity {
                vid: 0x18E1,
                pid: 0x0104,
                serial_number: None,
            })
        );
    }

    #[test]
    fn anything_but_a_usb_instance_id_is_not_an_identity() {
        assert_eq!(
            parse_instance_id(r"PCI\VEN_8086&DEV_1234\3&11583659&0&E8"),
            None
        );
        assert_eq!(parse_instance_id(r"USB\ROOT_HUB30\4&2A3B&0"), None);
        assert_eq!(parse_instance_id(r"USB\VID_18E1\X"), None);
        assert_eq!(parse_instance_id(""), None);
    }

    struct FixedEnumerator(Vec<PassThruDevice>);

    impl PassThruEnumerator for FixedEnumerator {
        fn enumerate(&self) -> Result<Vec<PassThruDevice>, DiscoveryError> {
            Ok(self.0.clone())
        }
    }

    fn device(path: &str, vid: u16, pid: u16) -> PassThruDevice {
        PassThruDevice {
            device_path: path.to_owned(),
            usb_vid: vid,
            usb_pid: pid,
            usb_serial_number: Some("AOLHE0000003666A".to_owned()),
        }
    }

    #[test]
    fn discovery_is_exact_about_the_product() {
        let enumerator = FixedEnumerator(vec![
            device(r"\\?\usb#vid_18e1&pid_0022#x#{guid}", 0x18E1, 0x0022),
            device(r"\\?\usb#vid_18e1&pid_0104#y#{guid}", 0x18E1, 0x0104),
        ]);
        let found =
            discover_unique(&enumerator, MONGOOSE_JLR_USB_VID, MONGOOSE_JLR_USB_PID).unwrap();
        assert_eq!(found.device_path, r"\\?\usb#vid_18e1&pid_0104#y#{guid}");
        assert_eq!(
            vendor_devices(&enumerator, MONGOOSE_JLR_USB_VID)
                .unwrap()
                .len(),
            2
        );
        assert!(matches!(
            discover_unique(
                &FixedEnumerator(Vec::new()),
                MONGOOSE_JLR_USB_VID,
                MONGOOSE_JLR_USB_PID
            ),
            Err(DiscoveryError::DeviceNotFound {
                vid: 0x18E1,
                pid: 0x0104
            })
        ));
        let two = FixedEnumerator(vec![
            device(r"\\?\a", 0x18E1, 0x0104),
            device(r"\\?\b", 0x18E1, 0x0104),
        ]);
        assert!(matches!(
            discover_unique(&two, MONGOOSE_JLR_USB_VID, MONGOOSE_JLR_USB_PID),
            Err(DiscoveryError::AmbiguousDevices(devices)) if devices.len() == 2
        ));
    }

    /// A pipe that plays scripted completions and then, like a real pending
    /// read, blocks until it is cancelled.
    #[derive(Default)]
    struct FakePipe {
        reads: Mutex<VecDeque<Result<Vec<u8>, PipeError>>>,
        written: Mutex<Vec<Vec<u8>>>,
        cancelled: AtomicBool,
        wake: Condvar,
        gate: Mutex<()>,
    }

    impl FakePipe {
        fn scripted(reads: Vec<Result<Vec<u8>, PipeError>>) -> Arc<Self> {
            Arc::new(Self {
                reads: Mutex::new(reads.into()),
                ..Self::default()
            })
        }
    }

    impl Pipe for FakePipe {
        fn read(&self, buffer: &mut [u8]) -> Result<usize, PipeError> {
            if let Some(next) = self.reads.lock().unwrap().pop_front() {
                return next.map(|bytes| {
                    let count = bytes.len().min(buffer.len());
                    buffer[..count].copy_from_slice(&bytes[..count]);
                    count
                });
            }
            let mut gate = self.gate.lock().unwrap();
            while !self.cancelled.load(Ordering::SeqCst) {
                gate = self.wake.wait(gate).unwrap();
            }
            Err(PipeError::Cancelled)
        }

        fn write(&self, bytes: &[u8]) -> Result<(), PipeError> {
            self.written.lock().unwrap().push(bytes.to_vec());
            Ok(())
        }

        fn cancel(&self) {
            let _gate = self.gate.lock().unwrap();
            self.cancelled.store(true, Ordering::SeqCst);
            self.wake.notify_all();
        }
    }

    #[test]
    fn bytes_arrive_in_order_and_what_a_small_buffer_leaves_waits_for_the_next_read() {
        let pipe = FakePipe::scripted(vec![Ok(b"abc".to_vec()), Ok(b"defgh".to_vec())]);
        let mut transport = DtmonproTransport::over(pipe);
        transport.set_read_timeout(Duration::from_secs(2)).unwrap();
        let mut buffer = [0_u8; 4];
        let read = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..read], b"abc");
        let read = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..read], b"defg");
        let read = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..read], b"h");
    }

    #[test]
    fn silence_is_a_timeout_not_an_error() {
        let pipe = FakePipe::scripted(Vec::new());
        let mut transport = DtmonproTransport::over(pipe);
        transport
            .set_read_timeout(Duration::from_millis(20))
            .unwrap();
        let mut buffer = [0_u8; 8];
        assert_eq!(transport.read(&mut buffer), Err(TransportError::Timeout));
        assert_eq!(transport.read(&mut buffer), Err(TransportError::Timeout));
    }

    #[test]
    fn a_device_that_goes_away_is_reported_once_and_then_the_transport_is_closed() {
        let pipe = FakePipe::scripted(vec![Err(PipeError::Io("device not connected".to_owned()))]);
        let mut transport = DtmonproTransport::over(pipe);
        transport.set_read_timeout(Duration::from_secs(2)).unwrap();
        let mut buffer = [0_u8; 8];
        assert_eq!(
            transport.read(&mut buffer),
            Err(TransportError::Io("device not connected".to_owned()))
        );
        assert_eq!(transport.read(&mut buffer), Err(TransportError::Closed));
    }

    #[test]
    fn writes_go_straight_through_and_close_cancels_the_pending_read() {
        let pipe = FakePipe::scripted(Vec::new());
        let mut transport = DtmonproTransport::over(Arc::clone(&pipe) as Arc<dyn Pipe>);
        transport.write_all(&[0x51, 0xE6, 0x01]).unwrap();
        assert_eq!(
            pipe.written.lock().unwrap().as_slice(),
            &[vec![0x51, 0xE6, 0x01]]
        );
        transport.close().unwrap();
        assert!(pipe.cancelled.load(Ordering::SeqCst));
        assert!(
            transport.reader.is_none(),
            "the reader answered the cancellation and was joined"
        );
        let mut buffer = [0_u8; 8];
        assert_eq!(transport.read(&mut buffer), Err(TransportError::Closed));
        assert_eq!(transport.write_all(&[1]), Err(TransportError::Closed));
        assert_eq!(transport.close(), Ok(()));
    }

    #[test]
    fn bytes_already_received_are_still_read_after_the_device_is_gone() {
        let pipe = FakePipe::scripted(vec![
            Ok(b"xyz".to_vec()),
            Err(PipeError::Io("gone".to_owned())),
        ]);
        let mut transport = DtmonproTransport::over(pipe);
        transport.set_read_timeout(Duration::from_secs(2)).unwrap();
        let mut buffer = [0_u8; 2];
        let read = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..read], b"xy");
        let read = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..read], b"z");
        assert_eq!(
            transport.read(&mut buffer),
            Err(TransportError::Io("gone".to_owned()))
        );
    }
}
