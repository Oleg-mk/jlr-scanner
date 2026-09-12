use app_contracts::{
    AdapterErrorCode, AdapterInfo, AdapterSnapshot, AdapterState, AdapterSummary,
    BoardCommunicationState, BoardInfoEvidence, InterfaceImplementationState, UserFacingError,
    VehicleInterfaceCapability, VehicleValidationState,
};
use diagnostic_execution::PreparedDiagnosticTransaction;
use mongoose_jlr::bench::{BenchTransport, SharedBenchBus};
use mongoose_jlr::MongooseJ1979ReadResult;
use mongoose_jlr::{list_vehicle_routes, MongooseJlrDevice, VehicleRouteId};
use mongoose_jlr::{MongooseCalibrationIdentificationResult, MongooseDiagnosticError};
use mongoose_jlr::{
    MongooseUdsReadResult, ProtocolError, RouteCapture, CAPTURE_IDLE_TIMEOUT, CAPTURE_MAX_FRAMES,
};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;
use transport_serial::{
    matching_devices, SerialDevice, SerialTransport, SystemSerialDeviceEnumerator,
    MONGOOSE_JLR_USB_PID, MONGOOSE_JLR_USB_VID,
};
use uds_execution::PreparedUdsTransaction;

const ADAPTER_NAME: &str = "MongoosePro JLR";
const TRANSPORT_NAME: &str = "USB CDC / Serial";
const BACKEND_NAME: &str = "mongoose-jlr";
const VEHICLE_MESSAGE: &str = "No vehicle connected";
/// The bench (ADR-0020) as the adapter panel knows it: a port name no serial
/// device can have, and a transport name the whole interface keys on.
pub const BENCH_PORT: &str = "bench";
pub const BENCH_TRANSPORT: &str = "bench";
const BENCH_NAME: &str = "Virtual vehicle (bench)";
const BENCH_BACKEND: &str = "bench-vehicle";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendFailure {
    Discovery(String),
    AlreadyInUse(String),
    Open(String),
    Disconnected(String),
    BoardCommunication(String),
    Disconnect(String),
    /// The session holds records of the other kind; a new session comes first.
    SessionMode(String),
}

pub trait AdapterBackend {
    type Connection;

    fn discover(&self) -> Result<Vec<AdapterSummary>, BackendFailure>;
    fn connect(
        &self,
        adapter: &AdapterSummary,
    ) -> Result<(Self::Connection, BoardInfoEvidence), BackendFailure>;
    fn disconnect(&self, connection: &mut Self::Connection) -> Result<(), BackendFailure>;
}

/// The system's adapters: the serial MongoosePro JLR, and the bench, whose
/// vehicle is shared with the session that describes it (ADR-0020).
#[derive(Clone)]
pub struct SystemAdapterBackend {
    bench: SharedBenchBus,
}

impl SystemAdapterBackend {
    pub fn new(bench: SharedBenchBus) -> Self {
        Self { bench }
    }
}

impl Default for SystemAdapterBackend {
    fn default() -> Self {
        Self::new(mongoose_jlr::bench::share_bus(Box::new(
            transport_api::EmptyBench,
        )))
    }
}

/// One connection, to the adapter or to the bench; the device code above
/// the transport is the same either way.
pub enum Link {
    Serial(MongooseJlrDevice<SerialTransport>),
    Bench(MongooseJlrDevice<BenchTransport>),
}

impl Link {
    fn close(&mut self) -> Result<(), ProtocolError> {
        match self {
            Self::Serial(device) => device.close(),
            Self::Bench(device) => device.close(),
        }
    }

