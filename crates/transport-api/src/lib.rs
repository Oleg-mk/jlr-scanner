//! Transport abstractions without hardware- or protocol-specific behavior.

mod byte_stream;
mod can;
pub use byte_stream::{ByteTransport, TransportError};
pub use can::{CanFrame, CanFrameError, CanFrameSource, CanId, CanSourceError, CanSourceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Serial,
    Network,
}

#[cfg(test)]
mod tests {
    use super::TransportKind;

    #[test]
    fn desktop_and_mobile_transport_shapes_are_represented() {
        assert_ne!(TransportKind::Serial, TransportKind::Network);
    }
}
