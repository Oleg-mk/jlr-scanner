use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnowledgeError {
    InvalidId(String),
    InvalidSource(String),
    InvalidFingerprint(String),
    InvalidEvidence(String),
    InvalidApplicability(String),
    InvalidRecord(String),
    MissingSource(String),
    MissingEvidence(String),
    DuplicateConflict { kind: &'static str, id: String },
    ValidationViolation(String),
    UnsupportedSchema(u32),
    Parse(String),
    Io(String),
}

impl fmt::Display for KnowledgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(value) => write!(formatter, "invalid stable ID: {value}"),
            Self::InvalidSource(message) => write!(formatter, "invalid source: {message}"),
            Self::InvalidFingerprint(message) => {
                write!(formatter, "invalid content fingerprint: {message}")
            }
            Self::InvalidEvidence(message) => write!(formatter, "invalid evidence: {message}"),
            Self::InvalidApplicability(message) => {
                write!(formatter, "invalid applicability: {message}")
            }
            Self::InvalidRecord(message) => {
                write!(formatter, "invalid knowledge record: {message}")
            }
            Self::MissingSource(id) => {
                write!(formatter, "evidence references missing source: {id}")
            }
            Self::MissingEvidence(id) => {
                write!(formatter, "record references missing evidence: {id}")
            }
            Self::DuplicateConflict { kind, id } => {
                write!(
                    formatter,
                    "{kind} ID already exists with different content: {id}"
                )
            }
            Self::ValidationViolation(message) => {
                write!(formatter, "validation-state violation: {message}")
            }
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported knowledge schema version: {version}")
            }
            Self::Parse(message) => write!(formatter, "knowledge ingestion parse error: {message}"),
            Self::Io(message) => write!(formatter, "knowledge source I/O error: {message}"),
        }
    }
}

impl std::error::Error for KnowledgeError {}
