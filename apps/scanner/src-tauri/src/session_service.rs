//! F10 session state for the shell: the loaded data library and the survey.
//!
//! Thin by design. Everything it does lives in `diagnostic-session`, which is
//! tested without Tauri; this file only holds the state between commands.

use app_contracts::{
    LibrarySnapshot, VehicleCatalogueSnapshot, VehicleContextInput, VehicleSurveySnapshot,
    VinDecodeSnapshot,
};
use diagnostic_session::KnowledgeLibrary;
use std::path::Path;

pub struct SessionService {
    library: KnowledgeLibrary,
    last_survey: Option<VehicleSurveySnapshot>,
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            library: KnowledgeLibrary::built_in(),
            last_survey: None,
        }
    }

    pub fn library(&self) -> &KnowledgeLibrary {
        &self.library
    }

    pub fn library_snapshot(&self) -> LibrarySnapshot {
        self.library.snapshot().clone()
    }

    pub fn catalogue(&self) -> VehicleCatalogueSnapshot {
        self.library.catalogue().clone()
    }

    pub fn decode_vin(&self, vin: &str) -> VinDecodeSnapshot {
        self.library.decode_vin(vin)
    }

    /// Replace the library with the built-in manifests plus the directory's.
    /// A directory that cannot be read yields a `Failed` snapshot that says
    /// so; the previous library is not kept, because a stale library shown as
    /// current would be a lie.
    pub fn load_directory(&mut self, directory: &str) -> LibrarySnapshot {
        // The application's loader: a folder loads only under a valid,
        // signed, unexpired issue stamp (ADR-0019).
        self.library = KnowledgeLibrary::load_issued_directory(Path::new(directory.trim()));
        // A survey answered by the previous library must not outlive it.
        self.last_survey = None;
        self.library_snapshot()
    }

    pub fn survey(&mut self, context: &VehicleContextInput) -> VehicleSurveySnapshot {
        let survey = self.library.survey(context);
        self.last_survey = Some(survey.clone());
        survey
    }

    /// Forget the survey of a session that ended; the library stays.
    pub fn clear_session(&mut self) {
        self.last_survey = None;
    }

    /// The survey the session report bundles: the last one answered by the
    /// current library.
    pub fn last_survey(&self) -> Option<VehicleSurveySnapshot> {
        self.last_survey.clone()
    }
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_contracts::LibraryState;

    fn documented_directory() -> String {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/knowledge/documented"
        )
        .to_string()
    }

    #[test]
    fn starts_with_built_in_data_and_refuses_an_unstamped_directory() {
        let mut service = SessionService::new();
        let initial = service.library_snapshot();
        assert_eq!(initial.state, LibraryState::NotLoaded);
        assert_eq!(initial.sources, 5);

        // The documented fixture directory carries no issue stamp, so the
        // application refuses it (ADR-0019) and keeps its built-in data.
        let loaded = service.load_directory(&documented_directory());
        assert_eq!(loaded.state, LibraryState::Failed);
        assert_eq!(
            loaded.issue.as_ref().map(|issue| issue.integrity),
            Some(app_contracts::LibraryIssueIntegrity::NoStamp)
        );
        assert!(loaded.message.contains("no issue stamp"));
        assert_eq!(loaded.sources, 5);
        assert!(loaded.directory.is_some());
    }

    #[test]
    fn a_missing_directory_is_a_failed_library_not_a_silent_empty_one() {
        let mut service = SessionService::new();
        let snapshot = service.load_directory("/definitely/not/a/directory");
        assert_eq!(snapshot.state, LibraryState::Failed);
        assert!(snapshot.message.starts_with("Cannot read"));
        // Built-in data survives so the adapter bindings are still known.
        assert_eq!(snapshot.sources, 5);
    }

    #[test]
    fn a_survey_without_vehicle_data_names_the_gap() {
        let mut service = SessionService::new();
        let survey = service.survey(&VehicleContextInput {
            vehicle_program: "X250".into(),
            model_year: Some(2010),
            ..VehicleContextInput::default()
        });
        assert!(survey.modules.is_empty());
        assert!(survey.message.contains("No modules are known"));
    }
}