    fn execute_prepared_calibration_identification(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError> {
        match self {
            Self::Serial(device) => {
                device.execute_prepared_calibration_identification(transaction, timeout)
            }
            Self::Bench(device) => {
                device.execute_prepared_calibration_identification(transaction, timeout)
            }
        }
    }

    fn capture_route(
        &mut self,
        route_id: VehicleRouteId,
        duration: Duration,
    ) -> Result<RouteCapture, ProtocolError> {
        match self {
            Self::Serial(device) => {
                device.capture_route(route_id, duration, CAPTURE_IDLE_TIMEOUT, CAPTURE_MAX_FRAMES)
            }
            Self::Bench(device) => {
                device.capture_route(route_id, duration, CAPTURE_IDLE_TIMEOUT, CAPTURE_MAX_FRAMES)
            }
        }
    }

    fn execute_prepared_uds_read(
        &mut self,
        transaction: &PreparedUdsTransaction,
        timeout: Duration,
    ) -> Result<MongooseUdsReadResult, MongooseDiagnosticError> {
        match self {
            Self::Serial(device) => device.execute_prepared_uds_read(transaction, timeout),
            Self::Bench(device) => device.execute_prepared_uds_read(transaction, timeout),
        }
    }

    fn execute_prepared_j1979_read(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Result<MongooseJ1979ReadResult, MongooseDiagnosticError> {
        match self {
            Self::Serial(device) => device.execute_prepared_j1979_read(transaction, timeout),
            Self::Bench(device) => device.execute_prepared_j1979_read(transaction, timeout),
        }
    }
}

pub struct SystemConnection {
    device: Link,
}

fn bench_summary() -> AdapterSummary {
    AdapterSummary {
        name: BENCH_NAME.to_owned(),
        port: BENCH_PORT.to_owned(),
        usb_vid: 0,
        usb_pid: 0,
        serial_number: None,
        driver: None,
    }
}

impl AdapterBackend for SystemAdapterBackend {
    type Connection = SystemConnection;

    fn discover(&self) -> Result<Vec<AdapterSummary>, BackendFailure> {
        let devices = matching_devices(
            &SystemSerialDeviceEnumerator,
            MONGOOSE_JLR_USB_VID,
            MONGOOSE_JLR_USB_PID,
        )
        .map_err(|error| BackendFailure::Discovery(error.to_string()))?;

        Ok(devices.into_iter().map(summary_from_device).collect())
    }

    fn connect(
        &self,
        adapter: &AdapterSummary,
    ) -> Result<(Self::Connection, BoardInfoEvidence), BackendFailure> {
        if adapter.port == BENCH_PORT {
            let mut mongoose = MongooseJlrDevice::open(BenchTransport::new(self.bench.clone()));
            let response = mongoose
                .get_board_info()
                .map_err(|error| BackendFailure::BoardCommunication(error.to_string()))?;
            let evidence = BoardInfoEvidence {
                response_command: format!("0x{:04X}", response.response_command),
                raw_response_hex: encode_hex(&response.raw_board_info),
            };
            return Ok((
                SystemConnection {
                    device: Link::Bench(mongoose),
                },
                evidence,
            ));
        }
        let device = SerialDevice {
            port_name: adapter.port.clone(),
            usb_vid: adapter.usb_vid,
            usb_pid: adapter.usb_pid,
            usb_serial_number: adapter.serial_number.clone(),
            driver_service: adapter.driver.clone(),
        };
        let transport = SerialTransport::open(&device).map_err(|error| {
            let details = error.to_string();
            if looks_like_adapter_in_use(&details) {
                BackendFailure::AlreadyInUse(details)
            } else {
                BackendFailure::Open(details)
            }
        })?;
        let mut mongoose = MongooseJlrDevice::open(transport);
        let response = match mongoose.get_board_info() {
            Ok(response) => response,
            Err(error) => {
                let details = error.to_string();
                let _ = mongoose.close();
                return Err(BackendFailure::BoardCommunication(details));
            }
        };

        let evidence = BoardInfoEvidence {
            response_command: format!("0x{:04X}", response.response_command),
            raw_response_hex: encode_hex(&response.raw_board_info),
        };
        Ok((
            SystemConnection {
                device: Link::Serial(mongoose),
            },
            evidence,
        ))
    }

    fn disconnect(&self, connection: &mut Self::Connection) -> Result<(), BackendFailure> {
        connection
            .device
            .close()
            .map_err(|error| BackendFailure::Disconnect(error.to_string()))
    }
}

pub struct AdapterService<B: AdapterBackend> {
    backend: B,
    state: AdapterState,
    adapters: Vec<AdapterSummary>,
    selected_adapter_port: Option<String>,
    adapter: Option<AdapterInfo>,
    board_communication: BoardCommunicationState,
    error: Option<UserFacingError>,
    connection: Option<B::Connection>,
    /// The scenario the bench was connected on, while it is the bench.
    bench_scenario: Option<u32>,
}

impl<B: AdapterBackend> AdapterService<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            state: AdapterState::NoAdapter,
            adapters: Vec::new(),
            selected_adapter_port: None,
            adapter: None,
            board_communication: BoardCommunicationState::Unavailable,
            error: None,
            connection: None,
            bench_scenario: None,
        }
    }

    pub fn snapshot(&self) -> AdapterSnapshot {
        AdapterSnapshot {
            state: self.state,
            adapters: self.adapters.clone(),
            selected_adapter_port: self.selected_adapter_port.clone(),
            adapter: self.adapter.clone(),
            board_communication: self.board_communication,
            capabilities: interface_capabilities(),
            selection_required: self.adapters.len() > 1
                && self.selected_adapter_port.is_none()
                && self.connection.is_none(),
            error: self.error.clone(),
            vehicle_message: VEHICLE_MESSAGE.to_owned(),
            // Only while the bench is what is connected: a number left from an
            // earlier bench must not travel with a real adapter.
            bench_scenario: self.is_bench().then_some(self.bench_scenario).flatten(),
        }
    }

    pub fn discover(&mut self) -> AdapterSnapshot {
        // The bench is not on any port: while it is connected, the serial
        // enumeration has nothing to say about it.
        if self.is_bench() {
            return self.snapshot();
        }
        let discovered = match self.backend.discover() {
            Ok(adapters) => adapters,
            Err(error) => {
                self.set_error(error);
                return self.snapshot();
            }
        };

        if self.connection.is_some() {
            let connected_port = self
                .adapter
                .as_ref()
                .map(|adapter| adapter.port.as_str())
                .unwrap_or_default();
            if discovered
                .iter()
                .any(|adapter| adapter.port == connected_port)
            {
                self.adapters = discovered;
                return self.snapshot();
            }

            if let Some(mut connection) = self.connection.take() {
                let _ = self.backend.disconnect(&mut connection);
            }
            self.adapters = discovered;
            self.adapter = None;
            self.selected_adapter_port = None;
            self.board_communication = BoardCommunicationState::Unavailable;
            self.state = AdapterState::Error;
            self.error = Some(user_error(BackendFailure::Disconnected(
                "connected adapter is no longer present".to_owned(),
            )));
            return self.snapshot();
        }

        if self.state == AdapterState::Error
            && self
                .error
                .as_ref()
                .is_some_and(|error| error.code == AdapterErrorCode::AdapterDisconnected)
            && discovered.is_empty()
        {
            self.adapters = discovered;
            return self.snapshot();
        }

        self.apply_discovery(discovered);
        self.snapshot()
    }

    pub fn connect(&mut self, requested_port: Option<String>) -> AdapterSnapshot {
        // Leaving the bench for a real adapter: the bench link goes first,
        // so that no state of it survives whatever the enumeration finds.
        if self.is_bench() {
            if let Some(mut connection) = self.connection.take() {
                let _ = self.backend.disconnect(&mut connection);
            }
            self.adapter = None;
            self.board_communication = BoardCommunicationState::Unavailable;
        }
        let discovered = match self.backend.discover() {
            Ok(adapters) => adapters,
            Err(error) => {
                self.set_error(error);
                return self.snapshot();
            }
        };
        self.adapters = discovered;

        let selected = match select_adapter(&self.adapters, requested_port.as_deref()) {
            Ok(Some(adapter)) => adapter,
            Ok(None) => {
                self.state = if self.adapters.is_empty() {
                    AdapterState::NoAdapter
                } else {
                    AdapterState::AdapterDetected
                };
                self.selected_adapter_port = None;
                self.error = None;
                return self.snapshot();
            }
            Err(error) => {
                self.set_error(error);
                return self.snapshot();
            }
        };

        self.state = AdapterState::Connecting;
        self.selected_adapter_port = Some(selected.port.clone());
        self.board_communication = BoardCommunicationState::Pending;
        self.error = None;

        match self.backend.connect(&selected) {
            Ok((connection, board_info)) => {
                self.connection = Some(connection);
                self.adapter = Some(AdapterInfo {
                    name: selected.name.clone(),
                    connection_status: "Connected".to_owned(),
                    port: selected.port.clone(),
                    usb_vid: selected.usb_vid,
                    usb_pid: selected.usb_pid,
                    serial_number: selected.serial_number.clone(),
                    transport: TRANSPORT_NAME.to_owned(),
                    driver: selected.driver.clone(),
                    backend: BACKEND_NAME.to_owned(),
                    board_info,
                });
                self.board_communication = BoardCommunicationState::Verified;
                self.state = AdapterState::Connected;
            }
            Err(error) => {
                self.board_communication = if matches!(error, BackendFailure::BoardCommunication(_))
                {
                    BoardCommunicationState::Failed
                } else {
                    BoardCommunicationState::Unavailable
                };
                self.set_error(error);
            }
        }

        self.snapshot()
    }

    pub fn disconnect(&mut self) -> AdapterSnapshot {
        if let Some(mut connection) = self.connection.take() {
            if let Err(error) = self.backend.disconnect(&mut connection) {
                self.set_error(error);
                return self.snapshot();
            }
        }

        self.adapter = None;
        self.board_communication = BoardCommunicationState::Unavailable;
        self.error = None;
        let discovered = self
            .backend
            .discover()
            .unwrap_or_else(|_| self.adapters.clone());
        self.apply_discovery(discovered);
        self.snapshot()
    }

    fn apply_discovery(&mut self, adapters: Vec<AdapterSummary>) {
        self.adapters = adapters;
        self.adapter = None;
        self.board_communication = BoardCommunicationState::Unavailable;
        self.error = None;
        match self.adapters.as_slice() {
            [] => {
                self.state = AdapterState::NoAdapter;
                self.selected_adapter_port = None;
            }
            [adapter] => {
                self.state = AdapterState::AdapterDetected;
                self.selected_adapter_port = Some(adapter.port.clone());
            }
            _ => {
                self.state = AdapterState::AdapterDetected;
                self.selected_adapter_port = None;
            }
        }
    }

    fn set_error(&mut self, failure: BackendFailure) {
        self.state = AdapterState::Error;
        self.error = Some(user_error(failure));
        self.adapter = None;
        self.connection = None;
    }
}

