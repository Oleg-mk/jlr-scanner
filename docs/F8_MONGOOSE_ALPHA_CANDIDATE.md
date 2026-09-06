# F8 — Tester-Ready Mongoose Alpha Candidate

F8 status: `TESTER_ALPHA_CANDIDATE_READY / IMPLEMENTED / SIMULATOR_TESTED /
REPLAY_TESTED / TRANSPORT_FAKE_TESTED / NO_VEHICLE_INTERACTION`.

Live vehicle status: `NOT_YET_EXTERNALLY_VALIDATED`.

Distribution status: `READY_FOR_SIGNING / DISTRIBUTION_BLOCKED_BY_F3.1`.

## Supported alpha slice

The first alpha intentionally supports one evidence-backed profile and one
explicit read-only operation:

- Jaguar XF / X250, model year 2010, 5.0L Supercharged;
- ECM / PCM target `jlr.x250.ecm`;
- MongoosePro JLR `18E1:0104` over the OS USB/serial driver;
- HS-CAN, J1962/C2DB04B pins 6/14, 500000 bit/s, 11-bit CAN;
- physical request `0x7E0`, expected response `0x7E8`;
- SAE J1979 Mode 09 InfoType 04 Calibration Identification;
- expected evidence-backed golden `CX23-14C204-ZAD`.

No other JLR model, X250 variant, adapter, vehicle operation, or transport is
represented as live-supported.

## Application flow and safety

The React/Tauri application automatically discovers MongoosePro JLR. Opening
the USB connection and obtaining board-info remain explicit and use the F3
production path. Adapter discovery, startup, and board-info do not open a
vehicle CAN channel and do not send a diagnostic request.

The only diagnostic Tauri intent is the argument-free
`read_calibration_identification` command. The UI cannot provide a CAN ID,
service, DID, payload, route, bitrate, or protocol. After the user presses
**Read Calibration ID**, the backend loads the one supported product profile,
passes its F6 evidence-backed RESOLVED environment to the existing F7
transaction compiler, and gives that immutable READ_ONLY
`PreparedDiagnosticTransaction` to `mongoose-jlr`.

The Mongoose backend keeps active channel setup and `cOutboundData` private. It
opens only the confirmed HS-CAN descriptor, verifies pins 6/14 and 500000 bit/s,
correlates `0x8008` transmit acknowledgements, sends the fixed ISO-TP single
frame for `09 04`, handles ISO-TP flow control for a multi-frame response,
accepts only the evidenced responder, reuses the shared ISO-TP reassembler and
`obd-j1979` decoder, and closes the channel. There is no public `send_raw`,
`send_can`, `send_bytes`, `send_payload`, or arbitrary execution API.

Application arbitrary CAN TX API remains `NO`. Pins 12/13 remain absent from
production routes and cannot be selected by this workflow.

## Offline and hardware-free acceptance

The application service executes the same prepared product transaction through
the existing simulator and deterministic replay fixture. Both return exactly
`CX23-14C204-ZAD`. A byte-transport fake covers active channel open, pins 6/14,
the fixed request, `0x8008` acknowledgements, ISO-TP flow control, three-frame
response reassembly, J1979 decode, and channel close.

No vehicle was connected. No live CAN channel was opened during F8 acceptance.
The real USB/board-info chain remains the separately accepted F3 evidence:
Windows 11 -> usbser -> production Tauri/Rust -> MongoosePro JLR -> `0x8109`.

## Diagnostic report

Every completed or failed user-triggered attempt produces schema-versioned JSON
that the user can save locally. It includes application and OS version,
timestamp, generated session ID, adapter model and VID/PID, board-info result,
vehicle profile, ECU target, logical/physical route, bitrate, protocol,
request/response IDs, capability, fixed request payload, actual responder, raw
diagnostic response, decoded result, error category, and failure stage.

The report excludes adapter serial number, COM port, VIN, account data, and
unrelated personal information. Nothing is uploaded automatically.

## Distribution and platforms

Normal CI builds an explicitly unsigned NSIS Windows 10/11 alpha artifact.
This proves packaging readiness; it does not make the artifact publicly
trustworthy. The existing signing-ready F3.1 workflow and signing configuration
are unchanged. No signing identity or external signing resource was created.

Therefore:

- alpha software: `READY`;
- Windows installer candidate: `UNSIGNED / READY_FOR_SIGNING`;
- public distribution: `NO — DISTRIBUTION_BLOCKED_BY_F3.1`;
- F3.1: `DEFERRED_EXTERNAL_DEPENDENCY — requires trusted signing identity`.

CI also compiles the Tauri application on macOS without hardware. That may
establish `MACOS_BUILD_COMPATIBLE`; it does not establish macOS Mongoose runtime
support. macOS Mongoose runtime validation remains `NO`.

## Explicit exclusions

F8 adds no Bluetooth, Wi-Fi, ELM327, VXDIAG, OpenPort, SDD, Drew/J2534 runtime,
ECU discovery, `0x7DF` scan, VIN, DTC, Mode 22, live data, CCF, actuator,
routine, SecurityAccess, programming, flashing, or raw CAN console.

See ADR-0008 for the typed live boundary decision.
