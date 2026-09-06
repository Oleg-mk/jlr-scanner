# F4 Replay, Simulation, ISO-TP, and UDS

## Status and safety

Acceptance status: **PASS**. GitHub Baseline validation run 33318590434 passed
the complete frontend and Windows Rust jobs for implementation commit a166033.

F4 builds a fully offline diagnostic execution path:

    ReplaySource or SimulatorSource
                 |
            raw CAN frames
                 |
              ISO-TP
                 |
               UDS
                 |
        typed diagnostic result

No vehicle or Mongoose device was used. No CAN channel was opened. No
application CAN TX API, ECU discovery, live UDS, security access, routine
control, write, reset, download, transfer, flashing, or programming surface was
added.

## Raw CAN source contract

transport-api owns validated classic CAN identifiers, timestamps, route
identity, DLC/data bounds, and the read-only CanFrameSource contract. Source
kind is LIVE, REPLAY, or SIMULATOR. Source choice does not change ISO-TP or UDS
logic.

transport-replay accepts schema version 1 JSON fixtures. It validates standard
and extended identifiers, route, classic DLC, data length, monotonic
microsecond timestamps, metadata, malformed input, and end of stream.
Deterministic mode performs no sleeps. Real-time mode reproduces timestamp
offsets.

Fixture metadata is an opaque envelope supporting vehicle program, year,
architecture generation, ECU family, powertrain or variant, market,
diagnostic implementation, evidence, and validation. Protocol code does not
read these fields. Documented and captured fixtures fail closed without
evidence and validation. The F4 golden fixture is explicitly synthetic and
generic.

## Simulator

diagnostic-simulator maps one typed expected UDS request to a deterministic
behavior and returns raw CAN frames. Supported behaviors are positive response,
negative NRC response, timeout, multi-frame response, malformed sequence, and
delayed response. It is a focused ECU-response test double, not a vehicle
simulator.

## ISO-TP core

The vehicle-independent classic-CAN core implements normal addressing:

- Single, First, Consecutive, and Flow Control frames;
- segmentation and reassembly up to the classic 12-bit length limit;
- sequence checking and wrap;
- block size and separation time, including millisecond and 100-microsecond encodings;
- sender and reassembly timeouts;
- optional padding and declared-length trimming;
- malformed, truncated, oversize, unexpected, and wrong-sequence rejection.

CAN identifier and route selection remain outside ISO-TP. Extended and mixed
addressing are future extensions, not vehicle-program branches.

## UDS core

The generic parser correlates positive service identifiers and 0x7F negative
responses, parses common NRC values, and preserves arbitrary response payload.
Typed request/result support is intentionally limited to:

- 0x10 DiagnosticSessionControl;
- 0x19 ReadDTCInformation;
- 0x22 ReadDataByIdentifier;
- 0x3E TesterPresent.

The public request API has no raw arbitrary-service constructor and no 0x11,
0x27, 0x2E, 0x31, 0x34, 0x36, flashing, or programming constructor.

## Golden paths

The simulator golden test scripts a generic 0x22 request, creates a positive
multi-frame raw CAN response, reassembles ISO-TP, correlates UDS, and returns a
typed DID result.

The replay golden test loads
fixtures/synthetic/f4_replay_uds_multiframe.json and executes the same CAN to
ISO-TP to UDS path twice with byte-for-byte deterministic results.

No JLR-specific fixture is used in F4. The documented and captured directories
contain policy files only.

## SDD-era model

The product covers the whole JLR SDD era across Jaguar, Land Rover, and Range
Rover. Future applicability is program plus year range plus architecture plus
ECU family plus variant plus market plus evidence and validation. Vehicle
program is open-ended. X250 is only one possible reference program.

SDD-era diagnostics are not UDS-only. KWP, ISO 9141, and other legacy protocol
work remains a separate phase and is not forced through this UDS core.

## Verification

Focused Rust tests cover raw CAN validation, replay rejection and end of
stream, all simulator behaviors, ISO-TP framing/flow-control edge cases, UDS
positive/negative correlation, and both golden end-to-end paths. The repository
architecture check covers dependency direction, no vehicle-program hardcoding,
no CAN TX, and no out-of-scope active UDS constructors.

Protocol references:

- ISO 15765-2:2024, Road vehicles, diagnostic communication over Controller Area Network, transport protocol and network layer services.
- Linux kernel ISO-TP documentation for SF, FF, CF, FC, block size, STmin, and sequence-error behavior.
- AUTOSAR Diagnostic Communication Manager specification for the typed UDS service identifiers used in F4.

## Deferred

Real vehicle UDS, Mongoose integration with the protocol core, JLR knowledge
ingestion, KWP and legacy protocols, ECU discovery, diagnostic UI, live data,
and all programming/security work are outside F4.