pub type SharedAdapterService = Mutex<AdapterService<SystemAdapterBackend>>;

impl<B: AdapterBackend> AdapterService<B> {
    /// Whether the connection, if any, is the bench rather than an adapter.
    pub fn is_bench(&self) -> bool {
        self.connection.is_some()
            && self
                .adapter
                .as_ref()
                .is_some_and(|adapter| adapter.transport == BENCH_TRANSPORT)
    }

    /// Refuse a switch between the bench and an adapter while the session
    /// holds records of the other kind (ADR-0020).
    pub fn refuse_session_switch(&mut self, wanted: &str) -> AdapterSnapshot {
        // Whatever is connected stays connected: refusing a switch is not a
        // failure of the connection.
        self.error = Some(user_error(BackendFailure::SessionMode(format!(
            "this session already holds {} records; start a new session before connecting the {wanted}",
            if wanted == BENCH_TRANSPORT { "adapter" } else { "bench" }
        ))));
        self.snapshot()
    }

    /// Forget a refused switch once a new session has made it moot.
    pub fn clear_session_error(&mut self) {
        if self
            .error
            .as_ref()
            .is_some_and(|error| error.code == AdapterErrorCode::SessionModeMismatch)
        {
            self.error = None;
        }
    }

    /// Connect the bench (ADR-0020): no port, no discovery, the vehicle the
    /// session describes behind a stand-in adapter that answers with the
    /// firmware's own frames. Whatever was connected is closed first.
    pub fn connect_bench(&mut self, scenario: u32) -> AdapterSnapshot {
        if let Some(mut connection) = self.connection.take() {
            let _ = self.backend.disconnect(&mut connection);
        }
        self.bench_scenario = Some(scenario);
        let summary = bench_summary();
        self.state = AdapterState::Connecting;
        self.selected_adapter_port = Some(summary.port.clone());
        self.board_communication = BoardCommunicationState::Pending;
        self.error = None;
        match self.backend.connect(&summary) {
            Ok((connection, board_info)) => {
                self.connection = Some(connection);
                self.adapter = Some(AdapterInfo {
                    name: summary.name.clone(),
                    connection_status: "Connected".to_owned(),
                    port: summary.port.clone(),
                    usb_vid: 0,
                    usb_pid: 0,
                    serial_number: None,
                    transport: BENCH_TRANSPORT.to_owned(),
                    driver: None,
                    backend: BENCH_BACKEND.to_owned(),
                    board_info,
                });
                self.board_communication = BoardCommunicationState::Verified;
                self.state = AdapterState::Connected;
            }
            Err(error) => {
                self.board_communication = BoardCommunicationState::Failed;
                self.bench_scenario = None;
                self.set_error(error);
            }
        }
        self.snapshot()
    }
}

