use std::fmt;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    Io(String),
    Timeout,
    Closed,
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "transport I/O error: {message}"),
            Self::Timeout => formatter.write_str("transport read timed out"),
            Self::Closed => formatter.write_str("transport is closed"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Minimal contract consumed by protocol engines.
///
/// Opening a transport must not imply an application-protocol write. `close`
/// closes only the underlying byte stream.
pub trait ByteTransport: Send {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
    fn set_read_timeout(&mut self, timeout: Duration) -> Result<(), TransportError>;
    fn close(&mut self) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    use super::TransportError;

    #[test]
    fn transport_errors_are_explicit() {
        assert_eq!(
            TransportError::Timeout.to_string(),
            "transport read timed out"
        );
    }
}
