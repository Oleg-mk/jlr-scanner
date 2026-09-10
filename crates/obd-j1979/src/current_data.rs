//! Modes 01 and 02: the PIDs, with the formulas the standard gives them.
//!
//! A PID this table decodes is decoded exactly as SAE J1979 says; a PID the
//! table knows by name and length but whose layout this crate does not
//! vouch for is returned raw and labelled raw. Nothing is guessed: an unknown
//! scaling is a raw value, not an invented one.

use crate::dtc::dtc_text;
use crate::support::{decode_support_bitmap, is_support_item};
use crate::J1979Error;

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterValue {
    Number(f64),
    Text(String),
    Flag(bool),
    /// A support bitmap: the PIDs the module answers after this one.
    Supported(Vec<u8>),
    /// Bytes this crate does not scale; `raw` holds them.
    Raw,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedParameter {
    pub pid: u8,
    pub name: String,
    pub unit: &'static str,
    pub value: ParameterValue,
    pub raw: Vec<u8>,
}

enum Decode {
    Support,
    /// `raw * scale + offset` over all the PID's bytes, big-endian.
    Linear {
        scale: f64,
        offset: f64,
        unit: &'static str,
    },
    /// `A * 100 / 255`.
    Percent,
    /// `(A - 128) * 100 / 128`, the fuel-trim form.
    SignedPercent,
    /// `A - 40` °C.
    Temperature,
    /// A layout the standard fixes but which is more than one value.
    Custom(fn(u8, &[u8]) -> Vec<DecodedParameter>),
    /// Known name and length; the layout is not carried here.
    Raw,
}

struct PidDefinition {
    pid: u8,
    name: &'static str,
    /// Bytes after the PID; 0 when the standard does not fix it.
    length: u8,
    decode: Decode,
}

const fn linear(
    pid: u8,
    name: &'static str,
    length: u8,
    scale: f64,
    offset: f64,
    unit: &'static str,
) -> PidDefinition {
    PidDefinition {
        pid,
        name,
        length,
        decode: Decode::Linear {
            scale,
            offset,
            unit,
        },
    }
}

const fn simple(pid: u8, name: &'static str, length: u8, decode: Decode) -> PidDefinition {
    PidDefinition {
        pid,
        name,
        length,
        decode,
    }
}

const fn raw(pid: u8, name: &'static str, length: u8) -> PidDefinition {
    PidDefinition {
        pid,
        name,
        length,
        decode: Decode::Raw,
    }
}

const fn custom(
    pid: u8,
    name: &'static str,
    length: u8,
    decode: fn(u8, &[u8]) -> Vec<DecodedParameter>,
) -> PidDefinition {
    PidDefinition {
        pid,
        name,
        length,
        decode: Decode::Custom(decode),
    }
}

fn number(
    pid: u8,
    name: impl Into<String>,
    unit: &'static str,
    value: f64,
    raw: &[u8],
) -> DecodedParameter {
    DecodedParameter {
        pid,
        name: name.into(),
        unit,
        value: ParameterValue::Number(value),
        raw: raw.to_vec(),
    }
}

fn text(
    pid: u8,
    name: impl Into<String>,
    value: impl Into<String>,
    raw: &[u8],
) -> DecodedParameter {
    DecodedParameter {
        pid,
        name: name.into(),
        unit: "",
        value: ParameterValue::Text(value.into()),
        raw: raw.to_vec(),
    }
}

fn flag(pid: u8, name: impl Into<String>, value: bool, raw: &[u8]) -> DecodedParameter {
    DecodedParameter {
        pid,
        name: name.into(),
        unit: "",
        value: ParameterValue::Flag(value),
        raw: raw.to_vec(),
    }
}

fn be(bytes: &[u8]) -> f64 {
    bytes
        .iter()
        .fold(0f64, |acc, &b| acc * 256.0 + f64::from(b))
}

fn u16_at(bytes: &[u8], at: usize) -> f64 {
    f64::from(u16::from_be_bytes([bytes[at], bytes[at + 1]]))
}

// --- the structured PIDs ---------------------------------------------------

fn readiness(pid: u8, bytes: &[u8], with_mil: bool) -> Vec<DecodedParameter> {
    let mut out = Vec::new();
    let (a, b, c, d) = (bytes[0], bytes[1], bytes[2], bytes[3]);
    if with_mil {
        out.push(flag(
            pid,
            "malfunction indicator lamp",
            a & 0x80 != 0,
            &bytes[..1],
        ));
        out.push(number(
            pid,
            "stored fault codes",
            "count",
            f64::from(a & 0x7F),
            &bytes[..1],
        ));
    }
    let compression = b & 0x08 != 0;
    out.push(text(
        pid,
        "ignition",
        if compression { "compression" } else { "spark" },
        &bytes[1..2],
    ));
    let state = |available: bool, incomplete: bool| -> &'static str {
        match (available, incomplete) {
            (false, _) => "not available",
            (true, true) => "incomplete",
            (true, false) => "complete",
        }
    };
    for (bit, name) in [
        (0u8, "misfire monitor"),
        (1, "fuel system monitor"),
        (2, "comprehensive component monitor"),
    ] {
        out.push(text(
            pid,
            name,
            state(b & (1 << bit) != 0, b & (1 << (bit + 4)) != 0),
            &bytes[1..2],
        ));
    }
    let names: [&str; 8] = if compression {
        [
            "NMHC catalyst monitor",
            "NOx / SCR aftertreatment monitor",
            "reserved",
            "boost pressure monitor",
            "reserved",
            "exhaust gas sensor monitor",
            "PM filter monitor",
            "EGR / VVT monitor",
        ]
    } else {
        [
            "catalyst monitor",
            "heated catalyst monitor",
            "evaporative system monitor",
            "secondary air monitor",
            "A/C refrigerant monitor",
            "oxygen sensor monitor",
            "oxygen sensor heater monitor",
            "EGR / VVT monitor",
        ]
    };
    for (bit, name) in names.iter().enumerate() {
        if *name == "reserved" {
            continue;
        }
        out.push(text(
            pid,
            *name,
            state(c & (1 << bit) != 0, d & (1 << bit) != 0),
            &bytes[2..4],
        ));
    }
    out
}

