//! One report for the whole session (F11, milestone M1).
//!
//! Every capture, module read and calibration read the shell records is kept
//! here as the JSON its own service produced, and saved together with the
//! library, the survey and the adapter as they are at the moment of saving.
//! Nothing is re-interpreted: each item carries its own validation state, and
//! the bundle says so about itself.

use app_contracts::{
    AdapterSnapshot, LibrarySnapshot, SessionReportSnapshot, VehicleSurveySnapshot,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SESSION_REPORT_SCHEMA: &str = "jlr-scanner.session-report";
pub const SESSION_REPORT_SCHEMA_VERSION: u32 = 1;

/// A session is either on the bench or on a real adapter, never both.
pub const SESSION_MODE_BENCH: &str = "bench";
pub const SESSION_MODE_REAL: &str = "real";

pub struct SessionReportService {
    started_unix_ms: u128,
    captures: Vec<Value>,
    module_reads: Vec<Value>,
    calibration_reads: Vec<Value>,
    /// Legislated OBD-II reads (ADR-0022, decision 7).
    standard_obd_reads: Vec<Value>,
    /// Live read runs, each with its own series (ADR-0022).
    live_read_runs: Vec<Value>,
    /// Mileage surveys: every module's answer side by side (ADR-0024).
    mileage_surveys: Vec<Value>,
    /// Module passports: what each module says it is (ADR-0027).
    module_passports: Vec<Value>,
    /// Configuration reads: the car's configuration as its modules hold it
    /// (ADR-0028).
    ccf_reads: Vec<Value>,
    mode: Option<String>,
    /// The bench scenario this session was connected on (ADR-0020), so the
    /// bundle can say which picture it holds even after the bench is gone.
    bench_scenario: Option<u32>,
}

impl SessionReportService {
    pub fn new() -> Self {
        Self {
            started_unix_ms: unix_ms(),
            captures: Vec::new(),
            module_reads: Vec::new(),
            calibration_reads: Vec::new(),
            standard_obd_reads: Vec::new(),
            live_read_runs: Vec::new(),
            mileage_surveys: Vec::new(),
            module_passports: Vec::new(),
            ccf_reads: Vec::new(),
            mode: None,
            bench_scenario: None,
        }
    }

    /// Whether a record of `wanted` kind may join this session: yes while
    /// it is empty or already of that kind (ADR-0020).
    pub fn accepts(&self, wanted: &str) -> bool {
        !self.snapshot().report_available || self.mode.as_deref() == Some(wanted)
    }

    pub fn set_mode(&mut self, mode: &str) {
        self.mode = Some(mode.to_string());
        if mode != SESSION_MODE_BENCH {
            self.bench_scenario = None;
        }
    }

    /// The scenario the bench answered on, kept for the bundle.
    pub fn set_bench_scenario(&mut self, scenario: u32) {
        self.bench_scenario = Some(scenario);
    }

    /// Whether this session is on the bench, in which case nothing it holds
    /// may be written to disk.
    pub fn is_bench(&self) -> bool {
        self.mode.as_deref() == Some(SESSION_MODE_BENCH)
    }

    pub fn snapshot(&self) -> SessionReportSnapshot {
        let total = self.captures.len()
            + self.module_reads.len()
            + self.calibration_reads.len()
            + self.standard_obd_reads.len()
            + self.live_read_runs.len()
            + self.mileage_surveys.len()
            + self.module_passports.len()
            + self.ccf_reads.len();
        SessionReportSnapshot {
            captures: self.captures.len() as u32,
            module_reads: self.module_reads.len() as u32,
            calibration_reads: self.calibration_reads.len() as u32,
            standard_obd_reads: self.standard_obd_reads.len() as u32,
            live_read_runs: self.live_read_runs.len() as u32,
            mileage_surveys: self.mileage_surveys.len() as u32,
            module_passports: self.module_passports.len() as u32,
            ccf_reads: self.ccf_reads.len() as u32,
            report_available: total > 0,
            mode: self.mode.clone(),
        }
    }

    pub fn add_capture(&mut self, json: &str) -> Result<(), String> {
        self.captures.push(parse(json)?);
        Ok(())
    }

    pub fn add_module_read(&mut self, json: &str) -> Result<(), String> {
        self.module_reads.push(parse(json)?);
        Ok(())
    }

    pub fn add_standard_obd_read(&mut self, json: &str) -> Result<(), String> {
        self.standard_obd_reads.push(parse(json)?);
        Ok(())
    }

    /// One whole run of live reading, series and all (ADR-0022).
    pub fn add_live_read_run(&mut self, json: &str) -> Result<(), String> {
        self.live_read_runs.push(parse(json)?);
        Ok(())
    }

    /// One whole mileage survey, every module's answer (ADR-0024).
    pub fn add_mileage_survey(&mut self, json: &str) -> Result<(), String> {
        self.mileage_surveys.push(parse(json)?);
        Ok(())
    }

    /// One whole module passport run, every identifier's text (ADR-0027).
    pub fn add_module_passport(&mut self, json: &str) -> Result<(), String> {
        self.module_passports.push(parse(json)?);
        Ok(())
    }

    /// One whole configuration read, every block of every module (ADR-0028).
    pub fn add_ccf_read(&mut self, json: &str) -> Result<(), String> {
        self.ccf_reads.push(parse(json)?);
        Ok(())
    }

    pub fn add_calibration_read(&mut self, json: &str) -> Result<(), String> {
        self.calibration_reads.push(parse(json)?);
        Ok(())
    }

    /// The whole session as one JSON document, or why there is none yet.
    pub fn report_json(
        &self,
        adapter: &AdapterSnapshot,
        library: &LibrarySnapshot,
        survey: Option<&VehicleSurveySnapshot>,
    ) -> Result<String, String> {
        if !self.snapshot().report_available {
            return Err("Nothing has been recorded in this session yet.".into());
        }
        let bundle = json!({
            "schema": SESSION_REPORT_SCHEMA,
            "schema_version": SESSION_REPORT_SCHEMA_VERSION,
            "application_version": env!("CARGO_PKG_VERSION"),
            "application_build": build_id(),
            "session_mode": self.mode.as_deref().unwrap_or(SESSION_MODE_REAL),
            // Which picture the bench painted: null for a real session, and
            // for a bench one the number that reproduces exactly these codes
            // from the same library.
            "bench_scenario": self.bench_scenario,
            "session_started_unix_ms": self.started_unix_ms,
            "saved_unix_ms": unix_ms(),
            "validation": "session bundle; every item carries its own validation state and none is vehicle-confirmed by being here",
            "adapter": adapter,
            "library": library,
            "survey": survey,
            "captures": self.captures,
            "module_reads": self.module_reads,
            "calibration_reads": self.calibration_reads,
            "standard_obd_reads": self.standard_obd_reads,
            "live_read_runs": self.live_read_runs,
            "mileage_surveys": self.mileage_surveys,
            "module_passports": self.module_passports,
            "ccf_reads": self.ccf_reads,
        });
        serde_json::to_string_pretty(&bundle).map_err(|error| error.to_string())
    }
}

impl Default for SessionReportService {
    fn default() -> Self {
        Self::new()
    }
}

fn parse(json: &str) -> Result<Value, String> {
    serde_json::from_str(json).map_err(|error| format!("report is not JSON: {error}"))
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

/// The commit CI built, seven characters, or "local" for a build made by
/// hand; set through `JLR_BUILD_SHA` at compile time. With the version it
/// names the build a report came from.
fn build_id() -> &'static str {
    match option_env!("JLR_BUILD_SHA") {
        Some(sha) if sha.len() >= 7 => &sha[..7],
        Some(sha) => sha,
        None => "local",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_contracts::AdapterSnapshot;

    #[test]
    fn an_empty_session_has_no_report_and_says_so() {
        let service = SessionReportService::new();
        assert!(!service.snapshot().report_available);
        let error = service
            .report_json(
                &AdapterSnapshot::default(),
                &LibrarySnapshot::default(),
                None,
            )
            .unwrap_err();
        assert!(error.contains("Nothing has been recorded"));
    }

    #[test]
    fn recorded_items_are_counted_and_bundled_verbatim() {
        let mut service = SessionReportService::new();
        service
            .add_capture(r#"{"name":"cap-1","frames":3}"#)
            .unwrap();
        service
            .add_module_read(r#"{"session":"read-1","dtcs":[]}"#)
            .unwrap();
        assert!(service.add_calibration_read("not json").is_err());

        let snapshot = service.snapshot();
        assert_eq!(snapshot.captures, 1);
        assert_eq!(snapshot.module_reads, 1);
        assert_eq!(snapshot.calibration_reads, 0);
        assert!(snapshot.report_available);

        let json = service
            .report_json(
                &AdapterSnapshot::default(),
                &LibrarySnapshot::default(),
                None,
            )
            .unwrap();
        let bundle: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(bundle["schema"], SESSION_REPORT_SCHEMA);
        assert_eq!(bundle["captures"][0]["name"], "cap-1");
        assert_eq!(bundle["module_reads"][0]["session"], "read-1");
        assert!(bundle["survey"].is_null());
        assert!(bundle["validation"]
            .as_str()
            .unwrap()
            .contains("none is vehicle-confirmed"));
    }
}
