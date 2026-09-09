//! Listen-only bus capture for the shell: turns a `RouteCapture` from the
//! adapter into a snapshot the UI can show and a `captured` replay fixture the
//! tester can save. The verdict states exactly what listening can and cannot
//! establish.

use app_contracts::{
    AdapterInfo, CaptureIdCount, CaptureSnapshot, CaptureState, VehicleContextInput,
};
use mongoose_jlr::{CanIdFormat, RouteCapture};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use transport_replay::{FixtureClass, MetadataEnvelope, ReplayFixture, ReplayFrame};

/// Identifiers listed in the snapshot, most frequent first.
const TOP_IDENTIFIERS: usize = 12;

struct Stored {
    snapshot: CaptureSnapshot,
    fixture: ReplayFixture,
}

#[derive(Default)]
pub struct CaptureService {
    last: Option<Stored>,
}

impl CaptureService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> CaptureSnapshot {
        self.last
            .as_ref()
            .map(|stored| stored.snapshot.clone())
            .unwrap_or_else(idle)
    }

    pub fn record(
        &mut self,
        capture: &RouteCapture,
        requested: Duration,
        context: &VehicleContextInput,
        adapter: &AdapterInfo,
    ) -> CaptureSnapshot {
        let snapshot = summarise(capture, requested);
        let fixture = fixture_from(capture, context, adapter);
        self.last = Some(Stored {
            snapshot: snapshot.clone(),
            fixture,
        });
        snapshot
    }

    /// The last capture was heard on the bench (ADR-0020): say so in the
    /// snapshot and in the fixture, which becomes synthetic and stops being
    /// vehicle evidence of any kind.
    pub fn mark_synthetic(&mut self) -> CaptureSnapshot {
        if let Some(stored) = self.last.as_mut() {
            stored.snapshot.synthetic = true;
            stored.fixture.fixture_class = FixtureClass::Synthetic;
            stored.fixture.name = format!("bench-{}", stored.fixture.name);
            stored.fixture.metadata.validation =
                Some("synthetic_bench_capture_not_vehicle_evidence".into());
        }
        self.snapshot()
    }

    pub fn record_failure(&mut self, route_id: &str, message: String) -> CaptureSnapshot {
        let snapshot = CaptureSnapshot {
            state: CaptureState::Failed,
            route_id: route_id.to_string(),
            error: Some(message),
            ..idle()
        };
        self.last = None;
        snapshot
    }

    pub fn capture_json(&self) -> Result<String, String> {
        let stored = self
            .last
            .as_ref()
            .ok_or_else(|| "no capture has completed".to_string())?;
        serde_json::to_string_pretty(&stored.fixture).map_err(|error| error.to_string())
    }
}

fn idle() -> CaptureSnapshot {
    CaptureSnapshot {
        state: CaptureState::Idle,
        route_id: String::new(),
        pins: String::new(),
        bitrate_bps: None,
        requested_seconds: 0,
        listened_ms: 0,
        frames: 0,
        frames_per_second: 0,
        distinct_ids: 0,
        standard_frames: 0,
        extended_frames: 0,
        dropped_frames: 0,
        truncated: false,
        top_ids: Vec::new(),
        verdict: String::new(),
        error: None,
        capture_available: false,
        synthetic: false,
    }
}

fn pins_text(pins: &[u8]) -> String {
    pins.iter().map(u8::to_string).collect::<Vec<_>>().join("/")
}

