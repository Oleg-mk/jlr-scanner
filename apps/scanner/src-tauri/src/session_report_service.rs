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

pub struct SessionReportService {
    started_unix_ms: u128,
    captures: Vec<Value>,
    module_reads: Vec<Value>,
    calibration_reads: Vec<Value>,
}

impl SessionReportService {
    pub fn new() -> Self {
        Self {
            started_unix_ms: unix_ms(),
            captures: Vec::new(),
            module_reads: Vec::new(),
            calibration_reads: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> SessionReportSnapshot {
        let total = self.captures.len() + self.module_reads.len() + self.calibration_reads.len();
        SessionReportSnapshot {
            captures: self.captures.len() as u32,
            module_reads: self.module_reads.len() as u32,
            calibration_reads: self.calibration_reads.len() as u32,
            report_available: total > 0,
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
            "session_started_unix_ms": self.started_unix_ms,
            "saved_unix_ms": unix_ms(),
            "validation": "session bundle; every item carries its own validation state and none is vehicle-confirmed by being here",
            "adapter": adapter,
            "library": library,
            "survey": survey,
            "captures": self.captures,
            "module_reads": self.module_reads,
            "calibration_reads": self.calibration_reads,
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
