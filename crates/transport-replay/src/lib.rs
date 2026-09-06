//! Machine-readable, deterministic raw CAN replay source.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use transport_api::{CanFrame, CanFrameSource, CanId, CanSourceError, CanSourceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackMode {
    Deterministic,
    RealTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixtureClass {
    Synthetic,
    Documented,
    Captured,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MetadataEnvelope {
    pub vehicle_program: Option<String>,
    pub model_year: Option<u16>,
    pub model_year_from: Option<u16>,
    pub model_year_to: Option<u16>,
    pub architecture_generation: Option<String>,
    pub ecu_family: Option<String>,
    pub powertrain: Option<String>,
    pub variant: Option<String>,
    pub market: Option<String>,
    pub diagnostic_implementation: Option<String>,
    pub evidence: Option<String>,
    pub validation: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplayFrame {
    pub timestamp_us: u64,
    pub route: String,
    pub id: u32,
    pub extended: bool,
    pub dlc: u8,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplayFixture {
    pub schema_version: u32,
    pub fixture_class: FixtureClass,
    pub name: String,
    #[serde(default)]
    pub metadata: MetadataEnvelope,
    pub frames: Vec<ReplayFrame>,
}

pub struct ReplaySource {
    fixture: ReplayFixture,
    frames: Vec<CanFrame>,
    cursor: usize,
    mode: PlaybackMode,
    wall_start: Option<Instant>,
    fixture_start_us: u64,
}

impl ReplaySource {
    pub fn from_json(source: &str, mode: PlaybackMode) -> Result<Self, CanSourceError> {
        let fixture: ReplayFixture = serde_json::from_str(source)
            .map_err(|error| CanSourceError::InvalidData(error.to_string()))?;
        Self::from_fixture(fixture, mode)
    }

    pub fn from_path(path: impl AsRef<Path>, mode: PlaybackMode) -> Result<Self, CanSourceError> {
        let source =
            fs::read_to_string(path).map_err(|error| CanSourceError::Io(error.to_string()))?;
        Self::from_json(&source, mode)
    }

    pub fn from_fixture(
        fixture: ReplayFixture,
        mode: PlaybackMode,
    ) -> Result<Self, CanSourceError> {
        if fixture.schema_version != 1 {
            return Err(CanSourceError::InvalidData(
                "unsupported replay schema version".into(),
            ));
        }
        if fixture.name.trim().is_empty() {
            return Err(CanSourceError::InvalidData(
                "fixture name must not be empty".into(),
            ));
        }
        if fixture.fixture_class != FixtureClass::Synthetic
            && (fixture
                .metadata
                .evidence
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
                || fixture
                    .metadata
                    .validation
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty())
        {
            return Err(CanSourceError::InvalidData(
                "documented and captured fixtures require evidence and validation".into(),
            ));
        }

        let mut prior_timestamp = None;
        let mut frames = Vec::with_capacity(fixture.frames.len());
        for raw in &fixture.frames {
            if raw.dlc as usize != raw.data.len() {
                return Err(CanSourceError::InvalidData(
                    "DLC does not match data length".into(),
                ));
            }
            if prior_timestamp.is_some_and(|prior| raw.timestamp_us < prior) {
                return Err(CanSourceError::InvalidData(
                    "timestamps must be monotonic".into(),
                ));
            }
            prior_timestamp = Some(raw.timestamp_us);
            let id = if raw.extended {
                CanId::extended(raw.id)
            } else if raw.id <= 0x7ff {
                CanId::standard(raw.id as u16)
            } else {
                return Err(CanSourceError::InvalidData(format!(
                    "standard CAN ID exceeds 0x7ff: {:#x}",
                    raw.id
                )));
            }
            .map_err(|error| CanSourceError::InvalidData(error.to_string()))?;
            frames.push(
                CanFrame::new(raw.timestamp_us, raw.route.clone(), id, raw.data.clone())
                    .map_err(|error| CanSourceError::InvalidData(error.to_string()))?,
            );
        }
        let fixture_start_us = frames.first().map_or(0, |frame| frame.timestamp_us);
        Ok(Self {
            fixture,
            frames,
            cursor: 0,
            mode,
            wall_start: None,
            fixture_start_us,
        })
    }

    pub fn fixture(&self) -> &ReplayFixture {
        &self.fixture
    }
}

impl CanFrameSource for ReplaySource {
    fn source_kind(&self) -> CanSourceKind {
        CanSourceKind::Replay
    }

    fn next_frame(&mut self) -> Result<Option<CanFrame>, CanSourceError> {
        let Some(frame) = self.frames.get(self.cursor).cloned() else {
            return Ok(None);
        };
        if self.mode == PlaybackMode::RealTime {
            let start = *self.wall_start.get_or_insert_with(Instant::now);
            let target =
                Duration::from_micros(frame.timestamp_us.saturating_sub(self.fixture_start_us));
            if let Some(remaining) = target.checked_sub(start.elapsed()) {
                thread::sleep(remaining);
            }
        }
        self.cursor += 1;
        Ok(Some(frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
      "schema_version": 1,
      "fixture_class": "synthetic",
      "name": "generic",
      "frames": [
        {"timestamp_us": 10, "route": "can-a", "id": 291, "extended": false, "dlc": 2, "data": [1, 2]}
      ]
    }"#;

    #[test]
    fn deterministic_replay_reaches_end_of_stream() {
        let mut replay = ReplaySource::from_json(VALID, PlaybackMode::Deterministic).unwrap();
        assert_eq!(replay.source_kind(), CanSourceKind::Replay);
        assert_eq!(replay.next_frame().unwrap().unwrap().timestamp_us, 10);
        assert_eq!(replay.next_frame().unwrap(), None);
    }

    #[test]
    fn rejects_dlc_mismatch_and_non_monotonic_time() {
        assert!(ReplaySource::from_json(
            &VALID.replace("\"dlc\": 2", "\"dlc\": 1"),
            PlaybackMode::Deterministic
        )
        .is_err());
        let reversed = VALID.replace(
            "]\n    }",
            ", {\"timestamp_us\": 1, \"route\": \"can-a\", \"id\": 291, \"extended\": false, \"dlc\": 1, \"data\": [1]}]\n    }",
        );
        assert!(ReplaySource::from_json(&reversed, PlaybackMode::Deterministic).is_err());
    }

    #[test]
    fn captured_data_requires_provenance() {
        let captured = VALID.replace("synthetic", "captured");
        assert!(ReplaySource::from_json(&captured, PlaybackMode::Deterministic).is_err());
    }

    #[test]
    fn accepts_extended_identifier_and_real_time_mode() {
        let extended = VALID
            .replace("\"id\": 291", "\"id\": 419385573")
            .replace("\"extended\": false", "\"extended\": true");
        let mut replay = ReplaySource::from_json(&extended, PlaybackMode::RealTime).unwrap();
        let frame = replay.next_frame().unwrap().unwrap();
        assert!(frame.id.is_extended());
        assert_eq!(frame.id.value(), 0x18ff_50e5);
    }
}
