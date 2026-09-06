//! Safe MongoosePro JLR identity and passive raw-CAN receive subset.

mod capture;
mod codec;
mod device;
mod error;
mod passive;
mod uds_live;

pub use capture::{CapturedFrame, RouteCapture, CAPTURE_IDLE_TIMEOUT, CAPTURE_MAX_FRAMES};
pub use codec::{
    BoardInfoResponse, Frame, FrameDecoder, MongoosePacket, SequenceCounter,
    BOARD_INFO_RESPONSE_COMMAND, GET_BOARD_INFO_COMMAND, MAX_PAYLOAD_LENGTH,
};
pub use device::{
    MongooseCalibrationIdentificationResult, MongooseDiagnosticError, MongooseJlrDevice,
    ReceiveCounters,
};
pub use error::ProtocolError;
pub use passive::{
    list_vehicle_routes, CanIdFormat, NetworkType, OpenReceiveRoute, PassiveCapability,
    RawCanFrame, VehicleRoute, VehicleRouteId,
};
pub use uds_live::{MongooseUdsReadResult, UDS_PENDING_TIMEOUT};
