# ADR-0027: the module passport — what a module says it is

- Status: Accepted, 2026-09-12
- Decision: a module's part numbers, serial number, hardware and software
  levels are read as one operation, `MODULE_PASSPORT`, class `READ_ONLY`,
  over the identification identifiers SDD's platform documents declare for
  that module on that car; the identifiers enter the knowledge base in the
  catalogue's own shape so a passport read is a module read, and their
  values are shown as the text they are.
- Reason: the first thing a workshop asks of a module after "does it
  answer" is "which one is it" — the part number that was fitted, the
  software it runs — and the product could read those identifiers one at a
  time, by hand, if the reader knew the numbers. SDD lists them per module.
- Consequence: one new slice of the platform ingest, one decoding rule, one
  service in the shell, one section of the report, one row in the safety
  registry. No new crate, no new safety class, nothing written.

## What SDD says, measured

Every `PLATFORM_<PROGRAM>_<YEAR>.xml` names, per module, three sets of data
identifiers: `NET` (the module's own set, `ggds_<module>`), `SWDL` (the
software part numbers, `ggds_swdl_N`) and `PDI` (the pre-delivery list,
`ggds_pdi_1`). Over the 55 documents this project reads: 3,168 set
definitions, 549 distinct identifiers, 5,671 references from modules to sets.
Every `NET` set opens with the same block SDD comments as *Core PIDs*:

| identifier | SDD's name | uses |
| --- | --- | --- |
| `0xF188` | ECU Software Number | 2,720 |
| `0xF111` | ECU Core Assembly Number | 1,936 |
| `0xF18C` | ECU Serial Number | 1,935 |
| `0xF191` | ECU Hardware Number | 1,924 |
| `0xF112` | ECU Assembly Number | 1,890 |
| `0xF113` | ECU Delivery Assembly Number | 1,889 |
| `0xF190` | Vehicle Identification Number | 1,880 |
| `0xF103` | Active Network Configuration Number | 1,829 |

and the `SWDL` and `PDI` sets add the further software and calibration part
numbers `0xF120`–`0xF128`, `0xF108`, `0xF0E8`, and the boot software
identification `0xF180`. Of 33,000 `<did>` declarations, 32,992 name service
`0x22`; the handful naming `0x09`, `0x1A`, `0x02`, `0x06` or `0x00` are not
reads this product makes and are left out. 348 `SWDL` references carry a
qualifier of their own — a DAB module has one software list for standard
hardware and another for DAB+ — and those narrow the row as a module's own
qualifier does.

The `NET` sets also carry identifiers that are not identification: the
total distance `0xDD01`, the legislated mirrors `0xF800`–`0xF80A`, a
module's own data such as `0x2B3C` or `0x4116`. They are seen and not
ingested by this slice: they come without a byte layout, and admitting them
would widen the module-read list — and the mileage survey — with rows the
decoder can only show as bytes. That is a separate decision.

## Decisions

1. **The passport is SDD's list for that module on that car.** The
   identifiers are the union of the module's `SWDL` and `PDI` sets and the
   `0xF100`–`0xF1FF` members of its `NET` set, service `0x22` only. Not a
   general ISO 14229 list: what a module answers is what SDD says it
   answers, and a module SDD gives no set is given none here. Fail closed,
   as `ADR-0009` has it.

2. **Recorded in the catalogue's own shape.** Each identifier becomes an
   `IdentifierParameter` record — `ClaimKey::ParameterDefinition` with SDD's
   own text for the name, `IdentifierDefinition` with the identifier and the
   encoding `text=ascii` — applicable to the programme, the marker, the
   module's qualifier and the set's, with `ecu_family` naming the module.
   The resolver's readable list and the transaction gate therefore admit a
   passport identifier exactly as they admit a catalogue one, and a passport
   read *is* a module read: the same prepared transaction, the same record,
   the same intake. No new path exists for it to take.

3. **A value is the text it is.** The encoding `text=ascii` says the whole
   payload is a string: printable ASCII is shown with the padding — NUL,
   `0xFF`, spaces — trimmed from the ends; anything else is shown as bytes
   with the reason. No part-number grammar is parsed, nothing is compared
   with a catalogue, and no word such as *outdated* or *wrong* appears. The
   product reads what the module holds and puts it on the screen.