fn monitor_status(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    readiness(pid, bytes, true)
}

fn drive_cycle_status(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    readiness(pid, bytes, false)
}

fn freeze_frame_dtc(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let value = if bytes[..2] == [0, 0] {
        "none".to_string()
    } else {
        dtc_text(bytes[0], bytes[1])
    };
    vec![text(
        pid,
        "fault code that froze the frame",
        value,
        &bytes[..2],
    )]
}

fn fuel_system_text(value: u8) -> &'static str {
    match value {
        0 => "not present",
        1 => "open loop, engine not yet warm",
        2 => "closed loop, oxygen sensor feedback",
        4 => "open loop, load or deceleration",
        8 => "open loop, system fault",
        16 => "closed loop, feedback fault",
        _ => "unlisted state",
    }
}

fn fuel_system_status(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let mut out = vec![text(
        pid,
        "fuel system 1",
        fuel_system_text(bytes[0]),
        &bytes[..1],
    )];
    if bytes.len() > 1 && bytes[1] != 0 {
        out.push(text(
            pid,
            "fuel system 2",
            fuel_system_text(bytes[1]),
            &bytes[1..2],
        ));
    }
    out
}

fn secondary_air_status(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let value = match bytes[0] {
        1 => "upstream",
        2 => "downstream of catalytic converter",
        4 => "from the outside atmosphere or off",
        8 => "pump commanded on for diagnostics",
        _ => "unlisted state",
    };
    vec![text(pid, "commanded secondary air", value, &bytes[..1])]
}

fn oxygen_sensors_present_two_banks(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    (0..8u8)
        .map(|bit| {
            flag(
                pid,
                format!(
                    "oxygen sensor bank {} sensor {} present",
                    bit / 4 + 1,
                    bit % 4 + 1
                ),
                bytes[0] & (1 << bit) != 0,
                &bytes[..1],
            )
        })
        .collect()
}

fn oxygen_sensors_present_four_banks(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    (0..8u8)
        .map(|bit| {
            flag(
                pid,
                format!(
                    "oxygen sensor bank {} sensor {} present",
                    bit / 2 + 1,
                    bit % 2 + 1
                ),
                bytes[0] & (1 << bit) != 0,
                &bytes[..1],
            )
        })
        .collect()
}

fn oxygen_sensor_voltage(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let sensor = pid - 0x14 + 1;
    let mut out = vec![number(
        pid,
        format!("oxygen sensor {sensor} voltage"),
        "V",
        f64::from(bytes[0]) / 200.0,
        &bytes[..1],
    )];
    if bytes[1] != 0xFF {
        out.push(number(
            pid,
            format!("oxygen sensor {sensor} short-term fuel trim"),
            "%",
            (f64::from(bytes[1]) - 128.0) * 100.0 / 128.0,
            &bytes[1..2],
        ));
    }
    out
}

fn oxygen_sensor_lambda_voltage(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let sensor = pid - 0x24 + 1;
    vec![
        number(
            pid,
            format!("oxygen sensor {sensor} lambda"),
            "λ",
            u16_at(bytes, 0) * 2.0 / 65536.0,
            &bytes[..2],
        ),
        number(
            pid,
            format!("oxygen sensor {sensor} voltage"),
            "V",
            u16_at(bytes, 2) * 8.0 / 65536.0,
            &bytes[2..4],
        ),
    ]
}

fn oxygen_sensor_lambda_current(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let sensor = pid - 0x34 + 1;
    vec![
        number(
            pid,
            format!("oxygen sensor {sensor} lambda"),
            "λ",
            u16_at(bytes, 0) * 2.0 / 65536.0,
            &bytes[..2],
        ),
        number(
            pid,
            format!("oxygen sensor {sensor} current"),
            "mA",
            u16_at(bytes, 2) / 256.0 - 128.0,
            &bytes[2..4],
        ),
    ]
}

fn obd_standard(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let value = match bytes[0] {
        1 => "OBD-II (CARB)",
        2 => "OBD (EPA)",
        3 => "OBD and OBD-II",
        4 => "OBD-I",
        5 => "not OBD compliant",
        6 => "EOBD",
        7 => "EOBD and OBD-II",
        8 => "EOBD and OBD",
        9 => "EOBD, OBD and OBD-II",
        10 => "JOBD",
        11 => "JOBD and OBD-II",
        12 => "JOBD and EOBD",
        13 => "JOBD, EOBD and OBD-II",
        17 => "EMD",
        18 => "EMD+",
        19 => "HD OBD-C",
        20 => "HD OBD",
        21 => "WWH OBD",
        23 => "HD EOBD-I",
        24 => "HD EOBD-I N",
        25 => "HD EOBD-II",
        26 => "HD EOBD-II N",
        28 => "OBDBr-1",
        29 => "OBDBr-2",
        30 => "KOBD",
        31 => "IOBD I",
        32 => "IOBD II",
        33 => "HD EOBD-IV",
        _ => "unlisted standard",
    };
    vec![text(
        pid,
        "OBD standard this vehicle conforms to",
        value,
        &bytes[..1],
    )]
}

