# Safety Boundaries

Every diagnostic operation must have one explicit safety class. If classification, applicability, or procedure validation is absent, the operation is unavailable: the system fails closed.

## Operation classes

### `READ_ONLY`

Identification, DTC read, live data, configuration read, and passive CAN monitoring.

The operations the product has today, every one of this class, by what its
record in the session bundle is called:

| bundle field | record schema | what it is | decided |
| --- | --- | --- | --- |
| `captures` | — | a listen on one bus for a stated time; nothing is transmitted | F8 |
| `calibration_reads` | — | SAE J1979 mode 09 calibration identification | F4–F7 |
| `module_reads` | — | one `ReadDataByIdentifier` or `ReadDTCInformation` to one module | F10, `ADR-0012` |
| `standard_obd_reads` | `prowlone.standard-obd-read` | the legislated J1979 services | `ADR-0022` §7 |
| `live_read_runs` | `prowlone.live-read-run`, operation `LIVE_READ` | a module read repeated at a stated cadence, floor and cap in the shell | `ADR-0022` |
| `mileage_surveys` | `prowlone.mileage-survey`, operation `MILEAGE_SURVEY` | one `ReadDataByIdentifier` per module the data names a distance for | `ADR-0024` |
| `module_passports` | `prowlone.module-passport`, operation `MODULE_PASSPORT` | one `ReadDataByIdentifier` per identification identifier the platform declares for a module — part numbers, serial, software and hardware levels, shown as the text they are | `ADR-0027` |
| `ccf_reads` | `prowlone.ccf-read`, operation `CCF_READ` | one `ReadDataByIdentifier` per module and identifier holding a block of the car configuration file, sync module first and then each copy, decoded with SDD's own layout; every value the type it is, a copy's difference two readings side by side, nothing judged; the write service SDD names beside every block is not recorded | `ADR-0028` |

An operation not in this table does not exist in the product. Adding one
means adding it here, with its class, in the same change. The first three
carry no schema of their own inside the bundle; that is a gap this table
makes visible rather than a decision.

### `VOLATILE_CONTROL`

Actuator commands whose effects end with the session or power cycle.

### `SERVICE_ROUTINE`

Explicit, validated service procedures.

### `PERSISTENT_CHANGE`

Adaptations or configuration changes. These are allowed only in the future Engineering level, with explicit user confirmation and a validated procedure.

### `FORBIDDEN_PROGRAMMING`

ECU firmware/VBF programming, bootloader or recovery flashing, immobilizer/key programming, and security programming. These operations are outside the product scope.

## Non-negotiable rules

- No diagnostic write operation exists without an explicit safety classification.
- No automatic vehicle-changing action occurs on app startup, device discovery, transport connection, or vehicle identification.
- Discovery and connection are never authorization to write.
- Unknown or unvalidated procedures fail closed.
- Live Mongoose execution accepts only a validated READ_ONLY transaction —
  a `PreparedDiagnosticTransaction` on the J1979 path, a
  `PreparedUdsTransaction` on the UDS path (`ADR-0012`, `ADR-0015`); the
  frontend cannot provide CAN IDs, raw bytes, service numbers, DIDs, routes,
  or bitrates.
- Automatic adapter detection, USB connection, and board-info never authorize
  or trigger vehicle CAN transmission.
- The K-line protocols (`ADR-0029`) carry the same rule as UDS: the `ds2`
  crate has no clear-fault request (`0x05`) and the `kwp2000` crate no
  `ClearDiagnosticInformation` (`0x14`), no `StartDiagnosticSession`, no
  `SecurityAccess`, no routine, no write, no download or upload; the frame
  encoders are private and the architecture check names the forbidden
  constructors. `StartCommunication 0x81` is the link handshake ISO 14230
  requires, not a diagnostic session. Until the second slice's probe has run
  on the adapter, the K-line routes refuse every open.