/// Reduce a capture to what the UI shows. Counts only; nothing is inferred
/// about which vehicle bus was heard.
pub fn summarise(capture: &RouteCapture, requested: Duration) -> CaptureSnapshot {
    let route_id = capture.route.id.as_str().to_string();
    let pins = pins_text(capture.route.obd_pins);
    let bitrate = capture.route.bitrate;
    let listened_ms = u64::try_from(capture.listened.as_millis()).unwrap_or(u64::MAX);

    let mut counts: BTreeMap<(bool, u32), u32> = BTreeMap::new();
    let mut standard = 0u32;
    let mut extended = 0u32;
    for captured in &capture.frames {
        let is_extended = captured.frame.id_format == CanIdFormat::Extended;
        if is_extended {
            extended += 1;
        } else {
            standard += 1;
        }
        *counts
            .entry((is_extended, captured.frame.arbitration_id))
            .or_default() += 1;
    }
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    let top_ids = ranked
        .iter()
        .take(TOP_IDENTIFIERS)
        .map(|((is_extended, id), count)| CaptureIdCount {
            id: if *is_extended {
                format!("0x{id:08X}")
            } else {
                format!("0x{id:03X}")
            },
            extended: *is_extended,
            count: *count,
        })
        .collect();

    let frames = u32::try_from(capture.frames.len()).unwrap_or(u32::MAX);
    let seconds = capture.listened.as_secs_f64();
    let frames_per_second = if seconds > 0.0 {
        (f64::from(frames) / seconds).round() as u32
    } else {
        0
    };
    let distinct_ids = u32::try_from(ranked.len()).unwrap_or(u32::MAX);
    let rate_text = bitrate
        .map(|value| format!("{} kbit/s", value / 1000))
        .unwrap_or_else(|| "the route's rate".to_string());

    let mut verdict = if frames > 0 {
        format!(
            "Traffic present on {route_id} (pins {pins}) at {rate_text}: about {frames_per_second} frames/s, {distinct_ids} distinct identifiers. A live bus is on this pair. Which of the vehicle's buses it is cannot be told from listening alone."
        )
    } else {
        format!(
            "No frames heard on {route_id} (pins {pins}) at {rate_text} in {:.1} s. Either this pair is silent at that rate — a diagnostic-only CAN behind a gateway carries nothing until a tester speaks — or the rate does not match, or nothing is connected. Listening alone cannot tell these apart.",
            seconds
        )
    };
    if capture.truncated {
        verdict.push_str(" The frame cap was reached before the time was up; the counts describe the captured part only.");
    }
    if capture.dropped_frames > 0 {
        verdict.push_str(&format!(
            " {} non-data packets from the adapter were dropped.",
            capture.dropped_frames
        ));
    }

    CaptureSnapshot {
        state: CaptureState::Completed,
        route_id,
        pins,
        bitrate_bps: bitrate,
        requested_seconds: u32::try_from(requested.as_secs()).unwrap_or(u32::MAX),
        listened_ms,
        frames,
        frames_per_second,
        distinct_ids,
        standard_frames: standard,
        extended_frames: extended,
        dropped_frames: capture.dropped_frames,
        truncated: capture.truncated,
        top_ids,
        verdict,
        error: None,
        capture_available: true,
        synthetic: false,
    }
}