impl AdapterService<SystemAdapterBackend> {
    pub fn connected_adapter(&self) -> Option<AdapterInfo> {
        (self.state == AdapterState::Connected)
            .then(|| self.adapter.clone())
            .flatten()
    }

    pub fn execute_calibration_identification(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Option<Result<MongooseCalibrationIdentificationResult, MongooseDiagnosticError>> {
        self.connection.as_mut().map(|connection| {
            connection
                .device
                .execute_prepared_calibration_identification(transaction, timeout)
        })
    }
}

impl AdapterService<SystemAdapterBackend> {
    /// Listen-only capture on one route. Receive-only by construction: the
    /// device method opens the route with the listen-only flag and never
    /// exposes a transmit primitive.
    pub fn capture_route(
        &mut self,
        route_id: VehicleRouteId,
        duration: Duration,
    ) -> Option<Result<RouteCapture, ProtocolError>> {
        self.connection
            .as_mut()
            .map(|connection| connection.device.capture_route(route_id, duration))
    }
}

impl AdapterService<SystemAdapterBackend> {
    /// One prepared read-only UDS request, live (ADR-0015). The device method
    /// accepts nothing but a prepared transaction.
    pub fn execute_uds_read(
        &mut self,
        transaction: &PreparedUdsTransaction,
        timeout: Duration,
    ) -> Option<Result<MongooseUdsReadResult, MongooseDiagnosticError>> {
        self.connection.as_mut().map(|connection| {
            connection
                .device
                .execute_prepared_uds_read(transaction, timeout)
        })
    }

