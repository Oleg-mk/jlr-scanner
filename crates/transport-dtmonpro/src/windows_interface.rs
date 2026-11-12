//! The Windows side of ADR-0041: the device interfaces `dtmonpro.sys` exposes,
//! and one overlapped handle on one of them. SetupAPI for the enumeration,
//! `CreateFileW`, `ReadFile` and `WriteFile` for the bytes — the calls SDD's
//! own library makes — and none of the driver's IOCTLs.

use crate::{
    parse_instance_id, DiscoveryError, PassThruDevice, Pipe, PipeError, DEVICE_INTERFACE_GUID,
    DRIVER_SERVICE, WRITE_TIMEOUT,
};
use std::mem::size_of;
use std::ptr;
use windows::core::{Error, GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW,
    SetupDiGetDeviceRegistryPropertyW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
    SPDRP_SERVICE, SPINT_ACTIVE, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
    SP_DEVINFO_DATA,
};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_IO_PENDING,
    ERROR_NO_MORE_ITEMS, ERROR_OPERATION_ABORTED, ERROR_SHARING_VIOLATION, GENERIC_READ,
    GENERIC_WRITE, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

fn is_win32(error: &Error, code: WIN32_ERROR) -> bool {
    error.code() == code.to_hresult()
}

fn enumeration_error(what: &str, error: Error) -> DiscoveryError {
    DiscoveryError::Enumeration(format!("{what}: {error}"))
}

/// Every present device interface of the driver's GUID whose instance id
/// names a USB device, sorted by path. Only the instance id is trusted for
/// what the device is, and only an interface that is active on a device
/// the `dtmonpro` service holds right now is listed: on the owner's PC a
/// registration the driver had left behind outlived the driver, its path
/// opened the cable through Windows's serial driver instead, and the panel
/// said "on SDD's driver" of a cable that was not (2026-11-12).
pub(crate) fn enumerate() -> Result<Vec<PassThruDevice>, DiscoveryError> {
    let guid = GUID::from_u128(DEVICE_INTERFACE_GUID);
    // SAFETY: the GUID outlives the call and the flags are the documented ones.
    let set = unsafe {
        SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }
    .map_err(|error| enumeration_error("SetupDiGetClassDevsW", error))?;
    let result = enumerate_in(set, &guid);
    // SAFETY: the set came from SetupDiGetClassDevsW and is destroyed once.
    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(set);
    }
    result
}