fn auxiliary_input(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    vec![flag(
        pid,
        "power take-off active",
        bytes[0] & 0x01 != 0,
        &bytes[..1],
    )]
}

fn maximum_values(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    vec![
        number(pid, "maximum lambda", "λ", f64::from(bytes[0]), &bytes[..1]),
        number(
            pid,
            "maximum oxygen sensor voltage",
            "V",
            f64::from(bytes[1]),
            &bytes[1..2],
        ),
        number(
            pid,
            "maximum oxygen sensor current",
            "mA",
            f64::from(bytes[2]),
            &bytes[2..3],
        ),
        number(
            pid,
            "maximum intake manifold absolute pressure",
            "kPa",
            f64::from(bytes[3]) * 10.0,
            &bytes[3..4],
        ),
    ]
}

fn maximum_air_flow(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    vec![number(
        pid,
        "maximum mass air flow",
        "g/s",
        f64::from(bytes[0]) * 10.0,
        &bytes[..1],
    )]
}

fn fuel_type(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let value = match bytes[0] {
        0 => "not available",
        1 => "gasoline",
        2 => "methanol",
        3 => "ethanol",
        4 => "diesel",
        5 => "LPG",
        6 => "CNG",
        7 => "propane",
        8 => "electric",
        9 => "bifuel, gasoline",
        10 => "bifuel, methanol",
        11 => "bifuel, ethanol",
        12 => "bifuel, LPG",
        13 => "bifuel, CNG",
        14 => "bifuel, propane",
        15 => "bifuel, electricity",
        16 => "bifuel, electric and combustion",
        17 => "hybrid gasoline",
        18 => "hybrid ethanol",
        19 => "hybrid diesel",
        20 => "hybrid electric",
        21 => "hybrid electric and combustion",
        22 => "hybrid regenerative",
        23 => "bifuel, diesel",
        _ => "unlisted fuel",
    };
    vec![text(pid, "fuel type", value, &bytes[..1])]
}

fn secondary_oxygen_trim(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let (term, banks) = match pid {
        0x55 => ("short-term", [1, 3]),
        0x56 => ("long-term", [1, 3]),
        0x57 => ("short-term", [2, 4]),
        _ => ("long-term", [2, 4]),
    };
    let mut out = vec![number(
        pid,
        format!("{term} secondary oxygen sensor trim, bank {}", banks[0]),
        "%",
        (f64::from(bytes[0]) - 128.0) * 100.0 / 128.0,
        &bytes[..1],
    )];
    if bytes.len() > 1 {
        out.push(number(
            pid,
            format!("{term} secondary oxygen sensor trim, bank {}", banks[1]),
            "%",
            (f64::from(bytes[1]) - 128.0) * 100.0 / 128.0,
            &bytes[1..2],
        ));
    }
    out
}

fn torque_data(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    ["idle", "point 1", "point 2", "point 3", "point 4"]
        .iter()
        .enumerate()
        .map(|(index, point)| {
            number(
                pid,
                format!("engine percent torque, {point}"),
                "%",
                f64::from(bytes[index]) - 125.0,
                &bytes[index..index + 1],
            )
        })
        .collect()
}

fn auxiliary_io(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    let status = bytes[1];
    [
        (0u8, "power take-off status"),
        (1, "automatic transmission in neutral"),
        (2, "manual transmission in neutral"),
        (3, "glow plug lamp"),
    ]
    .iter()
    .filter(|(bit, _)| support & (1 << bit) != 0)
    .map(|(bit, name)| flag(pid, *name, status & (1 << bit) != 0, &bytes[1..2]))
    .collect()
}

fn mass_air_flow_sensors(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    let mut out = Vec::new();
    if support & 0x01 != 0 {
        out.push(number(
            pid,
            "mass air flow sensor A",
            "g/s",
            u16_at(bytes, 1) * 0.03125,
            &bytes[1..3],
        ));
    }
    if support & 0x02 != 0 {
        out.push(number(
            pid,
            "mass air flow sensor B",
            "g/s",
            u16_at(bytes, 3) * 0.03125,
            &bytes[3..5],
        ));
    }
    out
}

fn coolant_temperatures(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    let mut out = Vec::new();
    if support & 0x01 != 0 {
        out.push(number(
            pid,
            "engine coolant temperature, sensor 1",
            "°C",
            f64::from(bytes[1]) - 40.0,
            &bytes[1..2],
        ));
    }
    if support & 0x02 != 0 {
        out.push(number(
            pid,
            "engine coolant temperature, sensor 2",
            "°C",
            f64::from(bytes[2]) - 40.0,
            &bytes[2..3],
        ));
    }
    out
}

fn intake_air_temperatures(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    (0..6u8)
        .filter(|bit| support & (1 << bit) != 0)
        .map(|bit| {
            let at = usize::from(bit) + 1;
            number(
                pid,
                format!(
                    "intake air temperature, bank {} sensor {}",
                    bit / 3 + 1,
                    bit % 3 + 1
                ),
                "°C",
                f64::from(bytes[at]) - 40.0,
                &bytes[at..at + 1],
            )
        })
        .collect()
}

fn throttle_actuator(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    let mut out = Vec::new();
    if support & 0x01 != 0 {
        out.push(number(
            pid,
            "commanded throttle actuator",
            "%",
            f64::from(bytes[1]) * 100.0 / 255.0,
            &bytes[1..2],
        ));
    }
    if support & 0x02 != 0 {
        out.push(number(
            pid,
            "relative throttle position",
            "%",
            f64::from(bytes[2]) * 100.0 / 255.0,
            &bytes[2..3],
        ));
    }
    out
}