    /// One prepared legislated OBD-II read, live (ADR-0022, decision 7).
    /// The device method accepts nothing but a transaction prepared from the
    /// standard.
    pub fn execute_j1979_read(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Option<Result<MongooseJ1979ReadResult, MongooseDiagnosticError>> {
        self.connection.as_mut().map(|connection| {
            connection
                .device
                .execute_prepared_j1979_read(transaction, timeout)
        })
    }
}

pub fn shared_service(bench: SharedBenchBus) -> SharedAdapterService {
    Mutex::new(AdapterService::new(SystemAdapterBackend::new(bench)))
}

pub fn lock_service<'a>(
    state: &'a tauri::State<'a, SharedAdapterService>,
) -> MutexGuard<'a, AdapterService<SystemAdapterBackend>> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn select_adapter(
    adapters: &[AdapterSummary],
    requested_port: Option<&str>,
) -> Result<Option<AdapterSummary>, BackendFailure> {
    if let Some(port) = requested_port {
        return adapters
            .iter()
            .find(|adapter| adapter.port == port)
            .cloned()
            .map(Some)
            .ok_or_else(|| {
                BackendFailure::Disconnected(format!("adapter on {port} was not found"))
            });
    }

    match adapters {
        [] => Ok(None),
        [adapter] => Ok(Some(adapter.clone())),
        _ => Ok(None),
    }
}

