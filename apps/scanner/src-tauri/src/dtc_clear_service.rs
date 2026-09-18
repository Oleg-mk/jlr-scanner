//! Clearing a module's fault codes (`ADR-0036`, stage 2 step 1): the first
//! operation of this product that changes what a module holds.
//!
//! The shell's side of it, beside `module_read_service`: the clear is
//! planned from the same survey and the same resolved plan a read uses, sent
//! only as a prepared service - over UDS with the extended session opened and
//! left where the module asks for it, over a K-line in the protocol's own
//! command - and recorded with what it erased: the codes the module held when
//! they were read in this session, and what it answered when read again.
//! Nothing here decides whether the clear was a good idea; the person did,
//! in the confirmation, and the record says so.

use crate::module_read_service::{hex, map_live_error, preparation_error};
use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage,
    DtcClearRequest, DtcClearSnapshot, DtcClearState, ModuleReadReport, ModuleReadSnapshot,
};
use diagnostic_environment::{DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver};
use diagnostic_session::{validation_label, vehicle_context, KnowledgeLibrary};
use kline_execution::{
    decode_service_response as decode_kline_service, prepare_kline_service, KlineServiceOutcome,
    KlineTarget, PreparedKlineService, ServiceKlineIntent,
};
use mongoose_jlr::{MongooseDiagnosticError, MongooseKlineReadResult, MongooseUdsServiceResult};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uds_execution::{
    decode_service_response as decode_uds_service, prepare_service_transaction,
    DiagnosticTargetIdentity, PreparedUdsService, ServiceUdsIntent, UdsServiceOutcome,
    ALL_DTC_GROUPS, READ_DTC_INFORMATION_CAPABILITY,
};

/// How long one module is given to answer a clear, and each exchange of the
/// session around it. A clear takes a module tens of milliseconds; two
/// seconds is what a read is given too.
pub const DTC_CLEAR_TIMEOUT: Duration = Duration::from_secs(2);
/// The record's schema in the session bundle (`ADR-0036`, decision 5).
pub const DTC_CLEAR_SCHEMA: &str = "prowlone.dtc-clear";
pub const DTC_CLEAR_SCHEMA_VERSION: u16 = 1;
/// The operation's name in `SAFETY_BOUNDARIES.md` and in every record.
pub const DTC_CLEAR_OPERATION: &str = "DTC_CLEAR";
/// The class the owner gave the clear (2026-09-18).
pub const DTC_CLEAR_SAFETY_CLASS: &str = "SERVICE_ROUTINE";

/// A clear the library could plan, on the path its module speaks.
pub enum PreparedDtcClear {
    Uds {
        service: PreparedUdsService,
        route_validation: String,
    },
    Kline {
        service: PreparedKlineService,
        route_validation: String,
    },
}

impl PreparedDtcClear {
    pub fn route_validation(&self) -> &str {
        match self {
            Self::Uds {
                route_validation, ..
            }
            | Self::Kline {
                route_validation, ..
            } => route_validation,
        }
    }

    fn request_hex(&self) -> String {
        match self {
            Self::Uds { service, .. } => hex(service.transaction().encoded_payload()),
            Self::Kline { service, .. } => hex(service.transaction().encoded_payload()),
        }
    }

    fn route_id(&self) -> String {
        match self {
            Self::Uds { service, .. } => service.transaction().backend_route().to_string(),
            Self::Kline { service, .. } => service.transaction().backend_route().to_string(),
        }
    }

    fn protocol(&self) -> String {
        match self {
            Self::Uds { service, .. } => service.transaction().protocol_family().to_string(),
            Self::Kline { service, .. } => service.transaction().protocol_family().to_string(),
        }
    }
}

/// What the adapter brought back, on whichever path the module speaks.
pub enum ClearResult {
    Uds(Result<MongooseUdsServiceResult, MongooseDiagnosticError>),
    Kline(Result<MongooseKlineReadResult, MongooseDiagnosticError>),
}

/// The clear's outcome and its record, kept for the panel and the report.
#[derive(Default)]
pub struct DtcClearService {
    last: Option<(DtcClearSnapshot, Value)>,
}