fn exhaust_gas_temperatures(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let bank = if matches!(pid, 0x78 | 0x98) { 1 } else { 2 };
    let support = bytes[0];
    (0..4u8)
        .filter(|bit| support & (1 << bit) != 0)
        .map(|bit| {
            let at = usize::from(bit) * 2 + 1;
            number(
                pid,
                format!("exhaust gas temperature, bank {bank} sensor {}", bit + 1),
                "°C",
                u16_at(bytes, at) * 0.1 - 40.0,
                &bytes[at..at + 2],
            )
        })
        .collect()
}

fn engine_run_times(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    [
        (0u8, "total engine run time", 1usize),
        (1, "total idle run time", 5),
        (2, "total run time with power take-off", 9),
    ]
    .iter()
    .filter(|(bit, _, _)| support & (1 << bit) != 0)
    .map(|(_, name, at)| {
        number(
            pid,
            *name,
            "s",
            be(&bytes[*at..*at + 4]),
            &bytes[*at..*at + 4],
        )
    })
    .collect()
}

fn fuel_rates(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    vec![
        number(
            pid,
            "engine fuel rate",
            "g/s",
            u16_at(bytes, 0) * 0.02,
            &bytes[..2],
        ),
        number(
            pid,
            "vehicle fuel rate",
            "g/s",
            u16_at(bytes, 2) * 0.02,
            &bytes[2..4],
        ),
    ]
}

fn actual_gear(pid: u8, bytes: &[u8]) -> Vec<DecodedParameter> {
    let support = bytes[0];
    if support & 0x01 != 0 {
        vec![number(
            pid,
            "actual gear ratio",
            "",
            u16_at(bytes, 2) * 0.001,
            &bytes[2..4],
        )]
    } else {
        Vec::new()
    }
}

// --- the table ---------------------------------------------------------------

