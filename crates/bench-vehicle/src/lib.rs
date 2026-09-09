//! The vehicle on the bench (ADR-0020), built from the loaded library: one
//! responder per module the survey names, on its bus and identifiers,
//! answering the read-only services the application sends — fault codes,
//! identifiers, the calibration identification — with values shaped by the
//! catalogue and made from a seed. Everything it says is synthetic; it
//! proves the software above it and never a car, which is why every value
//! it produces reaches the screen labelled so.
//!
//! What it answers comes from the same base that tells the application what
//! to ask: a module the library does not mount does not exist here, a bus the
//! library binds to no pins is silent here, and an identifier the catalogue
//! does not list draws the negative response a real module would give.
//!
//! The fault codes follow a **scenario number** the session connects with.
//! `0` is the healthy vehicle: every module answers and none reports a code,
//! which is the case a car in good order presents and which the application
//! must show as plainly as a broken one. Any other number seeds the draw:
//! each module gets none, one or two codes taken from those the library
//! describes for that very family, so one number always paints one picture,
//! a different number paints a different one, and a tester who quotes the
//! number can be shown the same screen again.

use app_contracts::VehicleContextInput;
use diagnostic_environment::DiagnosticEnvironmentResolver;
use diagnostic_session::decode::parameters_span;
use diagnostic_session::{vehicle_context, KnowledgeLibrary};
use std::collections::{BTreeMap, BTreeSet};
use transport_api::{BenchBus, BenchRoute, CanFrame, CanId};

/// ISO-TP padding, as the application pads its own requests.
const PADDING: u8 = 0x00;
/// The VIN identifier every module answers, with the described vehicle's VIN.
const VIN_IDENTIFIER: u16 = 0xF190;
/// Data bytes for an identifier whose catalogue entry records no byte range.
const UNKNOWN_IDENTIFIER_LENGTH: usize = 4;
/// A VIN of the bench's own, when the session names none.
const BENCH_VIN: &str = "SAJBENCH000000001";
/// The calibration identification the bench's powertrain module reports.
const BENCH_CALIBRATION_ID: &[u8; 16] = b"BENCH-SYNTH-CAL1";
/// The scenario on which every module is healthy and reports nothing.
pub const SCENARIO_HEALTHY: u32 = 0;
/// The scenario a session starts on, when nobody chose another.
pub const SCENARIO_DEFAULT: u32 = 1;
/// Draws attempted per module before the fixed list is fallen back on; a
/// draw that lands on a code no wire format can carry is spent.
const DRAW_ATTEMPTS: usize = 24;
/// Fault codes tried on a module the library describes nothing for, as text
/// and failure type. Generic enough to exist in most catalogues, so even a
/// thin library shows that the read path works.
const CANDIDATE_FAULTS: &[(&str, u8)] = &[
    ("P0300", 0x00),
    ("P0171", 0x00),
    ("P0700", 0x00),
    ("U0100", 0x87),
    ("U0155", 0x87),
    ("U0140", 0x87),
    ("B1A08", 0x11),
    ("C0035", 0x11),
];
/// The fault reported when the library describes none of the candidates.
const FALLBACK_FAULT: (&str, u8) = ("U0100", 0x87);
/// Faults reported per module, at most.
const FAULTS_PER_MODULE: usize = 2;
/// UDS status byte of every reported fault: test failed and confirmed.
const FAULT_STATUS: u8 = 0x09;

struct ModuleResponder {
    family: String,
    route: BenchRoute,
    request_id: u32,
    response_id: u32,
    extended: bool,
    /// Identifier to the number of data bytes it answers with.
    identifiers: BTreeMap<u16, usize>,
    /// Faults as they go on the wire: code high, code low, failure type.
    faults: Vec<[u8; 3]>,
    /// Consecutive frames waiting for the tester's flow control.
    pending: Vec<Vec<u8>>,
}

/// The vehicle described in the session, as far as the library describes it.
pub struct BenchVehicle {
    label: String,
    vin: String,
    modules: Vec<ModuleResponder>,
}