4. **One operation, the mileage survey's shape.** `MODULE_PASSPORT`, class
   `READ_ONLY`, schema `prowlone.module-passport`. The shell plans one read
   per module and identifier from the loaded library and owns the rules;
   the interface asks for one step at a time and can stop between any two;
   every read leaves the record a single read leaves, and the intake reads
   them as `module_passports[i].reads[j]` through the path a module read
   goes through. A run covers one chosen module or every module the survey
   reaches.

5. **Names on screen are the product's own.** The label a reader sees for
   `0xF111` is this project's wording in the interface language, keyed by
   the identifier (`ADR-0025`'s rule: a dictionary, not knowledge). The
   report keeps SDD's English parameter name as the identity of the row.

6. **The bench answers with plausible text.** For an identifier whose
   encoding is text the bench answers a deterministic part-number-shaped
   string, and the VIN identifier the bench's VIN, so the flow is
   exercised end to end with values that look like the real thing and are
   marked `SYNTHETIC` like every bench value (`ADR-0020`).

## What this does not decide

- Whether a module's software level is current: SDD's `IVS` part lineage
  could say which numbers supersede which. Reading it is a separate
  decision, and comparing would need the rule for what "current" means
  for a car no dealer has touched in years.
- Whether the `NET` sets' other identifiers are ingested (see above).
- Nothing about writing: a part number is read, never set. Module
  replacement set-up is stage 3 and needs its own ADR.

## Built, 2026-09-12

`IMPLEMENTED / FIXTURE_TESTED`; nothing has met a car.

- **Ingest.** `PlatformAdapter::add_identification` reads the root's named
  `data_identifier_set` definitions once and, per module, the members of its
  `SWDL` and `PDI` sets and the `0xF100`–`0xF1FF` members of its `NET` set,
  service `0x22` only, each an `IdentifierParameter` record valued
  `text=ascii` under SDD's own text for the name, narrowed by the module's
  qualifier and the set's. Real source: 34,788 records over 45 distinct
  identifiers, 99 module families and 21 programmes; the library exported
  afresh holds 314,462 records against 279,674 the day before, nothing
  rejected; the owner's copy was re-stamped. Golden test:
  `identification_identifiers_are_recorded_per_module_in_the_catalogue_shape`.
- **Decoding.** `text=ascii` is a third shape beside the scaled and the
  mapped: the payload trimmed of NUL, `0xFF` and spaces at both ends is the
  value when printable, the bytes with the reason when not, and a reason
  when nothing is left. `is_text` lets the bench and the library ask the
  encoding rather than a byte span. Test: `a_text_identifier_is_the_string_it_holds`.
- **Library.** `KnowledgeLibrary::identification_identifiers` lists a
  module's text identifiers with SDD's names; the survey's readable list
  shows them beside the catalogue's, so an unreachable module still says
  what it would hold.
- **Bench.** A text identifier answers a part-number-shaped string with a
  `BN` prefix no real part carries, the same for the same module and
  identifier; the VIN identifier answers the bench's VIN. Every row is
  `SYNTHETIC` (ADR-0020).
- **Shell.** `PassportService` in the mileage survey's shape: the plan from
  the loaded library, one step at a time, a stop between any two, the record
  a single read leaves per request, the report `prowlone.module-passport`
  with `readings`, `reads` and `not_planned`; four commands; the session
  bundle's `module_passports`. The end-to-end bench run reads SYNTHMOD's five
  identifiers, refuses OTHERMOD with the resolver's reason, and fails if a
  bench row ever reads `SOURCE_BACKED` or the report ever says *outdated*.
- **Intake.** `module_passports[i].reads[j]` through the builder a module
  read goes through — the same confirmation, no more; the texts themselves
  are not evidence about a route and are not recorded.
- **Interface.** A panel under the mileage one, one table per module — our
  label, SDD's name beneath, the address, the text as held — and a button
  beside *Read the mileage*; the labels for 38 identifiers in three
  languages, keyed by the identifier, none for an identifier SDD names
  inconsistently across modules (`0xF108`, `0xF0E8`). Three interface tests.
- **Registry.** `module_passports` in `SAFETY_BOUNDARIES.md`, class
  `READ_ONLY`.