const PIDS: &[PidDefinition] = &[
    simple(0x00, "PIDs supported, 01–20", 4, Decode::Support),
    custom(
        0x01,
        "monitor status since codes cleared",
        4,
        monitor_status,
    ),
    custom(0x02, "fault code that froze the frame", 2, freeze_frame_dtc),
    custom(0x03, "fuel system status", 2, fuel_system_status),
    simple(0x04, "calculated engine load", 1, Decode::Percent),
    simple(0x05, "engine coolant temperature", 1, Decode::Temperature),
    simple(
        0x06,
        "short-term fuel trim, bank 1",
        1,
        Decode::SignedPercent,
    ),
    simple(
        0x07,
        "long-term fuel trim, bank 1",
        1,
        Decode::SignedPercent,
    ),
    simple(
        0x08,
        "short-term fuel trim, bank 2",
        1,
        Decode::SignedPercent,
    ),
    simple(
        0x09,
        "long-term fuel trim, bank 2",
        1,
        Decode::SignedPercent,
    ),
    linear(0x0A, "fuel pressure, gauge", 1, 3.0, 0.0, "kPa"),
    linear(
        0x0B,
        "intake manifold absolute pressure",
        1,
        1.0,
        0.0,
        "kPa",
    ),
    linear(0x0C, "engine speed", 2, 0.25, 0.0, "rpm"),
    linear(0x0D, "vehicle speed", 1, 1.0, 0.0, "km/h"),
    linear(
        0x0E,
        "timing advance before top dead centre",
        1,
        0.5,
        -64.0,
        "°",
    ),
    simple(0x0F, "intake air temperature", 1, Decode::Temperature),
    linear(0x10, "mass air flow rate", 2, 0.01, 0.0, "g/s"),
    simple(0x11, "throttle position", 1, Decode::Percent),
    custom(
        0x12,
        "commanded secondary air status",
        1,
        secondary_air_status,
    ),
    custom(
        0x13,
        "oxygen sensors present, two banks",
        1,
        oxygen_sensors_present_two_banks,
    ),
    custom(0x14, "oxygen sensor 1", 2, oxygen_sensor_voltage),
    custom(0x15, "oxygen sensor 2", 2, oxygen_sensor_voltage),
    custom(0x16, "oxygen sensor 3", 2, oxygen_sensor_voltage),
    custom(0x17, "oxygen sensor 4", 2, oxygen_sensor_voltage),
    custom(0x18, "oxygen sensor 5", 2, oxygen_sensor_voltage),
    custom(0x19, "oxygen sensor 6", 2, oxygen_sensor_voltage),
    custom(0x1A, "oxygen sensor 7", 2, oxygen_sensor_voltage),
    custom(0x1B, "oxygen sensor 8", 2, oxygen_sensor_voltage),
    custom(0x1C, "OBD standard", 1, obd_standard),
    custom(
        0x1D,
        "oxygen sensors present, four banks",
        1,
        oxygen_sensors_present_four_banks,
    ),
    custom(0x1E, "auxiliary input status", 1, auxiliary_input),
    linear(0x1F, "run time since engine start", 2, 1.0, 0.0, "s"),
    simple(0x20, "PIDs supported, 21–40", 4, Decode::Support),
    linear(
        0x21,
        "distance travelled with the lamp on",
        2,
        1.0,
        0.0,
        "km",
    ),
    linear(
        0x22,
        "fuel rail pressure, relative to manifold vacuum",
        2,
        0.079,
        0.0,
        "kPa",
    ),
    linear(0x23, "fuel rail gauge pressure", 2, 10.0, 0.0, "kPa"),
    custom(
        0x24,
        "oxygen sensor 1, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x25,
        "oxygen sensor 2, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x26,
        "oxygen sensor 3, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x27,
        "oxygen sensor 4, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x28,
        "oxygen sensor 5, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x29,
        "oxygen sensor 6, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x2A,
        "oxygen sensor 7, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    custom(
        0x2B,
        "oxygen sensor 8, wide range",
        4,
        oxygen_sensor_lambda_voltage,
    ),
    simple(0x2C, "commanded EGR", 1, Decode::Percent),
    simple(0x2D, "EGR error", 1, Decode::SignedPercent),
    simple(0x2E, "commanded evaporative purge", 1, Decode::Percent),
    simple(0x2F, "fuel tank level", 1, Decode::Percent),
    linear(0x30, "warm-ups since codes cleared", 1, 1.0, 0.0, "count"),
    linear(0x31, "distance since codes cleared", 2, 1.0, 0.0, "km"),
    linear(
        0x32,
        "evaporative system vapour pressure",
        2,
        0.25,
        0.0,
        "Pa",
    ),
    linear(0x33, "absolute barometric pressure", 1, 1.0, 0.0, "kPa"),
    custom(
        0x34,
        "oxygen sensor 1, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x35,
        "oxygen sensor 2, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x36,
        "oxygen sensor 3, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x37,
        "oxygen sensor 4, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x38,
        "oxygen sensor 5, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x39,
        "oxygen sensor 6, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x3A,
        "oxygen sensor 7, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    custom(
        0x3B,
        "oxygen sensor 8, wide range, current",
        4,
        oxygen_sensor_lambda_current,
    ),
    linear(
        0x3C,
        "catalyst temperature, bank 1 sensor 1",
        2,
        0.1,
        -40.0,
        "°C",
    ),
    linear(
        0x3D,
        "catalyst temperature, bank 2 sensor 1",
        2,
        0.1,
        -40.0,
        "°C",
    ),
    linear(
        0x3E,
        "catalyst temperature, bank 1 sensor 2",
        2,
        0.1,
        -40.0,
        "°C",
    ),
    linear(
        0x3F,
        "catalyst temperature, bank 2 sensor 2",
        2,
        0.1,
        -40.0,
        "°C",
    ),
    simple(0x40, "PIDs supported, 41–60", 4, Decode::Support),
    custom(
        0x41,
        "monitor status this drive cycle",
        4,
        drive_cycle_status,
    ),
    linear(0x42, "control module voltage", 2, 0.001, 0.0, "V"),
    linear(0x43, "absolute load value", 2, 100.0 / 255.0, 0.0, "%"),
    linear(
        0x44,
        "commanded equivalence ratio",
        2,
        2.0 / 65536.0,
        0.0,
        "λ",
    ),
    simple(0x45, "relative throttle position", 1, Decode::Percent),
    simple(0x46, "ambient air temperature", 1, Decode::Temperature),
    simple(0x47, "absolute throttle position B", 1, Decode::Percent),
    simple(0x48, "absolute throttle position C", 1, Decode::Percent),
    simple(0x49, "accelerator pedal position D", 1, Decode::Percent),
    simple(0x4A, "accelerator pedal position E", 1, Decode::Percent),
    simple(0x4B, "accelerator pedal position F", 1, Decode::Percent),
    simple(0x4C, "commanded throttle actuator", 1, Decode::Percent),
    linear(0x4D, "time run with the lamp on", 2, 1.0, 0.0, "min"),
    linear(0x4E, "time since codes cleared", 2, 1.0, 0.0, "min"),
    custom(0x4F, "maximum values", 4, maximum_values),
    custom(0x50, "maximum mass air flow", 4, maximum_air_flow),
    custom(0x51, "fuel type", 1, fuel_type),
    simple(0x52, "ethanol fuel", 1, Decode::Percent),
    linear(
        0x53,
        "absolute evaporative system vapour pressure",
        2,
        0.005,
        0.0,
        "kPa",
    ),
    linear(
        0x54,
        "evaporative system vapour pressure",
        2,
        1.0,
        -32767.0,
        "Pa",
    ),
    custom(
        0x55,
        "short-term secondary oxygen sensor trim, banks 1 and 3",
        2,
        secondary_oxygen_trim,
    ),
    custom(
        0x56,
        "long-term secondary oxygen sensor trim, banks 1 and 3",
        2,
        secondary_oxygen_trim,
    ),
    custom(
        0x57,
        "short-term secondary oxygen sensor trim, banks 2 and 4",
        2,
        secondary_oxygen_trim,
    ),
    custom(
        0x58,
        "long-term secondary oxygen sensor trim, banks 2 and 4",
        2,
        secondary_oxygen_trim,
    ),
    linear(0x59, "fuel rail absolute pressure", 2, 10.0, 0.0, "kPa"),
    simple(
        0x5A,
        "relative accelerator pedal position",
        1,
        Decode::Percent,
    ),
    simple(
        0x5B,
        "hybrid battery pack remaining life",
        1,
        Decode::Percent,
    ),
    simple(0x5C, "engine oil temperature", 1, Decode::Temperature),
    linear(0x5D, "fuel injection timing", 2, 1.0 / 128.0, -210.0, "°"),
    linear(0x5E, "engine fuel rate", 2, 0.05, 0.0, "L/h"),
    raw(0x5F, "emission requirements the vehicle is designed to", 1),
    simple(0x60, "PIDs supported, 61–80", 4, Decode::Support),
    linear(0x61, "driver's demand engine torque", 1, 1.0, -125.0, "%"),
    linear(0x62, "actual engine torque", 1, 1.0, -125.0, "%"),
    linear(0x63, "engine reference torque", 2, 1.0, 0.0, "N·m"),
    custom(0x64, "engine percent torque data", 5, torque_data),
    custom(0x65, "auxiliary input and output", 2, auxiliary_io),
    custom(0x66, "mass air flow sensors", 5, mass_air_flow_sensors),
    custom(0x67, "engine coolant temperatures", 3, coolant_temperatures),
    custom(0x68, "intake air temperatures", 7, intake_air_temperatures),
    raw(0x69, "commanded EGR and EGR error", 7),
    raw(0x6A, "commanded diesel intake air flow control", 5),
    raw(0x6B, "exhaust gas recirculation temperature", 5),
    custom(
        0x6C,
        "commanded throttle actuator and relative position",
        3,
        throttle_actuator,
    ),
    raw(0x6D, "fuel pressure control system", 11),
    raw(0x6E, "injection pressure control system", 9),
    raw(0x6F, "turbocharger compressor inlet pressure", 3),
    raw(0x70, "boost pressure control", 10),
    raw(0x71, "variable geometry turbo control", 6),
    raw(0x72, "wastegate control", 5),
    raw(0x73, "exhaust pressure", 5),
    raw(0x74, "turbocharger speed", 5),
    raw(0x75, "turbocharger temperature", 7),
    raw(0x76, "turbocharger temperature", 7),
    raw(0x77, "charge air cooler temperature", 5),
    custom(
        0x78,
        "exhaust gas temperature, bank 1",
        9,
        exhaust_gas_temperatures,
    ),
    custom(
        0x79,
        "exhaust gas temperature, bank 2",
        9,
        exhaust_gas_temperatures,
    ),
    raw(0x7A, "diesel particulate filter, bank 1", 7),
    raw(0x7B, "diesel particulate filter, bank 2", 7),
    raw(0x7C, "diesel particulate filter temperature", 9),
    raw(0x7D, "NOx NTE control area status", 1),
    raw(0x7E, "PM NTE control area status", 1),
    custom(0x7F, "engine run time", 13, engine_run_times),
    simple(0x80, "PIDs supported, 81–A0", 4, Decode::Support),
    raw(0x81, "engine run time for AECD 1–5", 21),
    raw(0x82, "engine run time for AECD 6–10", 21),
    raw(0x83, "NOx sensor", 5),
    simple(0x84, "manifold surface temperature", 1, Decode::Temperature),
    raw(0x85, "NOx reagent system", 10),
    raw(0x86, "particulate matter sensor", 5),
    raw(0x87, "intake manifold absolute pressure, two banks", 5),
    raw(0x88, "SCR inducement system", 13),
    raw(0x89, "engine run time for AECD 11–15", 41),
    raw(0x8A, "engine run time for AECD 16–20", 41),
    raw(0x8B, "diesel aftertreatment", 7),
    raw(0x8C, "oxygen sensor, wide range", 17),
    simple(0x8D, "throttle position G", 1, Decode::Percent),
    linear(0x8E, "engine friction percent torque", 1, 1.0, -125.0, "%"),
    raw(0x8F, "particulate matter sensor, banks 1 and 2", 7),
    raw(0x90, "WWH-OBD vehicle OBD system information", 3),
    raw(0x91, "WWH-OBD vehicle OBD system information", 5),
    raw(0x92, "fuel system control", 2),
    raw(0x93, "WWH-OBD vehicle OBD counters", 3),
    raw(0x94, "NOx warning and inducement system", 12),
    custom(
        0x98,
        "exhaust gas temperature sensor, bank 1",
        9,
        exhaust_gas_temperatures,
    ),
    custom(
        0x99,
        "exhaust gas temperature sensor, bank 2",
        9,
        exhaust_gas_temperatures,
    ),
    raw(0x9A, "hybrid or EV system data", 6),
    raw(0x9B, "diesel exhaust fluid sensor", 4),
    raw(0x9C, "oxygen sensor data", 17),
    custom(0x9D, "fuel rates", 4, fuel_rates),
    linear(0x9E, "engine exhaust flow rate", 2, 0.2, 0.0, "kg/h"),
    raw(0x9F, "fuel system percentage use", 9),
    simple(0xA0, "PIDs supported, A1–C0", 4, Decode::Support),
    raw(0xA1, "NOx sensor, corrected", 9),
    linear(0xA2, "cylinder fuel rate", 2, 0.03125, 0.0, "mg/stroke"),
    raw(0xA3, "evaporative system vapour pressure, two sensors", 9),
    custom(0xA4, "transmission actual gear", 4, actual_gear),
    raw(0xA5, "commanded diesel exhaust fluid dosing", 4),
    linear(0xA6, "odometer", 4, 0.1, 0.0, "km"),
    raw(0xA7, "NOx sensor concentration, sensors 3 and 4", 0),
    raw(
        0xA8,
        "NOx sensor corrected concentration, sensors 3 and 4",
        0,
    ),
    raw(0xA9, "ABS disable switch state", 0),
    simple(0xC0, "PIDs supported, C1–E0", 4, Decode::Support),
    raw(0xC3, "fuel level input A/B", 0),
    raw(
        0xC4,
        "exhaust particulate control system diagnostic time and count",
        0,
    ),
];