impl BenchVehicle {
    /// Build the vehicle the library describes for this session's context.
    /// Modules without a request identifier, a response identifier and a
    /// bound route are left out and stay silent, exactly as the survey says
    /// they are unreachable.
    pub fn from_library(
        library: &KnowledgeLibrary,
        input: &VehicleContextInput,
        vin: Option<&str>,
        scenario: u32,
    ) -> Self {
        let survey = library.survey(input);
        let context = vehicle_context(input);
        let store = library.store();
        let mut modules = Vec::new();
        for entry in &survey.modules {
            let (Some(request), Some(response), Some(route)) = (
                entry.request_id.as_deref().and_then(parse_id),
                entry.response_id.as_deref().and_then(parse_id),
                entry.backend_route.as_deref().and_then(parse_route),
            ) else {
                continue;
            };
            let mut identifiers: BTreeMap<u16, usize> =
                DiagnosticEnvironmentResolver::readable_identifiers(
                    store,
                    &context,
                    &entry.ecu_family,
                )
                .into_iter()
                .map(|identifier| {
                    (
                        identifier.identifier,
                        parameters_span(&identifier.parameters)
                            .unwrap_or(UNKNOWN_IDENTIFIER_LENGTH),
                    )
                })
                .collect();
            identifiers.insert(VIN_IDENTIFIER, 17);
            modules.push(ModuleResponder {
                family: entry.ecu_family.clone(),
                route,
                request_id: request,
                response_id: response,
                extended: request > 0x7FF,
                identifiers,
                faults: choose_faults(library, &entry.ecu_family, scenario),
                pending: Vec::new(),
            });
        }
        let label = match (&input.year_breakpoint, input.model_year) {
            (Some(marker), _) => format!("{} {marker}", input.vehicle_program),
            (None, Some(year)) => format!("{} {year}", input.vehicle_program),
            (None, None) => input.vehicle_program.clone(),
        };
        Self {
            label,
            vin: vin
                .map(str::trim)
                .filter(|vin| vin.len() == 17)
                .unwrap_or(BENCH_VIN)
                .to_string(),
            modules,
        }
    }

    /// How many modules answer on the bench.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// The families that answer, in survey order.
    pub fn families(&self) -> Vec<&str> {
        self.modules
            .iter()
            .map(|module| module.family.as_str())
            .collect()
    }

    fn answer(vin: &str, module: &mut ModuleResponder, request: &[u8]) -> Option<Vec<u8>> {
        let service = *request.first()?;
        Some(match service {
            0x22 => {
                let identifier = u16::from_be_bytes([*request.get(1)?, *request.get(2)?]);
                match module.identifiers.get(&identifier) {
                    Some(length) => {
                        let mut payload = vec![0x62, (identifier >> 8) as u8, identifier as u8];
                        if identifier == VIN_IDENTIFIER {
                            payload.extend_from_slice(vin.as_bytes());
                        } else {
                            payload.extend(synthetic_bytes(&module.family, identifier, *length));
                        }
                        payload
                    }
                    None => vec![0x7F, 0x22, 0x31],
                }
            }
            0x19 => match request.get(1) {
                Some(0x02) => {
                    let mut payload = vec![0x59, 0x02, 0xFF];
                    for fault in &module.faults {
                        payload.extend_from_slice(fault);
                        payload.push(FAULT_STATUS);
                    }
                    payload
                }
                _ => vec![0x7F, 0x19, 0x12],
            },
            0x09 => match request.get(1) {
                Some(0x04) => {
                    let mut payload = vec![0x49, 0x04, 0x01];
                    payload.extend_from_slice(BENCH_CALIBRATION_ID);
                    payload
                }
                _ => vec![0x7F, 0x09, 0x12],
            },
            0x3E => match request.get(1) {
                Some(sub) if sub & 0x80 != 0 => return None,
                _ => vec![0x7E, 0x00],
            },
            other => vec![0x7F, other, 0x11],
        })
    }