fn summary_from_device(device: SerialDevice) -> AdapterSummary {
    AdapterSummary {
        name: ADAPTER_NAME.to_owned(),
        port: device.port_name,
        usb_vid: device.usb_vid,
        usb_pid: device.usb_pid,
        serial_number: device.usb_serial_number,
        driver: device.driver_service,
    }
}

fn interface_capabilities() -> Vec<VehicleInterfaceCapability> {
    let mut capabilities = list_vehicle_routes()
        .iter()
        .map(|route| {
            // ADR-0029: the K-line routes exist by pin and by name; the
            // adapter has never opened one, so they are a hypothesis, not an
            // implemented interface, and nothing about them is confirmed.
            let k_line = matches!(route.id, VehicleRouteId::KLine7 | VehicleRouteId::KLine8);
            VehicleInterfaceCapability {
                id: route.id.as_str().to_owned(),
                name: match route.id {
                    VehicleRouteId::HsCan => "HS-CAN",
                    VehicleRouteId::MsCan => "MS-CAN",
                    VehicleRouteId::KLine7 => "K-line (pin 7)",
                    VehicleRouteId::KLine8 => "K-line (pin 8)",
                }
                .to_owned(),
                pins: route
                    .obd_pins
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join("/"),
                nominal_bitrate: route.bitrate,
                hardware_confirmed: !k_line,
                fixture_tested: !k_line,
                implementation: if k_line {
                    InterfaceImplementationState::Hypothesis
                } else {
                    InterfaceImplementationState::Available
                },
                vehicle_validation: VehicleValidationState::NotYetValidated,
            }
        })
        .collect::<Vec<_>>();

    capabilities.push(VehicleInterfaceCapability {
        id: "ccp-hs-can".to_owned(),
        name: "CCP HS-CAN".to_owned(),
        pins: "12/13 on X250".to_owned(),
        nominal_bitrate: None,
        hardware_confirmed: false,
        fixture_tested: false,
        implementation: InterfaceImplementationState::UnsupportedByAdapter,
        vehicle_validation: VehicleValidationState::NotApplicable,
    });
    capabilities
}

