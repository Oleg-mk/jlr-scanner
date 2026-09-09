# Product Definition

## Product

ProwlOne is a modern, independent diagnostic scanner for Jaguar and Land Rover vehicles of the SDD generation. It owns its diagnostic engine, JLR knowledge base, protocol stack, transport abstraction, and adaptive UI.

It is not an SDD clone or frontend, a Mongoose-only program, a generic ELM scanner, or an ECU flasher. SDD may later serve only as a research/reference source for verification and is never a runtime dependency.

## Product levels

### Free diagnostic level

- Automatic vehicle identification and topology/ECU discovery.
- Scan supported ECUs; read and clear DTCs.
- DTC status and freeze-frame data where supported.
- ECU identification, VIN, software, and version data.
- Live data, graphs, and a diagnostic report.

### Future Engineering level

- Manufacturer-specific extended live data.
- Validated actuator tests, service routines, adaptations, and resets.
- Appropriate calibration operations and CCF read/analysis.
- CAN monitor, raw UDS/KWP console, advanced logging, and engineering parameters.

No subscription or billing implementation belongs to F0.

## Explicit exclusions

- ECU firmware flashing, VBF programming, or bootloader/recovery flashing.
- Key, immobilizer, or security programming.
- Arbitrary EEPROM modification.

## Platforms

Windows 11 is the first development/validation platform, followed by Windows 10 and later macOS. Android, iOS, and tablets use the same adaptive application and diagnostic core over `NetworkTransport`; mobile never requires direct USB Mongoose access.