fn enumerate_in(set: HDEVINFO, guid: &GUID) -> Result<Vec<PassThruDevice>, DiscoveryError> {
    let mut devices = Vec::new();
    for index in 0_u32.. {
        // SAFETY: zeroed is a valid state for these plain C structures.
        let mut interface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
        interface.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        // SAFETY: set, guid and interface are valid for the call.
        match unsafe { SetupDiEnumDeviceInterfaces(set, None, guid, index, &mut interface) } {
            Ok(()) => {}
            Err(error) if is_win32(&error, ERROR_NO_MORE_ITEMS) => break,
            Err(error) => return Err(enumeration_error("SetupDiEnumDeviceInterfaces", error)),
        }
        // A registration without a live symbolic link is a leftover, not a cable.
        if interface.Flags & SPINT_ACTIVE == 0 {
            continue;
        }

        let mut required = 0_u32;
        // SAFETY: a size query; a null detail buffer with size 0 is the documented way.
        match unsafe {
            SetupDiGetDeviceInterfaceDetailW(set, &interface, None, 0, Some(&mut required), None)
        } {
            Err(error) if is_win32(&error, ERROR_INSUFFICIENT_BUFFER) => {}
            Ok(()) => {}
            Err(error) => {
                return Err(enumeration_error(
                    "SetupDiGetDeviceInterfaceDetailW (size)",
                    error,
                ))
            }
        }
        let header = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        let bytes = (required as usize).max(header);
        // Four-byte words so the structure's header is aligned whatever the target.
        let mut buffer = vec![0_u32; bytes.div_ceil(4)];
        let detail = buffer
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: the buffer holds at least the header; the field is written unaligned
        // because the structure is packed on x86.
        unsafe { ptr::addr_of_mut!((*detail).cbSize).write_unaligned(header as u32) };
        let mut info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: detail points at `bytes` writable bytes, which is what the call is told.
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &interface,
                Some(detail),
                bytes as u32,
                None,
                Some(&mut info),
            )
        }
        .map_err(|error| enumeration_error("SetupDiGetDeviceInterfaceDetailW", error))?;
        // SAFETY: DevicePath starts inside the buffer; reading stops at the
        // terminator or at the end of what the call filled.
        let device_path = unsafe {
            let start = ptr::addr_of!((*detail).DevicePath).cast::<u16>();
            let offset = start as usize - detail as usize;
            let available = bytes.saturating_sub(offset) / 2;
            let mut units = Vec::with_capacity(available);
            for position in 0..available {
                let unit = start.add(position).read_unaligned();
                if unit == 0 {
                    break;
                }
                units.push(unit);
            }
            String::from_utf16_lossy(&units)
        };

        let mut instance_id = [0_u16; 512];
        // SAFETY: info came filled from the detail call; the buffer is ours.
        unsafe { SetupDiGetDeviceInstanceIdW(set, &info, Some(&mut instance_id), None) }
            .map_err(|error| enumeration_error("SetupDiGetDeviceInstanceIdW", error))?;
        let length = instance_id
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(instance_id.len());
        let instance_id = String::from_utf16_lossy(&instance_id[..length]);
        // The device must be bound to the package's driver now, whatever
        // interfaces its registry carries from before.
        if !bound_service(set, &info).eq_ignore_ascii_case(DRIVER_SERVICE) {
            continue;
        }
        if let Some(identity) = parse_instance_id(&instance_id) {
            devices.push(PassThruDevice {
                device_path,
                usb_vid: identity.vid,
                usb_pid: identity.pid,
                usb_serial_number: identity.serial_number,
            });
        }
    }
    devices.sort_by(|left, right| left.device_path.cmp(&right.device_path));
    devices.dedup();
    Ok(devices)
}

/// The name of the service that holds the device: `usbser` for Windows's
/// serial driver, `dtmonpro` for the package's; empty when Windows does not
/// say.
fn bound_service(set: HDEVINFO, info: &SP_DEVINFO_DATA) -> String {
    let mut buffer = [0_u8; 512];
    let mut kind = 0_u32;
    let mut filled = 0_u32;
    // SAFETY: info came from the detail call on this set; the buffer is ours.
    let read = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            info,
            SPDRP_SERVICE,
            Some(&mut kind),
            Some(&mut buffer),
            Some(&mut filled),
        )
    };
    if read.is_err() {
        return String::new();
    }
    let units: Vec<u16> = buffer[..(filled as usize).min(buffer.len())]
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|&unit| unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// One handle on the device interface, opened for overlapped I/O so that a
/// read can wait on one thread while a write goes out on another.
pub(crate) struct OverlappedPipe {
    handle: HANDLE,
}

// SAFETY: a Win32 file handle may be used from any thread; every transfer
// here carries its own OVERLAPPED and event, and the handle is closed once,
// in Drop, after the transport has stopped the reader.
unsafe impl Send for OverlappedPipe {}
unsafe impl Sync for OverlappedPipe {}

