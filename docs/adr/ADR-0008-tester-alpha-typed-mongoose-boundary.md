# ADR-0008: Tester alpha typed Mongoose execution boundary

- Status: Accepted
- Decision: Add an F8 application workflow that always resolves the single
  supported X250 profile through the F6 environment resolver and prepares the
  existing F7 `PreparedDiagnosticTransaction`. The production Mongoose device
  may execute exactly that prepared Calibration Identification transaction;
  no lower-level live CAN send surface is public. The Tauri/UI boundary exposes
  one argument-free user intent, `read_calibration_identification`, plus local
  report retrieval.
- Reason: F8 needs a future-testable physical Mongoose path while preserving the
  invariant that discovery, adapter connection, and frontend input cannot
  choose CAN identifiers, protocol bytes, services, DIDs, or arbitrary payloads.
  Keeping the fixed outbound serializer and active channel lifecycle private to
  `mongoose-jlr` prevents a typed application request from degrading into a raw
  transmit API.
- Consequence: The application opens HS-CAN only after the user presses the
  read button. The prepared transaction supplies route, bitrate, pins, CAN-ID
  width, request ID, expected responder, capability, and protocol identity.
  ISO-TP and SAE J1979 remain shared protocol components. Simulator, replay,
  and a transport fake exercise the same application workflow. Reports are
  created locally, contain no unrelated personal data, and are never uploaded.
  F8 does not connect to a vehicle and does not claim live vehicle validation.
  Windows signing remains the unchanged F3.1 external dependency.

## Safety constraints

- The only live operation is SAE J1979 Mode 09 InfoType 04 Calibration
  Identification, safety class `READ_ONLY`.
- No diagnostic command is sent at startup, during USB discovery, or when the
  adapter is connected for board-info.
- The Mongoose live entry point accepts only `PreparedDiagnosticTransaction`.
- No public `send_raw`, `send_can`, `send_bytes`, `send_payload`, or arbitrary
  diagnostic execution API is added.
- No functional `0x7DF` scan, ECU discovery, write/control, SecurityAccess,
  routine, programming, flashing, pin 12/13 route, or raw CAN console exists.

## Validation boundary

F8 acceptance uses simulator, replay, transport fakes, UI tests, packaging, and
the already validated USB-only board-info path. Active vehicle CAN remains
`NOT_YET_EXTERNALLY_VALIDATED` until a separate controlled external test.