fn user_error(failure: BackendFailure) -> UserFacingError {
    match failure {
        BackendFailure::Discovery(details) => UserFacingError {
            code: AdapterErrorCode::DiscoveryFailed,
            message: "Unable to detect adapters".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::AlreadyInUse(details) => UserFacingError {
            code: AdapterErrorCode::AdapterAlreadyInUse,
            message: "Adapter is already in use by another application".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::Open(details) => UserFacingError {
            code: AdapterErrorCode::UnableToOpenAdapter,
            message: "Unable to open adapter".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::Disconnected(details) => UserFacingError {
            code: AdapterErrorCode::AdapterDisconnected,
            message: "Adapter disconnected".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::BoardCommunication(details) => UserFacingError {
            code: AdapterErrorCode::BoardCommunicationFailed,
            message: "Board communication failed".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::Disconnect(details) => UserFacingError {
            code: AdapterErrorCode::DisconnectFailed,
            message: "Unable to disconnect adapter cleanly".to_owned(),
            technical_details: Some(details),
        },
        BackendFailure::SessionMode(details) => UserFacingError {
            code: AdapterErrorCode::SessionModeMismatch,
            message: "Start a new session before switching between the bench and an adapter"
                .to_owned(),
            technical_details: Some(details),
        },
    }
}

fn looks_like_adapter_in_use(details: &str) -> bool {
    let details = details.to_ascii_lowercase();
    details.contains("access is denied")
        || details.contains("os error 5")
        || details.contains("permission denied")
        || details.contains("in use")
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;

    #[derive(Default)]
    struct FakeState {
        discoveries: VecDeque<Result<Vec<AdapterSummary>, BackendFailure>>,
        connect_result: Option<Result<BoardInfoEvidence, BackendFailure>>,
        disconnect_result: Option<Result<(), BackendFailure>>,
        disconnects: usize,
    }

    #[derive(Clone, Default)]
    struct FakeBackend(Arc<Mutex<FakeState>>);

    impl FakeBackend {
        fn with_discoveries(discoveries: Vec<Result<Vec<AdapterSummary>, BackendFailure>>) -> Self {
            Self(Arc::new(Mutex::new(FakeState {
                discoveries: discoveries.into(),
                ..FakeState::default()
            })))
        }

        fn set_connect_result(&self, result: Result<BoardInfoEvidence, BackendFailure>) {
            self.0.lock().unwrap().connect_result = Some(result);
        }
    }

    impl AdapterBackend for FakeBackend {
        type Connection = ();

        fn discover(&self) -> Result<Vec<AdapterSummary>, BackendFailure> {
            self.0
                .lock()
                .unwrap()
                .discoveries
                .pop_front()
                .unwrap_or_else(|| Ok(Vec::new()))
        }

        fn connect(
            &self,
            _adapter: &AdapterSummary,
        ) -> Result<(Self::Connection, BoardInfoEvidence), BackendFailure> {
            self.0
                .lock()
                .unwrap()
                .connect_result
                .take()
                .unwrap_or_else(|| Ok(board_info()))
                .map(|evidence| ((), evidence))
        }

        fn disconnect(&self, _connection: &mut Self::Connection) -> Result<(), BackendFailure> {
            let mut state = self.0.lock().unwrap();
            state.disconnects += 1;
            state.disconnect_result.take().unwrap_or(Ok(()))
        }
    }

    fn adapter(port: &str) -> AdapterSummary {
        AdapterSummary {
            name: ADAPTER_NAME.to_owned(),
            port: port.to_owned(),
            usb_vid: MONGOOSE_JLR_USB_VID,
            usb_pid: MONGOOSE_JLR_USB_PID,
            serial_number: Some(format!("SERIAL-{port}")),
            driver: Some("usbser".to_owned()),
        }
    }

    fn board_info() -> BoardInfoEvidence {
        BoardInfoEvidence {
            response_command: "0x8109".to_owned(),
            raw_response_hex: "AA BB".to_owned(),
        }
    }

    #[test]
    fn zero_one_and_multiple_adapter_discovery_are_explicit() {
        let backend = FakeBackend::with_discoveries(vec![
            Ok(vec![]),
            Ok(vec![adapter("COM7")]),
            Ok(vec![adapter("COM7"), adapter("COM8")]),
        ]);
        let mut service = AdapterService::new(backend);

        assert_eq!(service.discover().state, AdapterState::NoAdapter);
        let one = service.discover();
        assert_eq!(one.state, AdapterState::AdapterDetected);
        assert_eq!(one.selected_adapter_port.as_deref(), Some("COM7"));
        let multiple = service.discover();
        assert_eq!(multiple.state, AdapterState::AdapterDetected);
        assert!(multiple.selection_required);
        assert!(multiple.selected_adapter_port.is_none());
    }

    #[test]
    fn connect_success_verifies_real_board_info_evidence_shape() {
        let backend = FakeBackend::with_discoveries(vec![Ok(vec![adapter("COM7")])]);
        let mut service = AdapterService::new(backend);
        let connected = service.connect(None);

        assert_eq!(connected.state, AdapterState::Connected);
        assert_eq!(
            connected.board_communication,
            BoardCommunicationState::Verified
        );
        assert_eq!(
            connected.adapter.unwrap().board_info.response_command,
            "0x8109"
        );
    }

    #[test]
    fn connection_and_board_info_errors_are_user_facing() {
        let backend = FakeBackend::with_discoveries(vec![Ok(vec![adapter("COM7")])]);
        backend.set_connect_result(Err(BackendFailure::AlreadyInUse("os error 5".to_owned())));
        let mut service = AdapterService::new(backend);
        let busy = service.connect(None);
        assert_eq!(busy.state, AdapterState::Error);
        assert_eq!(
            busy.error.unwrap().code,
            AdapterErrorCode::AdapterAlreadyInUse
        );

        let backend = FakeBackend::with_discoveries(vec![Ok(vec![adapter("COM7")])]);
        backend.set_connect_result(Err(BackendFailure::BoardCommunication(
            "invalid response".to_owned(),
        )));
        let mut service = AdapterService::new(backend);
        let invalid = service.connect(None);
        assert_eq!(invalid.board_communication, BoardCommunicationState::Failed);
        assert_eq!(
            invalid.error.unwrap().code,
            AdapterErrorCode::BoardCommunicationFailed
        );
    }

    #[test]
    fn disconnect_returns_to_detected_without_vehicle_commands() {
        let backend = FakeBackend::with_discoveries(vec![
            Ok(vec![adapter("COM7")]),
            Ok(vec![adapter("COM7")]),
        ]);
        let state = Arc::clone(&backend.0);
        let mut service = AdapterService::new(backend);
        assert_eq!(service.connect(None).state, AdapterState::Connected);
        let disconnected = service.disconnect();

        assert_eq!(disconnected.state, AdapterState::AdapterDetected);
        assert_eq!(state.lock().unwrap().disconnects, 1);
    }

    #[test]
    fn hot_unplug_is_reported_and_replug_can_be_detected() {
        let backend = FakeBackend::with_discoveries(vec![
            Ok(vec![adapter("COM7")]),
            Ok(vec![]),
            Ok(vec![adapter("COM9")]),
        ]);
        let mut service = AdapterService::new(backend);
        assert_eq!(service.connect(None).state, AdapterState::Connected);

        let unplugged = service.discover();
        assert_eq!(unplugged.state, AdapterState::Error);
        assert_eq!(
            unplugged.error.unwrap().code,
            AdapterErrorCode::AdapterDisconnected
        );

        let replugged = service.discover();
        assert_eq!(replugged.state, AdapterState::AdapterDetected);
        assert_eq!(replugged.selected_adapter_port.as_deref(), Some("COM9"));
    }

    #[test]
    fn capability_statuses_do_not_claim_vehicle_validation() {
        let capabilities = interface_capabilities();
        // HS-CAN, MS-CAN, the two K-line routes (ADR-0029), CCP.
        assert_eq!(capabilities.len(), 5);
        assert!(capabilities[..2].iter().all(|capability| {
            capability.implementation == InterfaceImplementationState::Available
                && capability.hardware_confirmed
                && capability.fixture_tested
                && capability.vehicle_validation == VehicleValidationState::NotYetValidated
        }));
        // A K-line route is a hypothesis with nothing confirmed and no bit
        // rate of its own; the bus states the rate.
        for (capability, name, pins) in [
            (&capabilities[2], "K-line (pin 7)", "7"),
            (&capabilities[3], "K-line (pin 8)", "8"),
        ] {
            assert_eq!(capability.name, name);
            assert_eq!(capability.pins, pins);
            assert_eq!(capability.nominal_bitrate, None);
            assert_eq!(
                capability.implementation,
                InterfaceImplementationState::Hypothesis
            );
            assert!(!capability.hardware_confirmed);
            assert!(!capability.fixture_tested);
            assert_eq!(
                capability.vehicle_validation,
                VehicleValidationState::NotYetValidated
            );
        }
        assert_eq!(
            capabilities[4].implementation,
            InterfaceImplementationState::UnsupportedByAdapter
        );
    }
}