fn definition(pid: u8) -> Option<&'static PidDefinition> {
    PIDS.iter().find(|definition| definition.pid == pid)
}

/// The name the standard gives a PID, when it has one.
pub fn pid_name(pid: u8) -> Option<&'static str> {
    definition(pid).map(|definition| definition.name)
}

/// The bytes a PID carries, when the standard fixes them.
pub fn pid_length(pid: u8) -> Option<usize> {
    definition(pid)
        .map(|definition| usize::from(definition.length))
        .filter(|length| *length > 0)
}

/// Decode one PID's bytes. Bytes beyond the PID's length are ignored.
pub fn decode_pid(pid: u8, bytes: &[u8]) -> Result<Vec<DecodedParameter>, J1979Error> {
    let Some(definition) = definition(pid) else {
        return Ok(vec![DecodedParameter {
            pid,
            name: format!("PID {pid:#04X}"),
            unit: "",
            value: ParameterValue::Raw,
            raw: bytes.to_vec(),
        }]);
    };
    let needed = usize::from(definition.length);
    if bytes.len() < needed {
        return Err(J1979Error::TruncatedParameter {
            pid,
            expected: needed,
            actual: bytes.len(),
        });
    }
    let bytes = if needed > 0 { &bytes[..needed] } else { bytes };
    let single = |value: f64, unit: &'static str| {
        vec![DecodedParameter {
            pid,
            name: definition.name.to_string(),
            unit,
            value: ParameterValue::Number(value),
            raw: bytes.to_vec(),
        }]
    };
    Ok(match definition.decode {
        Decode::Support => vec![DecodedParameter {
            pid,
            name: definition.name.to_string(),
            unit: "",
            value: ParameterValue::Supported(decode_support_bitmap(pid, bytes)),
            raw: bytes.to_vec(),
        }],
        Decode::Linear {
            scale,
            offset,
            unit,
        } => single(be(bytes) * scale + offset, unit),
        Decode::Percent => single(f64::from(bytes[0]) * 100.0 / 255.0, "%"),
        Decode::SignedPercent => single((f64::from(bytes[0]) - 128.0) * 100.0 / 128.0, "%"),
        Decode::Temperature => single(f64::from(bytes[0]) - 40.0, "°C"),
        Decode::Custom(decode) => decode(pid, bytes),
        Decode::Raw => vec![DecodedParameter {
            pid,
            name: definition.name.to_string(),
            unit: "",
            value: ParameterValue::Raw,
            raw: bytes.to_vec(),
        }],
    })
}