/// The artifact: a `captured` replay fixture with the provenance a reviewer
/// needs — route, pins, rate, adapter identity, how the vehicle was described —
/// and a validation tag that says what it is not.
fn fixture_from(
    capture: &RouteCapture,
    context: &VehicleContextInput,
    adapter: &AdapterInfo,
) -> ReplayFixture {
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    let route_id = capture.route.id.as_str();
    let text = |value: &str| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    };
    let optional = |value: &Option<String>| value.as_deref().and_then(text);
    ReplayFixture {
        schema_version: 1,
        fixture_class: FixtureClass::Captured,
        name: format!("capture-{route_id}-{unix_ms}"),
        metadata: MetadataEnvelope {
            vehicle_program: text(&context.vehicle_program),
            model_year: context.model_year,
            powertrain: optional(&context.powertrain),
            variant: optional(&context.variant),
            market: optional(&context.market),
            evidence: Some(format!(
                "listen-only capture on route {route_id}, J1962 pins {}, {} bit/s, via {} serial {}; SDD breakpoint marker {}; no frame was transmitted or acknowledged",
                pins_text(capture.route.obd_pins),
                capture
                    .route
                    .bitrate
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                adapter.name,
                adapter.serial_number.as_deref().unwrap_or("unknown"),
                optional(&context.year_breakpoint).unwrap_or_else(|| "not stated".into())
            )),
            validation: Some("captured_listen_only_bus_activity_not_module_confirmed".into()),
            ..MetadataEnvelope::default()
        },
        frames: capture
            .frames
            .iter()
            .map(|captured| ReplayFrame {
                timestamp_us: captured.host_offset_us,
                route: route_id.to_string(),
                id: captured.frame.arbitration_id,
                extended: captured.frame.id_format == CanIdFormat::Extended,
                dlc: captured.frame.dlc,
                data: captured.frame.payload().to_vec(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_contracts::BoardInfoEvidence;
    use mongoose_jlr::{list_vehicle_routes, CapturedFrame, RawCanFrame, VehicleRouteId};
    use transport_replay::{PlaybackMode, ReplaySource};

    fn frame(id: u32, extended: bool, data: &[u8], offset_us: u64) -> CapturedFrame {
        let mut bytes = [0u8; 8];
        bytes[..data.len()].copy_from_slice(data);
        CapturedFrame {
            frame: RawCanFrame {
                arbitration_id: id,
                id_format: if extended {
                    CanIdFormat::Extended
                } else {
                    CanIdFormat::Standard
                },
                dlc: data.len() as u8,
                data: bytes,
                device_timestamp: (offset_us / 1000) as u32,
                source_route: VehicleRouteId::HsCan,
                source_network: "test",
            },
            host_offset_us: offset_us,
        }
    }

    fn capture(frames: Vec<CapturedFrame>) -> RouteCapture {
        RouteCapture {
            route: list_vehicle_routes()[0],
            frames,
            listened: Duration::from_millis(2000),
            dropped_frames: 0,
            truncated: false,
        }
    }

    fn adapter() -> AdapterInfo {
        AdapterInfo {
            name: "MongoosePro JLR".into(),
            connection_status: "Connected".into(),
            port: "COM7".into(),
            usb_vid: 0x18E1,
            usb_pid: 0x0104,
            serial_number: Some("SERIAL-1".into()),
            transport: "USB CDC / Serial".into(),
            driver: Some("usbser".into()),
            backend: "mongoose-jlr".into(),
            board_info: BoardInfoEvidence {
                response_command: "0x8109".into(),
                raw_response_hex: "AA BB".into(),
            },
        }
    }

    fn context() -> VehicleContextInput {
        VehicleContextInput {
            vehicle_program: "L405".into(),
            model_year: Some(2014),
            year_breakpoint: Some("MY14".into()),
            ..VehicleContextInput::default()
        }
    }

    #[test]
    fn a_busy_pair_is_summarised_by_counts_and_the_verdict_claims_no_bus_identity() {
        let frames = vec![
            frame(0x321, false, &[1, 2, 3], 1_000),
            frame(0x321, false, &[1, 2, 4], 501_000),
            frame(0x18DAF110, true, &[0x03, 0x62, 0x19, 0x45], 900_000),
            frame(0x7E8, false, &[0x06], 1_500_000),
        ];
        let snapshot = summarise(&capture(frames), Duration::from_secs(2));

        assert_eq!(snapshot.state, CaptureState::Completed);
        assert_eq!(snapshot.route_id, "hs-can");
        assert_eq!(snapshot.pins, "6/14");
        assert_eq!(snapshot.bitrate_bps, Some(500_000));
        assert_eq!(snapshot.frames, 4);
        assert_eq!(snapshot.frames_per_second, 2);
        assert_eq!(snapshot.distinct_ids, 3);
        assert_eq!(snapshot.standard_frames, 3);
        assert_eq!(snapshot.extended_frames, 1);
        assert_eq!(snapshot.top_ids[0].id, "0x321");
        assert_eq!(snapshot.top_ids[0].count, 2);
        assert!(snapshot
            .top_ids
            .iter()
            .any(|entry| entry.id == "0x18DAF110" && entry.extended));
        assert!(snapshot
            .verdict
            .starts_with("Traffic present on hs-can (pins 6/14) at 500 kbit/s"));
        assert!(snapshot
            .verdict
            .contains("cannot be told from listening alone"));
        assert!(snapshot.capture_available);
    }

    #[test]
    fn a_silent_pair_is_named_as_undecidable_not_as_empty() {
        let snapshot = summarise(&capture(Vec::new()), Duration::from_secs(2));
        assert_eq!(snapshot.frames, 0);
        assert!(snapshot
            .verdict
            .starts_with("No frames heard on hs-can (pins 6/14)"));
        assert!(snapshot
            .verdict
            .contains("Listening alone cannot tell these apart"));
        assert!(snapshot.capture_available);
    }

    #[test]
    fn the_saved_artifact_is_a_captured_fixture_that_replays() {
        let mut service = CaptureService::new();
        assert_eq!(service.snapshot().state, CaptureState::Idle);
        assert!(service.capture_json().is_err());

        let frames = vec![
            frame(0x321, false, &[1, 2, 3], 1_000),
            frame(0x18DAF110, true, &[0x03, 0x62], 2_000),
        ];
        service.record(
            &capture(frames),
            Duration::from_secs(2),
            &context(),
            &adapter(),
        );
        let json = service.capture_json().unwrap();
        let mut source = ReplaySource::from_json(&json, PlaybackMode::Deterministic).unwrap();
        assert_eq!(source.fixture().fixture_class, FixtureClass::Captured);
        assert!(source.fixture().name.starts_with("capture-hs-can-"));
        let metadata = &source.fixture().metadata;
        assert_eq!(metadata.vehicle_program.as_deref(), Some("L405"));
        assert_eq!(metadata.model_year, Some(2014));
        assert!(metadata
            .evidence
            .as_deref()
            .unwrap()
            .contains("no frame was transmitted"));
        assert!(metadata
            .validation
            .as_deref()
            .unwrap()
            .contains("not_module_confirmed"));

        use transport_api::CanFrameSource;
        let first = source.next_frame().unwrap().unwrap();
        assert_eq!(first.route, "hs-can");
        assert_eq!(first.id.value(), 0x321);
        let second = source.next_frame().unwrap().unwrap();
        assert_eq!(second.id.value(), 0x18DAF110);
        assert!(source.next_frame().unwrap().is_none());
    }

    #[test]
    fn a_failure_replaces_the_last_capture() {
        let mut service = CaptureService::new();
        service.record(
            &capture(vec![frame(0x321, false, &[1], 1_000)]),
            Duration::from_secs(1),
            &context(),
            &adapter(),
        );
        let failed = service.record_failure("ms-can", "adapter disconnected".into());
        assert_eq!(failed.state, CaptureState::Failed);
        assert_eq!(failed.route_id, "ms-can");
        assert_eq!(failed.error.as_deref(), Some("adapter disconnected"));
        assert!(!failed.capture_available);
        assert!(service.capture_json().is_err());
    }
}