    fn reply(module: &ModuleResponder, route: BenchRoute, data: Vec<u8>) -> Option<CanFrame> {
        let id = if module.extended {
            CanId::extended(module.response_id).ok()?
        } else {
            CanId::standard(u16::try_from(module.response_id).ok()?).ok()?
        };
        CanFrame::new(0, route.as_str(), id, data).ok()
    }
}

impl BenchBus for BenchVehicle {
    fn on_frame(&mut self, route: BenchRoute, frame: &CanFrame) -> Vec<CanFrame> {
        let vin = self.vin.clone();
        let Some(module) = self
            .modules
            .iter_mut()
            .find(|module| module.route == route && module.request_id == frame.id.value())
        else {
            return Vec::new();
        };
        let Some(pci) = frame.data.first() else {
            return Vec::new();
        };
        match pci >> 4 {
            // Single frame: the whole request is here.
            0x0 => {
                let length = usize::from(pci & 0x0F);
                let Some(request) = frame.data.get(1..1 + length) else {
                    return Vec::new();
                };
                let Some(payload) = Self::answer(&vin, module, request) else {
                    return Vec::new();
                };
                let Ok(mut frames) = isotp::segment(&payload, Some(PADDING)) else {
                    return Vec::new();
                };
                if frames.is_empty() {
                    return Vec::new();
                }
                let first = frames.remove(0);
                module.pending = frames;
                Self::reply(module, route, first).into_iter().collect()
            }
            // Flow control from the tester: the consecutive frames follow.
            0x3 => {
                let pending = std::mem::take(&mut module.pending);
                let module = &*module;
                pending
                    .into_iter()
                    .filter_map(|data| Self::reply(module, route, data))
                    .collect()
            }
            // A multi-frame request: nothing the application sends is one.
            _ => Vec::new(),
        }
    }

    fn describe(&self) -> String {
        format!(
            "{}: {} module(s) answer on the bench; every value is synthetic",
            self.label,
            self.modules.len()
        )
    }
}

fn parse_id(text: &str) -> Option<u32> {
    let text = text.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u32::from_str_radix(hex, 16).ok()
}

fn parse_route(text: &str) -> Option<BenchRoute> {
    match text.trim() {
        "hs-can" => Some(BenchRoute::HsCan),
        "ms-can" => Some(BenchRoute::MsCan),
        _ => None,
    }
}

/// The seed a module's faults are drawn from: the scenario number mixed with
/// the family name, so two modules of one scenario draw differently and one
/// module draws the same on every run.
fn scenario_seed(scenario: u32, family: &str) -> u32 {
    let mut seed = 0x9E37_79B9u32 ^ scenario.wrapping_mul(2_654_435_761);
    for byte in family.bytes() {
        seed = seed.wrapping_mul(31).wrapping_add(u32::from(byte));
    }
    seed
}

/// The next draw. The low bits of a linear congruential sequence are the
/// weakest, so the value handed out is the sequence shifted past them.
fn next_draw(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed >> 8
}

/// The failure type a drawn code is reported with, read from the code's own
/// letter: the pairing the fixed list used, so the failure-type wording joins
/// as it did.
fn failure_type_for(code: &str) -> u8 {
    match code.as_bytes().first() {
        Some(b'U') => 0x87,
        Some(b'B' | b'C') => 0x11,
        _ => 0x00,
    }
}

