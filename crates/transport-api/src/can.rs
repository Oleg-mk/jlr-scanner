use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanId {
    Standard(u16),
    Extended(u32),
}

impl CanId {
    pub fn standard(value: u16) -> Result<Self, CanFrameError> {
        (value <= 0x7ff)
            .then_some(Self::Standard(value))
            .ok_or(CanFrameError::InvalidStandardId(value))
    }

    pub fn extended(value: u32) -> Result<Self, CanFrameError> {
        (value <= 0x1fff_ffff)
            .then_some(Self::Extended(value))
            .ok_or(CanFrameError::InvalidExtendedId(value))
    }

    pub fn value(self) -> u32 {
        match self {
            Self::Standard(value) => value.into(),
            Self::Extended(value) => value,
        }
    }

    pub fn is_extended(self) -> bool {
        matches!(self, Self::Extended(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanFrame {
    pub timestamp_us: u64,
    pub route: String,
    pub id: CanId,
    pub data: Vec<u8>,
}

impl CanFrame {
    pub fn new(
        timestamp_us: u64,
        route: impl Into<String>,
        id: CanId,
        data: impl Into<Vec<u8>>,
    ) -> Result<Self, CanFrameError> {
        let route = route.into();
        let data = data.into();
        if route.trim().is_empty() {
            return Err(CanFrameError::EmptyRoute);
        }
        if data.len() > 8 {
            return Err(CanFrameError::InvalidDlc(data.len()));
        }
        Ok(Self {
            timestamp_us,
            route,
            id,
            data,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanFrameError {
    InvalidStandardId(u16),
    InvalidExtendedId(u32),
    InvalidDlc(usize),
    EmptyRoute,
}

impl fmt::Display for CanFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStandardId(id) => {
                write!(formatter, "standard CAN ID exceeds 0x7ff: {id:#x}")
            }
            Self::InvalidExtendedId(id) => {
                write!(formatter, "extended CAN ID exceeds 0x1fffffff: {id:#x}")
            }
            Self::InvalidDlc(dlc) => write!(formatter, "classic CAN DLC exceeds 8: {dlc}"),
            Self::EmptyRoute => formatter.write_str("CAN route must not be empty"),
        }
    }
}
impl std::error::Error for CanFrameError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanSourceKind {
    Live,
    Replay,
    Simulator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanSourceError {
    InvalidData(String),
    Io(String),
}

impl fmt::Display for CanSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidData(message) => write!(formatter, "invalid CAN source data: {message}"),
            Self::Io(message) => write!(formatter, "CAN source I/O error: {message}"),
        }
    }
}
impl std::error::Error for CanSourceError {}

/// Read-only raw CAN source consumed by the shared protocol core.
pub trait CanFrameSource {
    fn source_kind(&self) -> CanSourceKind;
    fn next_frame(&mut self) -> Result<Option<CanFrame>, CanSourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_classic_can_shape() {
        assert!(CanId::standard(0x800).is_err());
        assert!(CanId::extended(0x2000_0000).is_err());
        assert!(CanFrame::new(0, "route", CanId::standard(0x123).unwrap(), [0; 9]).is_err());
    }
}