impl DtcClearService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> DtcClearSnapshot {
        self.last
            .as_ref()
            .map(|(snapshot, _)| snapshot.clone())
            .unwrap_or_else(idle)
    }

    /// Resolve and prepare without touching the adapter. A module on a
    /// K-line clears in its own protocol; every other module over UDS. The
    /// plan is the one a fault-code read resolves to, because a clear is
    /// addressed exactly as the read that found the codes.
    pub fn prepare(
        library: &KnowledgeLibrary,
        request: &DtcClearRequest,
    ) -> Result<PreparedDtcClear, DiagnosticError> {
        let context = vehicle_context(&request.context);
        if let Ok(target) = KlineTarget::family(&request.ecu_family) {
            let candidates = [
                (
                    ds2::FAULT_MEMORY_CAPABILITY,
                    ServiceKlineIntent::Ds2ClearFaultMemory {
                        target: target.clone(),
                    },
                ),
                (
                    kwp2000::READ_DTC_BY_STATUS_CAPABILITY,
                    ServiceKlineIntent::KwpClearDiagnosticInformation {
                        target: target.clone(),
                    },
                ),
            ];
            for (capability, intent) in candidates {
                let Ok(resolution) = DiagnosticEnvironmentResolver::resolve_ecu_family(
                    library.store(),
                    &context,
                    &request.ecu_family,
                    capability,
                ) else {
                    continue;
                };
                let DiagnosticEnvironmentResolution::Resolved(plan) = &resolution else {
                    continue;
                };
                let route_validation = validation_label(plan.validation_state).to_string();
                return prepare_kline_service(&resolution, intent)
                    .map(|service| PreparedDtcClear::Kline {
                        service,
                        route_validation,
                    })
                    .map_err(|error| preparation_error(error.to_string()));
            }
        }

        let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
            library.store(),
            &context,
            &request.ecu_family,
            READ_DTC_INFORMATION_CAPABILITY,
        )
        .map_err(|error| preparation_error(error.to_string()))?;
        let route_validation = match &resolution {
            DiagnosticEnvironmentResolution::Resolved(plan) => {
                validation_label(plan.validation_state).to_string()
            }
            _ => String::new(),
        };
        let target = DiagnosticTargetIdentity::family(&request.ecu_family)
            .map_err(|error| preparation_error(error.to_string()))?;
        prepare_service_transaction(
            &resolution,
            ServiceUdsIntent::clear_diagnostic_information(target, ALL_DTC_GROUPS),
        )
        .map(|service| PreparedDtcClear::Uds {
            service,
            route_validation,
        })
        .map_err(|error| preparation_error(error.to_string()))
    }

    /// Record what the clear came to: accepted, refused by the module, or
    /// not sent at all. `before` is the module's fault-code read in this
    /// session - what the clear erased - and goes into the record whole.
    pub fn finish(
        &mut self,
        request: &DtcClearRequest,
        prepared: Option<&PreparedDtcClear>,
        adapter: Option<&AdapterInfo>,
        before: Option<&(ModuleReadSnapshot, ModuleReadReport)>,
        result: Result<ClearResult, DiagnosticError>,
    ) -> DtcClearSnapshot {
        let mut snapshot = idle();
        snapshot.ecu_family = request.ecu_family.clone();
        if let Some(prepared) = prepared {
            snapshot.route_id = prepared.route_id();
            snapshot.route_validation = prepared.route_validation().to_string();
            snapshot.protocol = prepared.protocol();
            snapshot.request_hex = prepared.request_hex();
        }
        if let Some((read, _)) = before {
            snapshot.codes_before = read.dtcs.clone();
        }

        let mut exchanges: Vec<(String, String)> = Vec::new();
        let mut answer = None;
        match result {
            Ok(ClearResult::Uds(Ok(raw))) => match prepared {
                Some(PreparedDtcClear::Uds { service, .. }) => {
                    snapshot.session = Some(format!("0x{:02X}", raw.session));
                    snapshot.raw_response_hex = Some(hex(&raw.raw_diagnostic_response));
                    exchanges = raw
                        .exchanges
                        .iter()
                        .map(|(request, response)| (hex(request), hex(response)))
                        .collect();
                    match decode_uds_service(service, &raw.raw_diagnostic_response) {
                        Ok(Some(UdsServiceOutcome::Cleared { .. })) => {
                            snapshot.state = DtcClearState::Cleared;
                            answer = Some("cleared".to_string());
                        }
                        Ok(Some(UdsServiceOutcome::Negative(negative))) => {
                            snapshot.state = DtcClearState::Refused;
                            snapshot.refusal = Some(format!("{:?}", negative.code));
                            answer = Some(format!("refused: {:?}", negative.code));
                        }
                        Ok(None) => fail(
                            &mut snapshot,
                            "the final answer was still ResponsePending".to_string(),
                        ),
                        Err(error) => fail(&mut snapshot, error.to_string()),
                    }
                }
                _ => fail(
                    &mut snapshot,
                    "a UDS answer arrived without a prepared UDS clear".to_string(),
                ),
            },
            Ok(ClearResult::Kline(Ok(raw))) => match prepared {
                Some(PreparedDtcClear::Kline { service, .. }) => {
                    snapshot.raw_response_hex = Some(hex(&raw.raw_line_bytes));
                    exchanges.push((hex(&raw.request_payload), hex(&raw.raw_line_bytes)));
                    match decode_kline_service(service, &raw.raw_line_bytes) {
                        Ok(KlineServiceOutcome::Ds2Cleared { .. })
                        | Ok(KlineServiceOutcome::KwpCleared) => {
                            snapshot.state = DtcClearState::Cleared;
                            answer = Some("cleared".to_string());
                        }
                        Ok(KlineServiceOutcome::Ds2NotAccepted { status, .. }) => {
                            snapshot.state = DtcClearState::Refused;
                            snapshot.refusal = Some(format!("DS2 status 0x{status:02X}"));
                            answer = Some(format!("refused: DS2 status 0x{status:02X}"));
                        }
                        Ok(KlineServiceOutcome::KwpNegative { code, text }) => {
                            snapshot.state = DtcClearState::Refused;
                            snapshot.refusal = Some(format!("{text} (0x{code:02X})"));
                            answer = Some(format!("refused: {text} (0x{code:02X})"));
                        }
                        Err(error) => fail(&mut snapshot, error.to_string()),
                    }
                }
                _ => fail(
                    &mut snapshot,
                    "a K-line answer arrived without a prepared K-line clear".to_string(),
                ),
            },
            Ok(ClearResult::Uds(Err(error))) | Ok(ClearResult::Kline(Err(error))) => {
                snapshot.state = DtcClearState::Failed;
                snapshot.error = Some(map_live_error(error));
            }
            Err(error) => {
                snapshot.state = DtcClearState::Failed;
                snapshot.error = Some(error);
            }
        }
        snapshot.report_available = true;

        let timestamp_unix_ms: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let record = json!({
            "schema": DTC_CLEAR_SCHEMA,
            "schema_version": DTC_CLEAR_SCHEMA_VERSION,
            "operation": DTC_CLEAR_OPERATION,
            "safety_class": DTC_CLEAR_SAFETY_CLASS,
            "application_version": env!("CARGO_PKG_VERSION"),
            "timestamp_unix_ms": timestamp_unix_ms,
            "vehicle": request.context,
            "ecu_family": request.ecu_family,
            "adapter_model": adapter.map(|value| value.name.clone()),
            "route_id": snapshot.route_id,
            "route_validation": snapshot.route_validation,
            "protocol": snapshot.protocol,
            "session": snapshot.session,
            "request_payload": snapshot.request_hex,
            "raw_diagnostic_response": snapshot.raw_response_hex,
            "exchanges": exchanges,
            "state": snapshot.state,
            "answer": answer,
            "refusal": snapshot.refusal,
            "error": snapshot.error,
            "codes_before": snapshot.codes_before,
            "before": before.map(|(_, report)| report),
            "codes_after": Value::Null,
            "after": Value::Null,
            "validation": "a service operation of stage 2; the person confirmed it, the module answered, the codes it held are kept above; nothing here is vehicle-confirmed by being here",
        });
        self.last = Some((snapshot.clone(), record));
        snapshot
    }

    /// The module was read again after the clear: what it answers now, kept
    /// beside what it held before.
    pub fn attach_after(
        &mut self,
        after: &(ModuleReadSnapshot, ModuleReadReport),
    ) -> DtcClearSnapshot {
        if let Some((snapshot, record)) = self.last.as_mut() {
            snapshot.codes_after = Some(after.0.dtcs.clone());
            record["codes_after"] = json!(after.0.dtcs);
            record["after"] = json!(after.1);
        }
        self.snapshot()
    }

    /// The clear was answered by the bench (`ADR-0020`): its validation is
    /// `SYNTHETIC`, in the snapshot and in the record.
    pub fn mark_synthetic(&mut self) -> DtcClearSnapshot {
        if let Some((snapshot, record)) = self.last.as_mut() {
            snapshot.route_validation = "SYNTHETIC".into();
            record["route_validation"] = json!("SYNTHETIC");
        }
        self.snapshot()
    }

    pub fn record_json(&self) -> Result<String, String> {
        let (_, record) = self
            .last
            .as_ref()
            .ok_or_else(|| "No clear has been made yet".to_owned())?;
        serde_json::to_string_pretty(record).map_err(|error| error.to_string())
    }
}

fn fail(snapshot: &mut DtcClearSnapshot, details: String) {
    snapshot.state = DtcClearState::Failed;
    snapshot.error = Some(DiagnosticError {
        category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
        message: "Malformed diagnostic response".into(),
        technical_details: Some(details),
        stage: DiagnosticExecutionStage::DiagnosticDecode,
    });
}

/// The refusal the shell gives before anything is sent: the service mode is
/// off, or the module's codes were not read in this session.
pub fn not_offered(details: &str) -> DiagnosticError {
    DiagnosticError {
        category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
        message: "The clear is not offered".into(),
        technical_details: Some(details.into()),
        stage: DiagnosticExecutionStage::Preparation,
    }
}

fn idle() -> DtcClearSnapshot {
    DtcClearSnapshot {
        state: DtcClearState::Idle,
        ecu_family: String::new(),
        operation: DTC_CLEAR_OPERATION.into(),
        safety_class: DTC_CLEAR_SAFETY_CLASS.into(),
        route_id: String::new(),
        route_validation: String::new(),
        protocol: String::new(),
        session: None,
        request_hex: String::new(),
        raw_response_hex: None,
        refusal: None,
        codes_before: Vec::new(),
        codes_after: None,
        error: None,
        report_available: false,
    }
}
