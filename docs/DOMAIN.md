# Domain Contract

## Product scope

JLR Scanner targets the whole Jaguar, Land Rover, and Range Rover SDD era.
No single vehicle program defines the product architecture. X250 remains one
reference vehicle program, not the root of a knowledge tree and not a protocol
special case.

## Semantic identity and applicability

A canonical diagnostic meaning is independent from how any vehicle supplies
it. A future diagnostic implementation can apply across several programs and
year ranges without duplicating the implementation under every model.

Applicability is multi-dimensional and must support independent axes:

- vehicle_program as an open string;
- model_year_from and model_year_to;
- architecture_generation;
- ecu_family;
- powertrain and variant;
- market;
- diagnostic_implementation;
- evidence_state;
- validation_state.

These axes belong to JLR knowledge and evidence. CAN, ISO-TP, UDS, replay, and
simulator logic must not branch on them.

## Evidence states

- vehicle_test_confirmed
- raw_log_confirmed
- source_code_confirmed
- documentation_confirmed
- secondary_source
- candidate
- negative_evidence
- unknown

Applicability and evidence strength are separate dimensions. Evidence for one
model year or program must not be generalized automatically. Unknown remains
unknown until evidence changes it.

Synthetic fixtures prove protocol behavior only. Documented fixtures require
source provenance and validation metadata. Captured fixtures must represent
real sanitized captures and carry provenance and validation metadata. Synthetic
data must never be presented as JLR evidence.
