# Safety Boundaries

Every diagnostic operation must have one explicit safety class. If classification, applicability, or procedure validation is absent, the operation is unavailable: the system fails closed.

## Operation classes

### `READ_ONLY`

Identification, DTC read, live data, configuration read, and passive CAN monitoring.

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
- Live Mongoose execution accepts only a validated READ_ONLY
  `PreparedDiagnosticTransaction`; the frontend cannot provide CAN IDs, raw
  bytes, service numbers, DIDs, routes, or bitrates.
- Automatic adapter detection, USB connection, and board-info never authorize
  or trigger vehicle CAN transmission.