/// The faults a module reports on this scenario. `SCENARIO_HEALTHY` leaves
/// every module quiet. Any other number gives the module none, one or two
/// codes drawn from those the library describes for its family — so every
/// code reported has wording, that being what it was drawn from — and the
/// same number always draws the same ones.
fn choose_faults(library: &KnowledgeLibrary, family: &str, scenario: u32) -> Vec<[u8; 3]> {
    if scenario == SCENARIO_HEALTHY {
        return Vec::new();
    }
    let mut seed = scenario_seed(scenario, family);
    let wanted = next_draw(&mut seed) as usize % (FAULTS_PER_MODULE + 1);
    if wanted == 0 {
        return Vec::new();
    }
    let codes = library.dtc_codes_for(family);
    let mut drawn: BTreeSet<usize> = BTreeSet::new();
    let mut faults: Vec<[u8; 3]> = Vec::new();
    for _ in 0..DRAW_ATTEMPTS {
        if codes.is_empty() || faults.len() == wanted {
            break;
        }
        let index = next_draw(&mut seed) as usize % codes.len();
        if !drawn.insert(index) {
            continue;
        }
        let code = codes[index];
        if let Some(bytes) = fault_bytes(code, failure_type_for(code)) {
            faults.push(bytes);
        }
    }
    // A library that describes nothing for this family still has to show that
    // the read path works, so the fixed list answers for it.
    if faults.is_empty() {
        faults = CANDIDATE_FAULTS
            .iter()
            .filter(|(code, failure_type)| {
                library
                    .describe_dtc(code, *failure_type, family)
                    .description
                    .is_some()
            })
            .take(wanted)
            .filter_map(|(code, failure_type)| fault_bytes(code, *failure_type))
            .collect();
    }
    if faults.is_empty() {
        faults.extend(fault_bytes(FALLBACK_FAULT.0, FALLBACK_FAULT.1));
    }
    faults
}

/// `P0300` with failure type `0x00` as the three bytes ISO 14229 carries:
/// the letter in the top two bits, the digits in the fourteen below.
fn fault_bytes(code: &str, failure_type: u8) -> Option<[u8; 3]> {
    let mut chars = code.chars();
    let letter = match chars.next()? {
        'P' => 0u16,
        'C' => 1,
        'B' => 2,
        'U' => 3,
        _ => return None,
    };
    let digits = u16::from_str_radix(chars.as_str(), 16).ok()?;
    if digits > 0x3FFF {
        return None;
    }
    let value = (letter << 14) | digits;
    Some([(value >> 8) as u8, value as u8, failure_type])
}

/// Deterministic bytes for an identifier's value, in the middle of the
/// range so the catalogue's scaling shows a plausible figure: the same
/// module and identifier always give the same bytes.
fn synthetic_bytes(family: &str, identifier: u16, length: usize) -> Vec<u8> {
    let mut seed: u32 = 0x9E37_79B9 ^ u32::from(identifier);
    for byte in family.bytes() {
        seed = seed.wrapping_mul(31).wrapping_add(u32::from(byte));
    }
    (0..length)
        .map(|index| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            0x40 + ((seed >> 16) as u8 % 0x40) + (index as u8 & 0x0F)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_codes_go_on_the_wire_as_iso_14229_writes_them() {
        assert_eq!(fault_bytes("P0300", 0x00), Some([0x03, 0x00, 0x00]));
        assert_eq!(fault_bytes("C0035", 0x11), Some([0x40, 0x35, 0x11]));
        assert_eq!(fault_bytes("B1A08", 0x11), Some([0x9A, 0x08, 0x11]));
        assert_eq!(fault_bytes("U0100", 0x87), Some([0xC1, 0x00, 0x87]));
        assert_eq!(fault_bytes("X0000", 0), None);
    }

    #[test]
    fn synthetic_bytes_are_deterministic_and_mid_range() {
        let a = synthetic_bytes("PCM", 0x1945, 8);
        let b = synthetic_bytes("PCM", 0x1945, 8);
        assert_eq!(a, b);
        assert_ne!(a, synthetic_bytes("TCM", 0x1945, 8));
        assert!(a.iter().all(|byte| (0x40..=0x8F).contains(byte)));
    }

    #[test]
    fn identifiers_and_routes_parse_as_the_survey_writes_them() {
        assert_eq!(parse_id("0x7E0"), Some(0x7E0));
        assert_eq!(parse_id("0x18DAF110"), Some(0x18DA_F110));
        assert_eq!(parse_route("ms-can"), Some(BenchRoute::MsCan));
        assert_eq!(parse_route("sub-most"), None);
    }
}
