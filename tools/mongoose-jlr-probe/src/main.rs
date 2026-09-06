use mongoose_jlr::{
    list_vehicle_routes, CanIdFormat, MongooseJlrDevice, PassiveCapability, VehicleRouteId,
};
use std::collections::HashSet;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use transport_serial::{
    discover_mongoose_jlr, matching_devices, SerialTransport, SystemSerialDeviceEnumerator,
    MONGOOSE_JLR_USB_PID, MONGOOSE_JLR_USB_VID,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Enumerate,
    BoardInfo,
    Routes,
    PassiveRoute(VehicleRouteId),
}

fn parse_mode(arguments: &[String]) -> Result<Mode, String> {
    match arguments {
        [argument] if argument == "--enumerate" => Ok(Mode::Enumerate),
        [argument] if argument == "--board-info" => Ok(Mode::BoardInfo),
        [argument] if argument == "--routes" => Ok(Mode::Routes),
        [argument, route] if argument == "--passive-route" => {
            Ok(Mode::PassiveRoute(route.parse()?))
        }
        _ => Err(
            "usage: mongoose-jlr-probe (--enumerate | --board-info | --routes | --passive-route <route>)"
                .into(),
        ),
    }
}

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match parse_mode(&arguments).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(mode: Mode) -> Result<(), String> {
    match mode {
        Mode::Enumerate => enumerate(),
        Mode::BoardInfo => board_info(),
        Mode::Routes => routes(),
        Mode::PassiveRoute(route) => passive_route(route),
    }
}

fn enumerate() -> Result<(), String> {
    let devices = matching_devices(
        &SystemSerialDeviceEnumerator,
        MONGOOSE_JLR_USB_VID,
        MONGOOSE_JLR_USB_PID,
    )
    .map_err(|error| error.to_string())?;

    println!("writes_performed: 0");
    println!("matching_devices: {}", devices.len());
    for device in devices {
        print_device(&device);
    }
    Ok(())
}

fn board_info() -> Result<(), String> {
    let device = discover_mongoose_jlr().map_err(|error| error.to_string())?;
    print_device(&device);
    let transport = SerialTransport::open(&device).map_err(|error| error.to_string())?;
    let mut mongoose = MongooseJlrDevice::open(transport);
    let result = mongoose.get_board_info();
    let close_result = mongoose.close();

    let info = result.map_err(|error| error.to_string())?;
    close_result.map_err(|error| error.to_string())?;

    println!("writes_performed: 1");
    println!("response_command: 0x{:04X}", info.response_command);
    println!("sequence: {}", info.sequence);
    println!("route_a: 0x{:04X}", info.route_a);
    println!("route_b: 0x{:04X}", info.route_b);
    println!("raw_board_info_length: {}", info.raw_board_info.len());
    println!("raw_board_info: {}", hex(&info.raw_board_info));
    Ok(())
}

fn routes() -> Result<(), String> {
    println!("writes_performed: 0");
    for route in list_vehicle_routes() {
        println!("route: {}", route.id);
        println!("  network: {}", route.network_name);
        println!("  type: {:?}", route.network_type);
        println!(
            "  pins: {}",
            route
                .obd_pins
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("/")
        );
        println!(
            "  bitrate: {}",
            route
                .bitrate
                .map(|value| value.to_string())
                .unwrap_or_else(|| "UNKNOWN".into())
        );
        match route.passive_capability {
            PassiveCapability::Ready => println!("  passive: READY"),
            PassiveCapability::Blocked(reason) => println!("  passive: BLOCKED ({reason})"),
        }
    }
    Ok(())
}

fn passive_route(route: VehicleRouteId) -> Result<(), String> {
    const CAPTURE_DURATION: Duration = Duration::from_secs(20);
    const READ_TIMEOUT: Duration = Duration::from_millis(250);

    let device = discover_mongoose_jlr().map_err(|error| error.to_string())?;
    print_device(&device);
    let transport = SerialTransport::open(&device).map_err(|error| error.to_string())?;
    let mut mongoose = MongooseJlrDevice::open(transport);
    let opened = mongoose
        .open_receive_route(route)
        .map_err(|error| error.to_string())?;
    println!("route: {}", opened.route.id);
    println!("network: {}", opened.route.network_name);
    println!("device_channel_id: 0x{:04X}", opened.device_channel_id);
    println!("application_can_tx: 0");

    let started = Instant::now();
    let mut frames_received = 0_u64;
    let mut parse_errors = 0_u64;
    let mut unique_ids = HashSet::new();
    while started.elapsed() < CAPTURE_DURATION {
        match mongoose.receive_frame(READ_TIMEOUT) {
            Ok(Some(frame)) => {
                frames_received += 1;
                unique_ids.insert((frame.id_format, frame.arbitration_id));
                println!(
                    "{} {} {}{:X} {} {}",
                    frame.device_timestamp,
                    frame.source_route,
                    if frame.id_format == CanIdFormat::Extended {
                        "EXT:"
                    } else {
                        "STD:"
                    },
                    frame.arbitration_id,
                    frame.dlc,
                    hex(frame.payload())
                );
            }
            Ok(None) => {}
            Err(error) => {
                parse_errors += 1;
                eprintln!("parse_error: {error}");
            }
        }
    }
    let dropped_frames = mongoose.receive_counters().dropped_frames;
    let close_result = mongoose.close_route();
    let transport_close_result = mongoose.close();
    close_result.map_err(|error| error.to_string())?;
    transport_close_result.map_err(|error| error.to_string())?;

    println!("capture_duration_ms: {}", started.elapsed().as_millis());
    println!("frames_received: {frames_received}");
    println!("unique_ids: {}", unique_ids.len());
    println!("parse_errors: {parse_errors}");
    println!("dropped_frames: {dropped_frames}");
    println!("application_can_tx: 0");
    Ok(())
}

fn print_device(device: &transport_serial::SerialDevice) {
    println!("usb_vid: {:04X}", device.usb_vid);
    println!("usb_pid: {:04X}", device.usb_pid);
    println!(
        "usb_serial: {}",
        device.usb_serial_number.as_deref().unwrap_or("UNKNOWN")
    );
    println!("port: {}", device.port_name);
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn only_safe_modes_are_allowlisted() {
        assert_eq!(parse_mode(&args(&["--enumerate"])), Ok(Mode::Enumerate));
        assert_eq!(parse_mode(&args(&["--board-info"])), Ok(Mode::BoardInfo));
        assert_eq!(parse_mode(&args(&["--routes"])), Ok(Mode::Routes));
        assert_eq!(
            parse_mode(&args(&["--passive-route", "hs-can"])),
            Ok(Mode::PassiveRoute(VehicleRouteId::HsCan))
        );
        assert!(parse_mode(&args(&["--raw"])).is_err());
        assert!(parse_mode(&args(&["--opcode", "0109"])).is_err());
        assert!(parse_mode(&args(&["--firmware"])).is_err());
        assert!(parse_mode(&args(&["--can"])).is_err());
        assert!(parse_mode(&args(&["--send"])).is_err());
        assert!(parse_mode(&args(&["--tx"])).is_err());
        assert!(parse_mode(&args(&["--uds"])).is_err());
        assert!(parse_mode(&args(&["--isotp"])).is_err());
        assert!(parse_mode(&args(&["--diagnostic"])).is_err());
    }

    #[test]
    fn hex_output_is_stable() {
        assert_eq!(hex(&[0x00, 0xA5, 0xFF]), "00 A5 FF");
    }
}