/// The body after `41`: `PID data PID data …` for the PIDs asked for. A PID
/// whose length the standard does not fix can only come last.
pub fn decode_current_data(
    requested: &[u8],
    body: &[u8],
) -> Result<Vec<DecodedParameter>, J1979Error> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some((&pid, data)) = rest.split_first() {
        if !requested.contains(&pid) {
            return Err(J1979Error::UnexpectedPid(pid));
        }
        let length = if is_support_item(pid) {
            Some(4)
        } else {
            pid_length(pid)
        };
        let taken = match length {
            Some(length) => {
                if data.len() < length {
                    return Err(J1979Error::TruncatedParameter {
                        pid,
                        expected: length,
                        actual: data.len(),
                    });
                }
                length
            }
            None => {
                // Only the last PID in the answer can own the rest of it.
                let last = requested.last() == Some(&pid);
                if !last && !data.is_empty() {
                    return Err(J1979Error::UnknownPidLength(pid));
                }
                data.len()
            }
        };
        out.extend(decode_pid(pid, &data[..taken])?);
        rest = &data[taken..];
    }
    Ok(out)
}

/// The body after `42`: `PID frame data`. The frame number must be the one
/// asked for.
pub fn decode_freeze_frame(
    pid: u8,
    frame: u8,
    body: &[u8],
) -> Result<Vec<DecodedParameter>, J1979Error> {
    if body.len() < 2 {
        return Err(J1979Error::TruncatedResponse);
    }
    if body[0] != pid {
        return Err(J1979Error::UnexpectedPid(body[0]));
    }
    if body[1] != frame {
        return Err(J1979Error::UnexpectedFrame {
            expected: frame,
            actual: body[1],
        });
    }
    decode_pid(pid, &body[2..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(pid: u8, bytes: &[u8]) -> DecodedParameter {
        let mut decoded = decode_pid(pid, bytes).unwrap();
        assert_eq!(decoded.len(), 1, "{decoded:?}");
        decoded.remove(0)
    }

    fn value(pid: u8, bytes: &[u8]) -> f64 {
        match one(pid, bytes).value {
            ParameterValue::Number(value) => value,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_formulas_of_the_standard_give_its_worked_values() {
        assert_eq!(value(0x0C, &[0x1A, 0xF8]), 1726.0);
        assert_eq!(value(0x05, &[0x7B]), 83.0);
        assert_eq!(value(0x04, &[0xFF]), 100.0);
        assert_eq!(value(0x0D, &[0x3C]), 60.0);
        assert_eq!(value(0x06, &[0x80]), 0.0);
        assert_eq!(value(0x0E, &[0x80]), 0.0);
        assert_eq!(value(0x10, &[0x01, 0x00]), 2.56);
        assert_eq!(value(0x1F, &[0x00, 0x3C]), 60.0);
        assert_eq!(value(0x21, &[0x01, 0x00]), 256.0);
        assert_eq!(value(0x42, &[0x35, 0x60]), 13.664);
        assert_eq!(value(0x5C, &[0x50]), 40.0);
        assert_eq!(value(0x5E, &[0x01, 0x00]), 12.8);
        assert_eq!(value(0x44, &[0x80, 0x00]), 1.0);
        assert_eq!(value(0x62, &[0x7D]), 0.0);
        assert_eq!(value(0xA6, &[0x00, 0x01, 0x86, 0xA0]), 10000.0);
        assert_eq!(one(0x0C, &[0x1A, 0xF8]).unit, "rpm");
        assert_eq!(one(0x0C, &[0x1A, 0xF8]).name, "engine speed");
    }

    #[test]
    fn structured_pids_name_each_value() {
        let status = decode_pid(0x01, &[0x81, 0x07, 0xFF, 0x00]).unwrap();
        assert_eq!(status[0].value, ParameterValue::Flag(true));
        assert_eq!(status[1].value, ParameterValue::Number(1.0));
        assert_eq!(status[2].value, ParameterValue::Text("spark".into()));
        assert!(status
            .iter()
            .any(|p| p.name == "catalyst monitor"
                && p.value == ParameterValue::Text("complete".into())));

        let oxygen = decode_pid(0x14, &[0x64, 0x80]).unwrap();
        assert_eq!(oxygen[0].name, "oxygen sensor 1 voltage");
        assert_eq!(oxygen[0].value, ParameterValue::Number(0.5));
        assert_eq!(oxygen[1].value, ParameterValue::Number(0.0));
        // 0xFF in the trim byte means the sensor is not used for trim.
        assert_eq!(decode_pid(0x14, &[0x64, 0xFF]).unwrap().len(), 1);

        let wide = decode_pid(0x24, &[0x80, 0x00, 0x40, 0x00]).unwrap();
        assert_eq!(wide[0].value, ParameterValue::Number(1.0));
        assert_eq!(wide[1].value, ParameterValue::Number(2.0));

        assert_eq!(
            one(0x1C, &[0x06]).value,
            ParameterValue::Text("EOBD".into())
        );
        assert_eq!(
            one(0x51, &[0x01]).value,
            ParameterValue::Text("gasoline".into())
        );
        assert_eq!(
            decode_pid(0x03, &[0x02, 0x00]).unwrap()[0].value,
            ParameterValue::Text("closed loop, oxygen sensor feedback".into())
        );
        let coolant = decode_pid(0x67, &[0x03, 0x7B, 0x28]).unwrap();
        assert_eq!(coolant.len(), 2);
        assert_eq!(coolant[1].value, ParameterValue::Number(0.0));
        let run = decode_pid(0x7F, &[0x01, 0, 0, 0x0E, 0x10, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(run.len(), 1);
        assert_eq!(run[0].value, ParameterValue::Number(3600.0));
    }

    #[test]
    fn a_pid_without_a_carried_layout_stays_raw_and_an_unknown_one_is_named_by_number() {
        let raw = one(0x6D, &[0; 11]);
        assert_eq!(raw.value, ParameterValue::Raw);
        assert_eq!(raw.name, "fuel pressure control system");
        let unknown = one(0xF3, &[1, 2]);
        assert_eq!(unknown.value, ParameterValue::Raw);
        assert_eq!(unknown.name, "PID 0xF3");
        assert_eq!(pid_length(0xA7), None);
        assert_eq!(pid_length(0x0C), Some(2));
    }

    #[test]
    fn a_multi_pid_answer_is_walked_by_the_standard_lengths() {
        let decoded = decode_current_data(
            &[0x0C, 0x0D, 0x05],
            &[0x0C, 0x1A, 0xF8, 0x0D, 0x3C, 0x05, 0x7B],
        )
        .unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].value, ParameterValue::Number(1726.0));
        assert_eq!(decoded[1].value, ParameterValue::Number(60.0));
        assert_eq!(decoded[2].value, ParameterValue::Number(83.0));
        assert_eq!(
            decode_current_data(&[0x0C], &[0x0D, 0x3C]),
            Err(J1979Error::UnexpectedPid(0x0D))
        );
        assert_eq!(
            decode_current_data(&[0x0C], &[0x0C, 0x1A]),
            Err(J1979Error::TruncatedParameter {
                pid: 0x0C,
                expected: 2,
                actual: 1
            })
        );
        // A PID of unfixed length may only come last.
        assert_eq!(
            decode_current_data(&[0xA7, 0x0D], &[0xA7, 1, 2, 0x0D, 0x3C]),
            Err(J1979Error::UnknownPidLength(0xA7))
        );
        let tail = decode_current_data(&[0x0D, 0xA7], &[0x0D, 0x3C, 0xA7, 1, 2]).unwrap();
        assert_eq!(tail[1].raw, [1, 2]);
        let support = decode_current_data(&[0x00], &[0x00, 0xBE, 0x1F, 0xA8, 0x13]).unwrap();
        assert!(
            matches!(&support[0].value, ParameterValue::Supported(pids) if pids.contains(&0x0C))
        );
    }

    #[test]
    fn a_freeze_frame_checks_its_pid_and_frame() {
        let decoded = decode_freeze_frame(0x0C, 0, &[0x0C, 0x00, 0x1A, 0xF8]).unwrap();
        assert_eq!(decoded[0].value, ParameterValue::Number(1726.0));
        assert_eq!(
            decode_freeze_frame(0x0C, 0, &[0x0C, 0x01, 0x1A, 0xF8]),
            Err(J1979Error::UnexpectedFrame {
                expected: 0,
                actual: 1
            })
        );
        let code = decode_freeze_frame(0x02, 0, &[0x02, 0x00, 0x03, 0x00]).unwrap();
        assert_eq!(code[0].value, ParameterValue::Text("P0300".into()));
    }
}
