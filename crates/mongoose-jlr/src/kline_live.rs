//! Execution of a prepared K-line read on the MongoosePro JLR (`ADR-0029`,
//! slice B).
//!
//! The sibling of `uds_live` for the serial lines, and the same one-way
//! boundary: only a `PreparedKlineTransaction` from `kline-execution` can
//! reach this code, the line is opened for that transaction's own protocol,
//! pin, baud and framing, the request bytes go out once, whatever the line
//! carries back is returned undecoded, and the line is closed whatever
//! happens.
//!
//! What the adapter has said about the words below, and what it has not, is
//! in `crate::kline` and in `docs/evidence/mongoose-probe-2026-09-12/`. The
//! resource, the pin and the fast init are confirmed on the bench; the
//! parity command and the inbound record are not, and the first module that
//! answers is what confirms them. Until one does, the survey keeps calling
//! these routes a hypothesis — `ADR-0015`'s rule, on a line instead of a bus.

use crate::device::{MongooseDiagnosticError, MongooseJlrDevice};
use crate::kline::{self, LineParity};
use crate::passive::VehicleRouteId;
use kline_execution::{Parity, PreparedKlineTransaction, TransactionSafetyClass};
use std::time::{Duration, Instant};
use transport_api::ByteTransport;

/// How long a line is listened to after the last byte before it is called
/// quiet. A K-line module answers within tens of milliseconds; a gap longer
/// than this is the end of what it had to say.
pub const KLINE_QUIET_GAP: Duration = Duration::from_millis(120);

/// What a K-line read brought back: the bytes the line carried, echo and
/// all. A K-line echoes what the tester sends; stripping that echo is the
/// protocol's business, not the adapter's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongooseKlineReadResult {
    pub route: VehicleRouteId,
    pub node_address: u8,
    pub request_payload: Vec<u8>,
    pub raw_line_bytes: Vec<u8>,
}

impl<T: ByteTransport> MongooseJlrDevice<T> {
    /// Executes one prepared read-only K-line request: open the line for its
    /// protocol, select the pin, set the framing, wake the bus as the bus
    /// says, send once, listen until the line is quiet, close.
    pub fn execute_prepared_kline_read(
        &mut self,
        transaction: &PreparedKlineTransaction,
        timeout: Duration,
    ) -> Result<MongooseKlineReadResult, MongooseDiagnosticError> {
        let route_id = validate_kline_transaction(transaction)?;
        let pin = kline::pin_for_route(route_id).ok_or(
            MongooseDiagnosticError::UnsupportedTransaction("a K-line read needs a K-line route"),
        )?;
        let resource = kline::resource_for_protocol(transaction.protocol_family()).ok_or(
            MongooseDiagnosticError::UnsupportedTransaction(
                "this protocol has no K-line resource on this adapter",
            ),
        )?;
        let parity = match transaction.framing().parity {
            Parity::None => LineParity::None,
            Parity::Odd => LineParity::Odd,
            Parity::Even => LineParity::Even,
        };

        self.open_kline_line(
            resource,
            transaction.bitrate_bps(),
            pin,
            parity,
            transaction.wakeup(),
            transaction.node_address(),
        )
        .map_err(MongooseDiagnosticError::CanConnection)?;
        let result = self.kline_exchange(transaction, route_id, timeout);
        let close = self.close_route();
        match (result, close) {
            (Ok(result), Ok(())) => Ok(result),
            (Ok(_), Err(error)) => Err(MongooseDiagnosticError::ChannelClose(error)),
            (Err(error), _) => Err(error),
        }
    }

    fn kline_exchange(
        &mut self,
        transaction: &PreparedKlineTransaction,
        route_id: VehicleRouteId,
        timeout: Duration,
    ) -> Result<MongooseKlineReadResult, MongooseDiagnosticError> {
        let request_payload = transaction.encoded_payload().to_vec();
        self.transmit_line_bytes(&request_payload)
            .map_err(MongooseDiagnosticError::RequestTransmission)?;

        // A serial line has no frame boundary the adapter reports, so what
        // comes back is gathered until the line goes quiet or the budget runs
        // out. Where a frame ends is the protocol's business.
        let started = Instant::now();
        let mut raw_line_bytes = Vec::new();
        while let Some(remaining) = timeout.checked_sub(started.elapsed()) {
            let gap = KLINE_QUIET_GAP.min(remaining);
            match self
                .receive_line_bytes(gap)
                .map_err(MongooseDiagnosticError::ResponseReception)?
            {
                Some(bytes) => raw_line_bytes.extend_from_slice(&bytes),
                // Quiet for a whole gap after something was heard is the end
                // of the answer; quiet from the start is silence.
                None if raw_line_bytes.is_empty() => continue,
                None => break,
            }
        }
        if raw_line_bytes.is_empty() {
            return Err(MongooseDiagnosticError::Timeout);
        }
        Ok(MongooseKlineReadResult {
            route: route_id,
            node_address: transaction.node_address(),
            request_payload,
            raw_line_bytes,
        })
    }
}

/// The transaction must describe exactly what this backend can do: a
/// read-only K-line read, on one of the adapter's own K-line routes, with
/// that route's pin, a baud rate the line states and a wake-up the adapter
/// can perform.
fn validate_kline_transaction(
    transaction: &PreparedKlineTransaction,
) -> Result<VehicleRouteId, MongooseDiagnosticError> {
    let TransactionSafetyClass::ReadOnly = transaction.safety_class();
    let route_id: VehicleRouteId = transaction.backend_route().parse().map_err(|_| {
        MongooseDiagnosticError::UnsupportedTransaction("unknown adapter route for a K-line read")
    })?;
    let Some(pin) = kline::pin_for_route(route_id) else {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "a K-line read must be on a K-line route",
        ));
    };
    if transaction.physical_pins() != [pin] {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "the plan's connector pin is not the route's own",
        ));
    }
    if kline::resource_for_protocol(transaction.protocol_family()).is_none() {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "this protocol is not one this product speaks on a K-line",
        ));
    }
    if transaction.bitrate_bps() == 0 {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "a K-line read needs the line's own baud rate",
        ));
    }
    if !kline::wakeup_is_supported(transaction.wakeup()) {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "the bus asks for a wake-up this adapter has not been shown to do",
        ));
    }
    Ok(route_id)
}