impl OverlappedPipe {
    pub(crate) fn open(device_path: &str) -> Result<Self, PipeError> {
        let path = wide(device_path);
        // SAFETY: the path is NUL-terminated and outlives the call.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .map_err(|error| {
            if is_win32(&error, ERROR_SHARING_VIOLATION) || is_win32(&error, ERROR_ACCESS_DENIED) {
                PipeError::Io(format!("the adapter is in use by another program: {error}"))
            } else {
                PipeError::Io(format!(
                    "the MongoosePro driver's device interface could not be opened: {error}"
                ))
            }
        })?;
        Ok(Self { handle })
    }

    /// Starts one overlapped transfer and waits for it: forever for a read,
    /// which the transport ends with `cancel`, and for `wait_ms` for a write.
    fn transfer(
        &self,
        start: impl FnOnce(*mut OVERLAPPED) -> windows::core::Result<()>,
        wait_ms: u32,
    ) -> Result<u32, PipeError> {
        // SAFETY: an unnamed manual-reset event, closed below.
        let event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map_err(|error| PipeError::Io(format!("CreateEventW: {error}")))?;
        let mut overlapped = OVERLAPPED {
            hEvent: event,
            ..OVERLAPPED::default()
        };
        let outcome = (|| {
            match start(&mut overlapped) {
                Ok(()) => {}
                Err(error) if is_win32(&error, ERROR_IO_PENDING) => {}
                Err(error) => return Err(classify(error)),
            }
            // SAFETY: the event belongs to this transfer.
            let waited = unsafe { WaitForSingleObject(event, wait_ms) };
            if waited == WAIT_TIMEOUT {
                // SAFETY: cancel this transfer, then reap it so the OVERLAPPED
                // is not written after it goes out of scope.
                unsafe {
                    let _ = CancelIoEx(self.handle, Some(&overlapped));
                    let mut transferred = 0_u32;
                    let _ = GetOverlappedResult(self.handle, &overlapped, &mut transferred, true);
                }
                return Err(PipeError::Timeout);
            }
            if waited != WAIT_OBJECT_0 {
                return Err(PipeError::Io(format!(
                    "waiting for the transfer failed: {:?}",
                    waited
                )));
            }
            let mut transferred = 0_u32;
            // SAFETY: the transfer has signalled; the OVERLAPPED is still alive.
            unsafe { GetOverlappedResult(self.handle, &overlapped, &mut transferred, false) }
                .map_err(classify)?;
            Ok(transferred)
        })();
        // SAFETY: the event is ours and no transfer refers to it any more.
        unsafe {
            let _ = CloseHandle(event);
        }
        outcome
    }
}

fn classify(error: Error) -> PipeError {
    if is_win32(&error, ERROR_OPERATION_ABORTED) {
        PipeError::Cancelled
    } else {
        PipeError::Io(error.to_string())
    }
}

impl Pipe for OverlappedPipe {
    fn read(&self, buffer: &mut [u8]) -> Result<usize, PipeError> {
        let transferred = self.transfer(
            // SAFETY: the buffer outlives the transfer, which `transfer` sees to the end.
            |overlapped| unsafe { ReadFile(self.handle, Some(buffer), None, Some(overlapped)) },
            INFINITE,
        )?;
        Ok(transferred as usize)
    }

    fn write(&self, bytes: &[u8]) -> Result<(), PipeError> {
        let wait_ms = u32::try_from(WRITE_TIMEOUT.as_millis()).unwrap_or(u32::MAX);
        let transferred = self.transfer(
            // SAFETY: as for read.
            |overlapped| unsafe { WriteFile(self.handle, Some(bytes), None, Some(overlapped)) },
            wait_ms,
        )?;
        if transferred as usize == bytes.len() {
            Ok(())
        } else {
            Err(PipeError::Io(format!(
                "short write: {transferred} of {} bytes",
                bytes.len()
            )))
        }
    }

    fn cancel(&self) {
        // SAFETY: cancels every pending transfer on this handle; each one then
        // completes with ERROR_OPERATION_ABORTED and reaps itself.
        unsafe {
            let _ = CancelIoEx(self.handle, None);
        }
    }
}

impl Drop for OverlappedPipe {
    fn drop(&mut self) {
        // SAFETY: closed once; the transport stops the reader before the last
        // reference to the pipe is dropped.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
