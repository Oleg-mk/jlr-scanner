# F7 Evidence-Backed Read-Only Diagnostic Execution

## Status and boundary

F7 status: **PASS / IMPLEMENTED / REAL_F6_ENVIRONMENT_GOLDEN_TESTED /
SIMULATOR_TESTED / REPLAY_TESTED / CI_TESTED / NO_VEHICLE_INTERACTION**.

F7 compiles one F6 RESOLVED evidence-backed environment and one typed
read-only intent into an immutable offline transaction. It executes only
against `diagnostic-simulator` or `transport-replay`. It cannot open Mongoose,
open a live CAN channel, transmit a frame, discover an ECU, accept an arbitrary
payload, or expose a frontend diagnostic API.

## Architecture decision

ADR-0007 records the F7 boundary. The audit found that `diagnostics-core` is
structurally UDS-specific: its public transaction takes `UdsRequest` and its
result contains `uds::TypedDiagnosticResult`. F7 therefore does not force SAE
J1979 into UDS types and does not broadly rewrite the F4 core.

    diagnostic-environment (F6 data-only plan)
                    |
                    v
           diagnostic-execution
              |             |
              v             v
         obd-j1979         isotp
              \             /
               read-only CAN frames
                  /       \
          simulator       replay

`obd-j1979` has no knowledge, transport, JLR, UDS, or UI dependency.
`diagnostic-execution` is the only plan-to-protocol boundary. Knowledge,
diagnostic-environment, transport, ISO-TP, UDS, and diagnostics-core do not
depend on it.

The existing simulator keeps its typed UDS API. Its new protocol-neutral
payload seam is confined to offline fixture construction and still delegates
all segmentation to the existing `isotp` crate.

## Typed read-only intent

F7 exposes exactly one application intent:

    ReadOnlyDiagnosticIntent::CalibrationIdentification

The intent includes an explicit target ECU family and diagnostic
implementation. There is no `send(bytes)`, raw CAN, generic service, DID,
write, control, reset, security, routine, or programming intent.

The intent compiles only when the resolution is `RESOLVED` and the plan proves
the exact read-only capability
`obd.service09.infotype04.calibration_id.read_only` for the requested target.

## Prepared transaction

`PreparedDiagnosticTransaction` retains validated:

- `READ_ONLY` safety classification;
- target ECU family and diagnostic implementation;
- HS-CAN logical network and J1962/C2DB04B pins 6/14;
- offline route identity and 500000 bit/s bitrate;
- ISO 15765-4 / SAE J1979 protocol family and normal physical addressing;
- explicit 11-bit physical request `0x7E0`;
- explicit independently evidenced response `0x7E8`;
- functional address `0x7DF` as metadata;
- typed Mode 09 InfoType 04 request identity;
- observed calibration marker and field-level F6 evidence traces.

Encoded `09 04` bytes remain inside the typed protocol request. The prepared
transaction offers no arbitrary payload mutation or transmission method.

## Addressing policy

F7 target execution uses the explicit physical pair:

    request metadata: 0x7E0
    expected response: 0x7E8

The resolved ECU is already known, so F7 does not broadcast `0x7DF` and does
not collect multiple responders. `0x7DF` remains preserved in the plan,
transaction, result, and provenance as the documented functional address.

No response address is calculated from the request address. There is no `+8`
rule. `0x7E0` and `0x7E8` are consumed from distinct F6 fields with distinct
evidence traces.

## SAE J1979 codec

`obd-j1979` implements only Mode 09 InfoType 04 Calibration Identification.
The request is `09 04`. The decoder requires:

- positive response SID `0x49`;
- InfoType `0x04`;
- non-zero item count;
- exactly `count * 16` calibration bytes after the three-byte header;
- printable ASCII content;
- trailing NUL padding only;
- no truncated, surplus, empty, or internally padded fields.

This layout follows the retained ELM327DSJ pages 42-43 Mode 09 examples: the
third application byte is the number of data items and Calibration ID uses a
16-byte field. ISO-TP framing is not implemented in this codec.

## Positive simulator golden

The golden starts from the real F6 source-backed 2010 Jaguar XF X250 5.0L
Supercharged ECM environment. A synthetic response reproduces the independently
observed calibration value `CX23-14C204-ZAD` in one NUL-padded 16-byte field.

The existing ISO-TP implementation segments the 19-byte J1979 response into a
First Frame and two Consecutive Frames. The execution bridge reassembles those
frames, verifies responder `0x7E8`, decodes J1979, and returns exactly
`CX23-14C204-ZAD`.

The response frames are synthetic. They are not represented as an X250 vehicle
capture and do not change the validation state of the real environment.

## Replay golden

`fixtures/synthetic/f7_x250_mode09_calibration_multiframe.json` contains the
same three machine-readable CAN frames and is explicitly classified
`synthetic`. Deterministic replay passes through the same ISO-TP reassembler and
J1979 decoder and returns the same typed result on repeated runs.

The completed result envelope keeps separate:

- environment evidence provenance from F6;
- execution source (`SIMULATOR` or `REPLAY`);
- synthetic execution fixture identity;
- target and explicit request/response identities;
- typed calibration values.

## Fail-closed behavior

Preparation rejects `INDETERMINATE` and `CONFLICT` as distinct errors. It also
rejects target mismatch, capability mismatch, unsupported protocol, empty
route fields, zero bitrate, unsupported addressing mode, invalid CAN ID width,
and missing observed calibration provenance.

Execution rejects a responder other than `0x7E8`, wrong SID, wrong InfoType,
malformed count/length/encoding/padding, invalid ISO-TP sequence, truncation,
timeout, empty fixture identity, and non-synthetic replay input for this F7
path.

## Explicit non-goals

F7 adds no vehicle or Mongoose execution, live CAN source, arbitrary payload
API, functional multi-responder discovery, ECU scan, DID enumeration, Mode 22,
VIN, DTC workflow, DTC clearing, UDS expansion, programming, security access,
routine control, actuator control, reset, write operation, or frontend change.

Application CAN TX API remains **NO**.

## Future live boundary

A later explicitly authorized phase may design a separately reviewed live
transport adapter for already prepared read-only transactions. F7 deliberately
rejects that source class and provides no conversion to Mongoose commands or a
CAN transmitter.
