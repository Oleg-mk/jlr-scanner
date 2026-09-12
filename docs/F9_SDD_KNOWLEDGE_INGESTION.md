# F9 — SDD knowledge extraction and ingestion

F9 status: **the surveyed SDD corpus is fully ingested, and the addressing gap
is closed.** The addressing, DID-catalogue, DTC, ODST, DTC-index, and platform
slices are `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`.

Derived knowledge is `SourceBacked` documented evidence. Nothing in F9 is
vehicle-confirmed, and ingesting a manufacturer document never makes it so.

See `ADR-0009` for the architectural decision and
`docs/research/sdd/PROVENANCE.md` for the source survey.

## What F9 adds

The `sdd-ingest` crate implements the existing F5 `IngestionAdapter` extension
point. It depends on `knowledge` and a read-only XML parser only, and no crate
depends on it. `knowledge` remains unaware of SDD.

Two rules are enforced in code rather than left to review:

- **Classification follows the source type.** Evidence class and validation
  state are derived from the registered `SourceType`, so a synthetic fixture
  produces `SyntheticTest` / `Unverified` and a documented source produces
  `OemDocumentation` / `SourceBacked`. A fixture cannot be laundered into JLR
  evidence by a parser change or a copy-paste. A single source never claims
  `Corroborated`; that requires a second independent source and is decided by
  the store.
- **Only what the source states.** Values absent from the file stay absent. The
  addressing table states a CAN width, never a request identifier, so
  `request_id` remains `None` rather than being inferred from a mnemonic.

## First slice: `CANLinkMonitorData.xml`

`CanLinkMonitorAdapter` parses the SDD CAN link monitor configuration into two
independent claim families.

**Vehicle addressing width** — entity `VehicleProgram`, claim
`DiagnosticAddressing`, value carrying `CanIdFormat` and the raw mode string.
Applicability constrains the vehicle program; every other dimension is `Any`
because the file states the width for the program as a whole.

**Module alias** — entity `DiagnosticAddressing` keyed by the mnemonic, claim
`Alias`, value the module description. Applicability is left unknown on every
dimension. The table is global and never says which program a mnemonic belongs
to, so these records **fail closed** and resolve as `InsufficientEvidence` for
any specific vehicle. Recording them as applicable everywhere would have been a
fabrication.

### Real-source run

| | |
| --- | --- |
| evidence records | 161 |
| knowledge records | 161 |
| vehicle addressing claims | 25 |
| module alias claims | 136 |

`X250` model year 2010 resolves to `Standard11Bit` / `11bit` as `Applicable`.
The `7E0` alias reads `Powertrain control module` and resolves as
`InsufficientEvidence` for X250, which is the intended fail-closed outcome.

Both parameters independently corroborate the F8 prepared transaction, which
derived 11-bit addressing and `0x7E0` for the X250 ECM/PCM from separate F6
evidence. They are retained as two sources, not merged into one claim.

### Two things only real data revealed

The synthetic fixture did not predict either, and both are now golden-tested.

**Prose model-year markers.** The file uses `Post MY10` and `Pre MY10` besides
`ALL` and numeric years. Whether those include model year 2010 is stated
nowhere. The record is kept, the marker is preserved verbatim in the evidence
locator, and `YearConstraint` stays `Unknown` so it cannot resolve for a year.

**Reused mnemonics.** Ten mnemonics appear twice with different descriptions —
`90` is both `Bluetooth telephone module` and `Cellular telephone control
module`. Occurrences get distinct record ids while keeping the same entity and
claim key, so the store reports a `ConflictReport` instead of one row silently
overwriting the other. An exact repeat is ignored as carrying no new
information.

## Second slice: the DID formatting catalogue

`ConverterCatalogue` reads SDD converter definitions and retains the declared
output quantity and unit. The conversion arithmetic stays in SDD; this crate
records what a value means, not how to compute it.

`DidFormattingAdapter` parses the six `gradex/Snapshot` catalogues. Every
parameter element in that corpus is a `ReadParameter`, so it is stage-1 material
by nature rather than by filtering — and **any other element is rejected, not
skipped**, so a future write path cannot enter unnoticed.

Each parameter becomes an `IdentifierParameter` record keyed by parameter name,
valued as an `IdentifierDefinition` carrying the DID, a deterministic encoding
descriptor (`bytes=0..1;size=2;mask=0xffff;converter=CVT_...`), and the unit
resolved through the converter. A converter the catalogue does not hold leaves
the unit absent rather than invented. Since F11 (M1) the descriptor also
carries the converter's arithmetic verbatim — `scale=`, `offset=` and
`offset_first=` for a linear converter, `map=x:y|x:y|…` for a map converter,
and `states=low..high=name|…` for the converter's named raw-count ranges
(`quantityState`, names escaped) — so a value can be presented as SDD presents
it; see `F11_TESTER_APPLICATION.md` for what is applied and what is withheld.

### Qualification maps onto applicability

SDD qualifies a DID with a flat `AND` of string equality tests. All 3,220 fully
qualified entries use exactly that shape, with no disjunction anywhere.

| SDD tactic | Applicability dimension |
| --- | --- |
| `module` | `ecu_family` |
| `model` (`CM_MODEL`) | `vehicle_program` |
| `type` (`CM_TYPE`) | `powertrain` |
| `subType` (`CM_SUBTYPE`) | `variant` |
| `year` (`CM_YEAR_BREAKPOINT`) | `other["sdd_year_breakpoint"]` |

An unrecognised tactic or a disjunction stops ingestion. Ignoring either would
widen applicability past what the source states, which is the one failure mode
this design exists to prevent.

**Model years stay unknown on purpose.** SDD qualifies by a breakpoint such as
`MY10`, and the published data never says whether that means the 2010 model year
alone or 2010 and later. The marker is recorded verbatim on its own dimension
and `model_year` remains `Unknown`, so these records do not resolve for a
specific year. Establishing breakpoint semantics is a separate evidence question
and would upgrade the corpus on re-ingest.

### Real-source run

| | |
| --- | --- |
| converters loaded | 1,854 |
| parameter definitions | **15,281** |
| distinct DIDs | 1,385 |
| parameters carrying a unit | 15,281 (100%) |

Per catalogue: 5,687 fully qualified bit, 3,220 fully qualified, 2,881
unqualified bit, 2,023 module qualified bit, 925 unqualified, 545 module
qualified. Nothing was rejected. Units are real engineering units — V, pct, km,
degC, A, s, Nm, rpm, kph, Pa.

## Third slice: DTC descriptions

`DtcHelpAdapter` parses the per-code `rdsDtcHelp*.xml` documents. Each is
self-contained: the fault code, the description texts, and the qualifiers that
say which module, model, and model-year designation each description belongs to.

`ADR-0010` adds `EntityKind::DiagnosticTroubleCode` for this. None of the
existing kinds describes a fault code, and overloading one would make queries
mean something other than what they say. The addition is non-breaking because
`EntityKind` is only ever compared for equality in this workspace.

A description is recorded **per qualifier**, so applicability stays exact rather
than collapsing into the union of everything a file mentions. Qualifier
attributes map as `module` to `ecu_family` and `model` to `vehicle_program`,
with `year`, `dtcType`, and `moduleDataName` recorded verbatim on their own
custom dimensions. An attribute the parser does not recognise stops ingestion.

### The model-year decision, validated by the data

Designations are recorded verbatim and `model_year` is left `Unknown`. The
temptation was to map `MY##` onto `2000 + ##`, which would have looked correct
on the DID corpus.

The real DTC corpus spans **`MY94` to `MY17`** — it covers 1990s Jaguars such as
XJS and X300. That mapping would have placed a quarter of the corpus in the
2090s. The literal `BASE` also appears as a year value 4,856 times and carries
no stated meaning. Neither is interpreted.

The consequence is deliberate and worth stating plainly: these records **filter
exactly** by designation but never resolve as `Applicable`, because a record
whose `model_year` is unknown cannot answer a calendar-year question. Both
behaviours are golden-tested.

### Real-source run

| | |
| --- | --- |
| files scanned | 6,168 |
| files rejected | 0 |
| knowledge records | **93,521** |
| distinct fault codes | 6,168 |
| distinct modules | 119 |
| distinct vehicle programs | 28 |
| model-year designations | 33 |
| conflicts reported | 897 |

The 897 conflicts are real disagreements in SDD's own data, surfaced rather than
resolved.

One real fault code, `C!A67`, contains punctuation that cannot form a stable
entity id. An earlier revision rejected the whole document for it. That rule was
replaced while adding the index slice, because the same code appears in a
4,031-entry index where rejecting the file would discard thousands of sound
records over one malformed key. The identifier is now normalised while the raw
code stays verbatim in the evidence locator and excerpt, so nothing is lost and
nothing is invented.

Programs covered include X250, plus L316, L319, L320, L322, L359, L405, L494,
L538, L550, X100, X150, X152, X200, X202, X260, X300, X330, X350, X351, X400,
X760, X761 and XJS.

## Fourth slice: on-demand self tests

`OdstInfoAdapter` parses the `rds-odst-info-<MODULE>.xml` documents. This slice
is different from the first three and the difference is the point of it.

**An on-demand self test is not a read.** It commands an ECU to run a routine,
which makes it stage-2 material under `docs/ROADMAP.md` and outside the current
scope. `ADR-0009` permits such material to be recorded as known-but-unavailable
and never as an available operation, so every record this adapter produces
carries `DiagnosticSafetyClass::ServiceRoutine`. A golden test asserts that no
record is ever `ReadOnly`, and the real-source example asserts the same over the
whole corpus.

Nothing can execute these. The application execution surface accepts only typed
read-only intent and the architecture checker rejects adding another path.
Knowing which self tests a module declares is useful for planning stage 2; it
does not bring stage 2 forward.

Each test becomes a `DiagnosticCapability` named from its help-screen data name,
falling back to `ODST test <id>` when the document does not name it. Qualifiers
narrow applicability: `model` to `vehicle_program`, `moduleType` to
`ecu_family`, `TYPE` and `SUBTYPE` to `powertrain` and `variant` — the same
dimensions the DID catalogue uses for the same concepts. Vendor qualifiers such
as `CM_QUAL_ENG_TYPE` are recorded verbatim under `sdd_qual_*` rather than being
mapped onto a dimension whose meaning the source never states. Model years stay
designations, consistently with the DTC slice.

### Real-source run

| | |
| --- | --- |
| files scanned | 93 |
| files rejected | 0 |
| knowledge records | 1,153 |
| distinct self tests | 123 |
| distinct modules | 93 |
| distinct vehicle programs | 23 |
| classified `ServiceRoutine` | 1,153 |
| classified `ReadOnly` | **0** |

## Fifth slice: the DTC index files

Three index files complete the surveyed corpus. `DtcDescriptionAdapter` serves
both `dtcDescriptions.xml` and `dtcModuleDescriptions.xml`, which share a root
and differ only in whether an entry names a module. `DtcFaultTypeAdapter` reads
`dtcFaultTypes.xml`, the failure type byte catalogue.

These indexes are **deliberately weaker** than the per-code help documents: they
carry no model or model-year qualification. An entry without a module stays
unscoped rather than claiming to hold for every module, and where an index and a
help document disagree the store reports it instead of preferring one.

A failure type byte is DTC vocabulary rather than a separate domain object, so
entries share `EntityKind::DiagnosticTroubleCode` and are distinguished by an
`FTB-` identifier prefix and the `sdd_failure_type` claim key. SDD numbers the
SAE reserved range `-1`; a bare minus sign would read as a separator inside an
identifier, so negatives are spelled out as `FTB-neg1`.

### One bug the real data exposed

The first implementation keyed records by code and module alone. That silently
dropped 393 entries, because **209 code/module pairs carry more than one
wording** — `B1250` on `CCM` is both `In car temperature sensor circuit failure`
and `In car sensor open circuit or short circuit high fault`.

Occurrences now get distinct record ids under the same entity and claim key, so
the disagreement is reported as a conflict, while an exact repeat is still
dropped as carrying nothing new. This is the same rule the addressing slice
already used for reused mnemonics; applying it here was an oversight the real
corpus caught.

### Real-source run

| | |
| --- | --- |
| `dtcDescriptions.xml` | 4,031 records |
| `dtcModuleDescriptions.xml` | 1,758 records |
| `dtcFaultTypes.xml` | 257 records |
| distinct fault codes | 4,612 |
| distinct modules | 121 |
| conflicts reported | 535 |

## Model-year resolution, opt-in

`ADR-0011` adds a `ModelYearTimeline`. Supplying one to an adapter additionally
expresses an SDD marker as a `YearConstraint::Range`; the verbatim marker is
always retained, so the derivation is reversible and a later correction
re-derives rather than re-ingests. Without a timeline nothing changes.

Timelines are built by observing the corpus, never hardcoded, so vehicle
programs stay open data. Two-digit years resolve by **validation, not
assumption**: SDD's window is closed at 1994-2017, so `MY94` reads as 1994,
`MY17` as 2017, and anything outside is rejected. Without that check a 1990s
Jaguar would land in the 2090s.

A breakpoint runs from its own year until the next; the last stays open above.
`BASE` is bounded above by the first breakpoint and open below, because no
launch year is stated. Markers inside one calendar year coarsen to the same
range, which is correct at year granularity — the marker remains the finer key.

`Pre MY10` and `Post MY10` carry their own boundary, so they resolve without a
program sequence: `Pre` runs to 2009 and `Post` from 2010, partitioning the
range with no gap and no overlap. That partition is not cosmetic. L319, L320 and
L322 use exactly these markers to separate **29-bit from 11-bit CAN addressing**,
so a boundary placed one year out would mean the wrong addressing width on a
real vehicle. The same inclusivity argument applies: were `Post MY10` to begin in
2011, a 2010 Discovery would declare no addressing width at all.

### One correction the derivation exposed

Qualified records could not resolve at all, and the reason was not the years.
SDD qualification is a conjunction of equality tests, so a dimension the
expression does not test is **unconstrained**, not undetermined. The adapters
had been leaving those dimensions `Unknown`, which meant every qualified record
resolved as `InsufficientEvidence` no matter what a caller supplied. Untested
dimensions are now `Any`.

The distinction is deliberate: an omission inside a qualification expression is
a positive statement, whereas the DTC index files carry no qualification
expression at all, so their unstated dimensions remain genuinely unknown.

### End to end

A two-pass run over the real DID catalogue derives sequences for all 21
programs, X250 reading 2008, 2010, 2012, 2013 against an XF launched in 2008 and
facelifted in 2012. Asking the store for **X250, model year 2010, PCM, V6
diesel** now returns 8 applicable parameter definitions with names, units, and
byte layouts, each traceable to the SDD source.

That is the first question about a real vehicle the knowledge base answers end
to end.

## Sixth slice: platform addressing

The platform documents of the corpus became available on 2026-09-02; the
provenance record notes what they delivered and a prediction this document had
made and got wrong.

`PlatformAdapter` reads `PLATFORM_<PROGRAM>_<YEAR>.xml`, which carries what the
rest of the corpus never states: the vehicle's network architecture and the
diagnostic CAN identifiers for each module.

Programme and model-year marker are taken from the document's own `<qualifier>`
elements, never from the file name, and a document naming more than one of
either is rejected rather than resolved arbitrarily. The adapter is generic
across the fleet; X250 is only the verification case.

### Programming addressing is not ingested

Every module declares addresses for a `diag` and a `prog` session.
Programming-session addressing is a route into software download, outside every
current stage, so it is **not ingested at all** rather than recorded and trusted
to be ignored. A route present in the knowledge base is a route something can
later be built on. A golden test asserts none of the fixture's programming
identifiers appears anywhere in the store.

Connector pins are likewise never fabricated: the platform states a rate and an
identifier width but no J1962 pins, so `pins` stays empty.

### Real-source run

| | |
| --- | --- |
| platform files | 110 |
| ingested | 110 |
| rejected | 0 |
| module addressing claims | **1,418** |
| distinct modules | 89 |
| vehicle programs | 22 |

Buses are not only CAN. `KW2000`, `KW2000STAR`, `DS2`, `SCP`, `D2B` and `ISO`
all appear, confirming what `docs/ARCHITECTURE.md` already assumed: the SDD era
is not UDS-only, and legacy protocol support must not be forced through UDS.

### Third independent confirmation of the X250 route

`PLATFORM_X250_201000.xml` declares `PCM` at `can_tx 0x7E0`, `can_rx 0x7E8`, on
`CAN_HS` at 500 kbit/s with 11-bit identifiers.

F6 evidence derived that route from wiring documentation, and F8 built its
prepared transaction on it, both before this file was readable. Three unrelated
sources now agree, which is what the F5 evidence model exists to make visible.

## Acceptance

- 46 F9 golden tests over synthetic fixtures, covering parsing, fail-closed
  applicability, classification, conflict reporting, determinism, idempotency,
  converter resolution, bit-packed parameter separation, per-qualifier scoping,
  safety classification of non-read material, negative and CDATA edge cases, and
  rejection of unsupported qualification, non-read access, identifier mismatch,
  a wrong document root, and malformed input without partial write.
- Workspace: 137 tests across 41 suites, clippy clean, formatted, architecture
  boundaries pass with new `sdd-ingest` manifest and source guards.
- Real-source ingestion executed through the two examples, which report what
  they would store and write nothing:
  `ingest_can_link_monitor`, `ingest_did_catalogue`, `ingest_dtc_help`,
  `ingest_odst`, `ingest_dtc_index`, `resolve_model_years`, and `ingest_platform`.

Rust tests were run under Linux in a container because Smart App Control blocks
freshly linked unsigned binaries on the Windows host with `0xc0e90002`. See
`docs/F3_1_WINDOWS_SIGNING.md`.

## Not yet done

Every file identified in the survey is now ingested. Further breadth would come
from other SDD components not yet surveyed, or from tester captures.

Model-year resolution is implemented but rests on a corroborated rather than a
documented reading; a JLR statement would raise its standing.

## Hard limits

`.exml` decryption is out of scope. Firmware is never ingested: components whose
names contain `FLASH` and every `.vbf` payload are excluded from extraction and
from the knowledge base, per `ADR-0005`. Extracted SDD trees stay outside the
repository; only derived records, provenance, and synthetic fixtures are
committed.

## Exporting the corpus for the application (2026-09-03)

`ADR-0014` makes F5 manifests the application's runtime input. The example
`export_manifests` walks one or more locally extracted SDD roots, ingests
everything the adapters above understand into one store — so the set is
validated exactly as the application will validate it — and writes one JSON
bundle per adapter kind. Model-year ranges are derived with the same two-pass
timeline as `resolve_model_years`, observing both platform and DID markers.

```bash
cargo run --release -p sdd-ingest --example export_manifests -- <out dir> <root>...
```

Roots are the SDD component directories, so that source locators keep the
`COMPONENT/…` form the examples above record. The first real run took 30
seconds: 45 platform, 6 DID catalogue, 3 DTC index, 6,168 DTC help, 93 ODST
and 1 link-monitor documents → 6,316 manifests, 126,706 records, 0 rejected.
The output is SDD-derived and stays outside the repository; see `G4` in
`ROADMAP.md`.

The first export also corrected the platform adapter: 744 module addresses in
the corpus are physical node addresses (`type="phys"`) under `normal_fixed` or
`enhanced` schemes, not CAN identifiers, and had produced no record. They are
now recorded verbatim as `sdd_physical_address` claims with the module's bus,
so the module is seen; identifiers are not derived (see the F10 document).

## VIN decode tables and module names (F11, 2026-09-04)

Two further adapters read parts of the payload outside the XCL data:

- `VinDecodeAdapter` reads `CURRENT_JLR_VIN_DECODE_XML/Xml/VINDecode.xml`
  into text claims on a new entity kind, `VinDecodeModel` (`VIN-MODEL-<n>`):
  `sdd_vin_rule` for each rule block (`1..3=SAJ;12..12!=B`) and
  `sdd_vin_attribute.<Name>` for each attribute (`const=Jaguar`,
  `chars=6..7;01=X200|…`, decoded text escaped). The chart issue a model was
  taken from travels on the evidence as a note. Nothing is decoded at ingest.
- `ModuleTextAdapter` reads three item families of SDD's text database
  (`CURRENT_PAG_MCP_TEXT_XML/Xml/Text/@J/`): `@J_14229_M_DESC_*` and
  `@J_M_DESC_*` into `sdd_module_name.<lang>` and
  `sdd_module_name_legacy.<lang>` claims on the ECU family, and
  `@J_I_ISO15031_FAULT_TYPE_<hex>` into `sdd_failure_type.<lang>` claims on
  the `FTB-<decimal>` entity `dtcFaultTypes.xml` also describes (the text
  database numbers the byte in hexadecimal; the ingest converts). One claim
  per language the item carries (twelve; no Ukrainian). SDD tags the language
  as `xmlns:lang`, which the adapter reads as the plain attribute it is.

The exporter classifies both by file name and writes `vin_decode.json` and
`sdd_text.json`. See `F11_TESTER_APPLICATION.md`, slices 7–9.


## 2026-09-07 — ten legacy Jaguar platform documents were never walked

Found while asking why a survey of X350 MY2006 returns no modules at all.

The exporter walks the roots it is given. The recorded command passes
`CURRENT_JLR_XCL_XML_DATA_XML` for the platform documents, and that
component holds 45 of them, all ingested: 18 programmes, 45 programme-years,
from L316 MY07 to X761 MY17. That part is complete.

The older Jaguars are not there. Their platform documents sit in a different
component, `CURRENT_JLR_MCP_XML_XML`, which no export has ever walked:

| Document | modules | programme, marker | networks |
| --- | --- | --- | --- |
| `PLATFORM_X101.xml` | 17 | X100 MY01 | `CAN_HS_NVJCOM`, `SCP`, `ISO` |
| `PLATFORM_X103.xml` | 20 | X100 MY03 | `CAN_HS_NVJCOM`, `SCP`, `ISO` |
| `PLATFORM_X202.xml` | 1 | X202 MY02.5 | `CAN_HS_NVJCOM` |
| `PLATFORM_X204.xml` | 24 | X202 MY04 | + `CAN_HS`, `D2B` |
| `PLATFORM_X206.xml` | 24 | X202 MY06 | + `CAN_HS`, `D2B` |
| `PLATFORM_X350.xml` | 26 | X350 MY02_5 | `CAN_HS_NVJCOM`, `SCP`, `ISO`, `D2B` |
| `PLATFORM_X356.xml` | 28 | X350 MY06 | + `CAN_HS` |
| `PLATFORM_X358.xml` | 30 | X350 MY08 | + `CAN_HS` |
| `PLATFORM_X400.xml` | 16 | X400 MY04 | `CAN_HS_NVJCOM`, `SCP`, `ISO`, `D2B` |
| `PLATFORM_X404.xml` | 30 | X400 MY04 | + `CAN_HS_ISO14229` |

216 modules over the S-Type, XK8, XJ of 2003-2009 and X-Type. The schema is
the one `PlatformAdapter` already parses: `module_fitment` entries with
`module_code_name`, `address type="can_tx"` and `"can_rx"`, `network`,
`data_identifier_set`, `fitment` and a text-database `tm` reference; the
programme and marker come from the document's own `qualifier`, not its file
name, and each of the ten declares exactly one of each after de-duplication.
So they would parse as they stand.

### One thing to settle, and one that settled itself

- **`PLATFORM_X400.xml` and `PLATFORM_X404.xml` both declare X400 MY04**,
  with 16 and 30 module entries. This looked like a collision and is not
  one. `X404` is the fuller revision of the same programme-year: its
  acronyms are a strict superset, adding `DSM`, `GWM` and `TPM`, and its
  repeated `ABS`, `ECM` and `TCM` entries are the variant qualifiers the
  schema's own comment describes, petrol against diesel and high against
  low line. The two documents merge on module family into the union of 19,
  which is what the survey shows. Their source ids differ by file stem, so
  no duplicate source arises.
- **Ingesting them adds knowledge, not reach.** These cars are pre-UDS:
  `CAN_HS_NVJCOM` runs JAGCAN, `SCP` is J1850 PWM at 41.6 kbit/s, `ISO` is
  K-line at 10.4 kbit/s, `D2B` is optical. The adapter has the hardware for
  three of them, which the 2026-09-06 resource sweep confirmed: resource 2
  is J1850 PWM on pins 2/10 and resources 3 and 4 are K-line on pins 3, 7 or
  8. What is missing is the protocol stacks and the bus bindings; none of
  those buses is bound, and `uds-execution` does not speak NVJCOM. Only the
  handful of modules these documents place on `CAN_HS` — 3 on X204, 5 on
  X206, 5 on X356, 6 on X358 — and the 4 on `CAN_HS_ISO14229` of X404 could
  ever use today's read path, and only once that bus is bound.

The honest order is therefore: ingest for the survey and the fault-code
wording first, so a person with an XJ of that era at least sees what the car
carries and why it cannot be read yet; the legacy protocols are their own
phase with their own ADR.

### Ingested the same day

The exporter now walks `CURRENT_JLR_MCP_XML_XML` beside the XCL tree. One
code change was needed: SDD writes the mid-year fraction with an underscore
in the XCL documents and with a dot in the legacy ones, so `parse_marker`
accepts `MY02.5` as it already accepted `MY02_5`, with a golden test that
the two mean the same point. Nothing else moved.

| | before | after |
| --- | --- | --- |
| platform manifests | 45 | 55 |
| platform records | 9,861 | 11,564 |
| programmes | 18 | 22 |
| sources | 6,505 | 6,515 |
| records | 129,441 | 130,388 |

Rejected: none. The library loads with no failed manifest.

What the four new programmes give, surveyed offline:

| Vehicle | modules known | reachable today |
| --- | --- | --- |
| X350 MY02_5 | 26 | 0 |
| X350 MY06 | 28 | 5 |
| X350 MY08 | 30 | 6 |
| X400 MY04 | 19 | 0 |
| X202 MY02.5, MY04, MY06 | 1, 24, 24 | 0 |
| X100 MY01, MY03 | 17, 20 | 0 |

The XJ of 2006 and 2008 is the one that gains something usable now: `ABS`
`0x760/0x768`, `AHCM` `0x7B3/0x7BB`, `PBM` `0x7B4/0x7BC`, `PCM`
`0x7E0/0x7E8`, `TPM` `0x7B5/0x7BD`, and on MY08 also `DCSM` `0x776/0x77E`,
all on `CAN_HS` over `hs-can`. Everything else on these cars sits on
`CAN_HS_NVJCOM`, `SCP`, `ISO` or `D2B` and says so with its reason.

One cosmetic consequence: the vehicle picker now offers `X202` with marker
`MY02.5` and `X350` with `MY02_5`, two spellings of the same point, because
that is how SDD writes them. The marker is left as the source wrote it
rather than normalised, since it is also a join key against the other
slices.

Worth knowing beside this: the vehicle pictures added on 2026-09-06 cover
X-Type, XK8 and S-Type, so those cars can now be chosen, drawn, surveyed,
and truthfully told which of their modules cannot be reached and why.

## 2026-09-09 — read a second time, independently

The corpus was read again by `scripts/xml-crosscheck/crosscheck.py`, a
separate program in another language that shares no code with this crate, and
its reading compared with the exported library. The method, the counts and
the findings are in `docs/evidence/xml-crosscheck-2026-09-09.md`. Two things
belong here, in the ingest's own document:

- **Field widths are confirmed.** Every one of the 3,799 byte ranges and
  masks the library carries is one a DID-formatting document declares. The
  parser does not misread a width.
- **Qualified addresses collapse.** A platform document may declare the same
  module acronym several times under a `<qualifier>` naming an engine type, a
  build year or a market, each with its own diagnostic address. The record id
  this crate writes is `sdd-169-platform-<document>.module.<FAMILY>.addressing`,
  which carries no qualifier, and the qualifier does not reach
  `applicability` either; so the rows collide and only the last address in
  the document survives. Nine module rows in the corpus are affected,
  including the X250 of 2010, whose instrument cluster and parking-brake
  module answer at different addresses on the V6 and 4.2 V8 than on the 5.0
  and the diesel.

  **Fixed the same day, at the owner's decision.** `module_quals` reads the
  module's own `<qualifier>`; `qualified_by_module` narrows the applicability
  with it — `type` to the powertrain, `SubType` to the variant, a test naming
  a market to the market, everything else to `other` under SDD's own name —
  and `qual_suffix` adds a short digest of those tests to every record the
  module produces, so two rows stop colliding. A module with no test of its
  own keeps the id it always had, which is most of them. The digest is used
  rather than the words because a stable id may be 128 characters and an
  evidence id repeats the source id inside itself; the words stay in the
  evidence note. Re-exported: 131,501 records against 130,388, nothing
  rejected, and the cross-check then agreed on every one of 742 module rows
  in both directions.

  Two consequences worth stating. A module whose address depends on the
  engine is now unresolved for a car that does not state one, rather than
  answered with one engine's address; that is the intended direction, and the
  survey shows the reason. And SDD splits the naturally aspirated V8 of the
  X250 by displacement, which lands in `variant`; the catalogue now carries
  the variants a programme declares, beside its engines, and the interface
  offers them, so that car can say which half it is.

## 2026-09-11 — how much text the help layer actually holds

Measured when the owner asked whether the layer could be said in Ukrainian, a
language SDD does not publish. The earlier estimate of "months of prose" was
wrong, and wrong in a way worth recording: the unit of work is not a line on
a screen.

A help screen is a list of `helpScreenItem` references; the text itself lives
once in a `helpScreenMnemonicList` and is reused. So the corpus is:

| | |
|---|---:|
| Named texts with content | 26,893 |
| Distinct texts among them | 25,393 (2.21 MB) |
| Screen slots they fill | 193,520 |
| Mean reuse per named text | 7.2 |
| Texts of 60 characters or fewer | 41 % |
| …of 120 or fewer | 78 % |
| Longer than 250 characters | 3.6 % |

They are short lines, not paragraphs: `Wiring harness fault`, `Clear the DTCs
and road test the vehicle`. By kind, `H_CAUSE` holds 7,338, `H_ACTION` 4,755,
`J_I_ACTION` 2,444, and the manufacturer-prefixed families another 4,000 —
causes and actions are two thirds of everything.

**The reuse is what matters for planning.** Ordered by how many slots each
text fills:

| texts | slots covered | share |
|---:|---:|---:|
| 10 | 59,080 | 30.5 % |
| 250 | 96,292 | 49.8 % |
| 1,000 | 118,329 | 61.1 % |
| 4,000 | 148,181 | 76.6 % |
| 26,893 | 193,520 | 100 % |

Ten texts cover a third of everything a reader sees, because `Possible
causes` is placed 23,166 times and `Actions required:` 22,951. The layer can
therefore be translated in slices, the first slices worth far more than their
share of the rows, and a text with no translation falls back to English one
line at a time, as `ADR-0025` already has it.

Separately, the code's own description is a much smaller layer: 1,049 distinct
descriptions, of which **261 are a raw mnemonic key in all twelve languages** —
SDD ships no text for them — leaving 788 real ones at a mean of 38 characters.

**Not decided here.** Whether this layer is translated at all, by whom, and
whether the result can live in this repository or must travel in the issued
library, is open. The measurement exists so that question is argued over
numbers.

## 2026-09-11 — the corpus is published in twelve languages

Read while the owner was translating the parameter names by hand. Every
text-bearing SDD data family ships once per language, not once in English:
`SNAPSHOT`, `DTC_HELP`, `ODST`, `RULES`, `QUAL`, `APP_HELP`, `PDF` and
`SYMPTOM_HELP`, each in `DE EN ES FR IT JA KO NL PT PT_BR RU ZH`. There is no
Ukrainian, which is the one thing the earlier reading had right.

**Measured, by the documents' own keys.** The English and Russian snapshot
components were walked and every parameter keyed by its `KeyedData id` plus
its `ReadParameter id` — identity, not text similarity:

| | |
|---|---:|
| Parameters in the English component | 15,269 |
| Parameters in the Russian component | 15,269 |
| Keys present in both | 15,269 (100 %) |
| Text left in English | **0** |
| Distinct English names | 4,976 |
| …with more than one Russian rendering | **0** |
| A whole language as a name dictionary | 0.99 MB, **0.10 MB packed** |
| The same keyed per identifier instead | 2.35 MB, 0.13 MB packed |

The fault-code family is the same: the Russian `DTC_HELP` pack holds 3,452
files against English's 3,452, with identical tag counts and identical
`dtcDescriptionId` values, and only the human text changed — `Coolant
temperature fault` reads `Неисправность, связанная с температурой
охлаждающей жидкости`.

Two consequences. A second language costs a tenth of a megabyte against a
12 MB issued library, so the choice between translations is about wording and
voice and never about size. And because no English name has two Russian
renderings, a dictionary keyed by the English name loses nothing — which is
the shape the owner's own translation file already has.

**A correction.** `docs/F11_TESTER_APPLICATION.md` stated that fault-code
descriptions exist in English only, "the other pack is Chinese". That was
wrong, and it was this assistant's sentence. Twelve packs exist and the
Russian one is on the owner's disk, extracted, 83 MB. The decision the line
sat next to — Russian a full interface, Ukrainian the deliberate choice — is
the owner's from 2026-09-04 and is untouched.

**Not ingested, and deliberately so for now.** `COMMON_SDD_DATA_RULES` is
78 MB over 56 files, one `isip` document per programme and model year,
holding SDD's symptom-driven tree: system, group, section, then named
pinpoint procedures — `Anti-lock braking system valve supply  -  Power feed
test`. It is the path a technician takes when there is no code, and this
product has no equivalent layer. It needs its own ADR and its own decision
about whether it belongs in stage 1 at all. Recorded here so it is not
rediscovered a third time.

## 2026-09-10 — the help layer, read at last

The per-code DTC help documents were being read for their descriptions and
nothing else. They carry more: a screen of text per code, chosen by model,
model year and fault type, listing possible causes, actions required and,
for a quarter of the corpus, the conditions under which the module sets the
code. The owner asked for it after saying plainly how diagnosis actually
begins — with a code, not with a panel of numbers.

**The survey, before any code was written.** Of the 6,168 files in
`COMMON_SDD_DATA_DTC_HELP_LANG_EN/rds-dtc-help`, every one parses:

| | |
|---|---:|
| With real help text | 4,206 (68 %) |
| With only the `J_H_NO_HELP_AVAIL` placeholder | 1,962 (32 %) |
| Distinct mnemonics in the string tables | 42,637 |
| …carrying more than one text under one name | **0** |
| Text of the whole dictionary, one text per name | 3.7 MB |
| Qualifiers on description selections | 214,522 |
| …resolving to a screen that says something | 98,949 |
| Distinct screens actually selected | 37,291 |
| Codes naming a datalogger identifier in the text | 223 (157 identifiers, 121 of them in our own catalogue) |
| Codes carrying a number with a unit or a comparison | 1,613 (26 %) |

**The encoding fault did not exist.** Reported earlier in the day as
`-40�C` in the source; the files are valid UTF-8 to the byte, with zero
U+FFFD anywhere, and thirteen non-ASCII characters in all — the degree sign
(882), the en dash (99), `®`, `±`, `²`, the micro sign, bullets, curly
quotes, and 58 no-break spaces. The mojibake was the terminal that printed
it. Nothing needed fixing; what needed doing was reading the bytes as UTF-8,
which the parser already does.

**What is recorded.** The chain as SDD writes it, not the resolved product:

- `sdd_help` — which screen a car is given, qualified by module, model,
  model-year designation and, on its own dimension `sdd_fault_type`, the
  fault type. About 74,500 records, each valued with a screen's name.
- `sdd_help_screen.<name>` — what that screen says, its mnemonics resolved
  in order and joined by newlines, unqualified because the text is the text.
  About 37,300 records.

Resolving at ingest instead would write the same screen out once per car
that selects it: 44.3 MB of duplicated text against about 15 MB, for no
gain. A screen with no items — SDD has many, and 103,008 qualifiers point at
one — produces no record at all rather than an empty claim.

**Packed on the way out.** The export writes `<name>.json.gz` (`ADR-0023`).
The same 242,691 records that fill 311 MB as plain JSON take 11 MB packed,
and `dtc_help.json` alone drops from 275.7 MB to 9.97 MB. The application
reads both forms, and the issue stamp hashes the JSON rather than the bytes
on disk, so packing changes nothing about what a copy is or how it is
verified.

**What is not done.** The identifiers named in the action text are left in
the prose; joining them to the catalogue so a code can offer «watch this
signal» is a separate change. The thresholds are left in the prose too, and
deliberately: «below 65 Volts» carries conditions and delays that a `min`
and a `max` would drop, and a machine threshold read out of a sentence is
exactly the kind of invention this project does not make. The sentence is
shown; the reader does the reading.

## 2026-09-12 — the identification identifiers (ADR-0027)

The platform documents carry one more thing the first slices left where it
lay: per module, the names of three identifier sets — `NET` (the module's
own, `ggds_<module>`), `SWDL` (software part numbers, `ggds_swdl_N`) and
`PDI` (the pre-delivery list, `ggds_pdi_1`) — and, at the document root, the
sets themselves, each a list of `<did>` with the identifier, SDD's attribute
name, the service and, usually, a human text. Over the 55 documents: 3,168
set definitions, 549 distinct identifiers, 5,671 references from modules to
sets. Of 33,000 `<did>` declarations 32,992 name service `0x22`.

`PlatformAdapter::add_identification` records, per module, the members of
its `SWDL` and `PDI` sets and the `0xF100`–`0xF1FF` members of its `NET`
set, service `0x22` only, as `IdentifierParameter` records in the DID
catalogue's own shape — `ClaimKey::ParameterDefinition` under SDD's text
for the name, `IdentifierDefinition` with the encoding `text=ascii` — so the
resolver's readable list and the transaction gate admit them exactly as they
admit a catalogue parameter. The row is narrowed by the module's own
qualifier and by the set's: 348 `SWDL` references carry a qualifier of
their own, a DAB module having one software list for standard hardware and
another for DAB+. A set the document references but never defines yields
nothing; a member read with any other service is left out; the `NET` set's
other members — the distance `0xDD01`, the legislated mirrors, a module's
own data — are seen and not ingested, because they come without a byte
layout and would widen the module-read list with rows the decoder can only
show as bytes.

### Real-source run

| | |
| --- | --- |
| platform documents | 55 |
| identification records | **34,788** |
| distinct identifiers | 45 |
| module families | 99 |
| vehicle programmes | 21 |
| most declared | `0xF188` (2,142), `0xF124` (1,918), `0xF18C` (1,890), `0xF191` (1,890) |
| rejected | 0 |

The library exported afresh holds 314,462 records against 279,674 the day
before; `platform.json.gz` alone goes from 11,564 to 47,465. Golden test:
`identification_identifiers_are_recorded_per_module_in_the_catalogue_shape`,
over a fixture whose `NET` set mixes identification with a distance, a
legislated mirror and a member read with `0x09`, whose two `SWDL` sets are
chosen by a qualifier, and which references one set it never defines.

## 2026-09-12 — the car configuration file (ADR-0028)

`CCF_DATA_<PROGRAM>[_<YEAR>].xml` describes one car's configuration: a
header naming the module that keeps the master copy (`<sync>`) and the
modules holding copies (`<source>`); blocks, each with `serviceIdRd`,
`serviceIdWr` and, per module, an `<address>` naming the data identifier,
the offset of the block inside that identifier's payload and its byte range;
and a layout of groups and parameters — `id="SSS_EEE_bbb_BBB"` the start
byte, the stop byte, the first and the last bit — with a mask, a type
(`ENUM`, `BOOL`, `BIN`, `ASCII`, `BCD`, `UNDEF`), a `<category>` saying
whether SDD's editor displays and edits it, and options with a value, a name,
sometimes a sales `code`, and a text id. The programme and marker come from
the document's own qualifiers, never the file name.

`CcfAdapter` records three things. The scheme (`sdd_ccf_scheme`: `did`, or
`vdf` for a document that pages its `CCF` block through VDF blocks) and the
sources (`sdd_ccf_source.<MODULE>`: `sync` or `copy`) as text claims on the
programme. For a `did` document, every `<address>` of a block read with
`0x22` whose module is a module — `AS_BUILT`, `AS_IS` and `OTHER` are the
factory file and placeholders — as an `IdentifierParameter` in the DID
catalogue's shape, the parameter named `Car configuration block <BLOCK>`,
the encoding `ccf=<BLOCK>;offset=<n>;length=<n>`, applicable to the
programme, the marker, the address's engine qualifier and the module, so
the resolver's readable list and the transaction gate admit the read as
they admit a catalogue parameter. And every parameter as a
`ConfigurationParameter` record — a new entity kind, `ADR-0010`'s
precedent — valued as one escaped `key=value;…` text: block, byte and bit
span, mask, type, the display, edit and scope flags, group and titles in
English and Russian, and the options as `value=name=code=en=ru|…`. The
texts come from a `TextLookup` the exporter fills from every item of SDD's
text database; an id SDD has no text for stays empty and the name speaks.
`serviceIdWr`, `t_ccf` and `upload="mem"` sources are not recorded.

### Two things only the real corpus revealed

- **A document may stamp a few addresses with a neighbouring year.** The
  X250 document of 2013 names `MY13` sixteen times and `MY12` four — its
  reserved block's addresses. The platform rule (exactly one marker, or
  reject) would have lost the whole car. The document's marker is now the
  one most qualifiers name, a tie is refused, and an address whose own
  qualifier names another year is narrowed to that year: the source's
  statement, kept.
- **A span may be written backwards.** The X351 document of 2016 gives one
  parameter the id `460_406_006_007`, the stop byte before the start. A span
  that cannot be read is a parameter this product cannot place; it is left
  unrecorded and the document's other 1,263 parameters stand, rather than
  the document being refused whole.

### Real-source run

| | |
| --- | --- |
| documents | 48 (46 under `CURRENT_JLR_XCL_XML_DATA_XML`, 2 legacy under `CURRENT_JLR_MCP_XML_XML`) |
| programmes | 20 |
| rejected | 0 |
| scheme `did` / `vdf` | 30 / 18 |
| block identifiers | 335 over `BCM`, `FSJB`, `GWM`, `IPC`, `PCM`, `RSJB` |
| …by identifier | `0xF105` 154, `0xF106` 85, `0xDE00` 64, `0xC25F` 12, `0xD900` 12, `0xF114` 4, `0xE736`/`0xE737` 2 each |
| parameters | **38,505**, of which SDD's editor displays 2,871 |
| …with a group title text | 24,450 |
| options | 141,125 |
| text items read for the titles | 6,981 |
| `ccf.json.gz` | 6.8 MB; the library 353,450 records, 21 MB on disk |

Golden tests: `f9_ccf_golden.rs`, five of them, over a fixture that mixes
a readable block with padding, two blocks in one identifier at two offsets,
a copy narrowed by an engine, an address stamped with a neighbouring year, a
span written backwards, texts carrying every separator, and a paged
document that yields no identifier.

## 2026-09-12 — the K-line buses (ADR-0029)

Six of the platform documents' networks are not CAN: `<network>` elements
whose `<iso>` child states the physical layer — `rate` in baud, `settings`
with `data_bits`, `parity` and `stop_bits`, a `<wakeup type="…">`
(`bmw_ds2`, `bmw_kw2000_star`, `kw2000_fast`, `rosco`) and, for four of the
six, a `<connection>` naming a connector pin (`K_LINE_PIN_7`, `K_LINE_PIN_8`,
`ISO_K_KW2000`, `ISO_K_ROSCO`) that the document's own `<connector>` maps to
a J1962 pin number. Their protocols are `DS2`, `KW2000`, `KW2000STAR` and
`ROSCO`; a module on one of them has a one-byte `<address type="phys">` and
no CAN identifier.

`PlatformAdapter` records, per such bus, a `NetworkRoute` on the bus's own
name — connector and pin where stated, the rate as the bit rate — and two
text claims, `sdd_iso_settings` (`data_bits=8;parity=even;stop_bits=1`) and
`sdd_iso_wakeup`; per module, a `DiagnosticAddressing` with the node address
in both identifier fields, no CAN identifier format and the addressing mode
`iso9141_node` (a word the `knowledge` crate owns, because the resolver reads
it), beside the `sdd_physical_address` text every physically addressed
module already had; and read-only capabilities by protocol — `DS2`:
`ds2.ecu_identification.read_only` and `ds2.fault_memory.read_only`;
`KW2000`: `kwp2000.service1a.read_ecu_identification.read_only` and
`kwp2000.service18.read_dtc_by_status.read_only`; `KW2000STAR` and `ROSCO`:
none, so the survey says the protocol is named and not spoken rather than
trying the wrong one. Nothing about a bus is assumed: a bus that states no
pin gets no pin and no connector, and the adapter route is not the
document's to state — it is the hypothesis manifest's
(`mongoose_jlr_kline_route_hypotheses.json`, `unverified_research`).

### Real-source run

| | |
| --- | --- |
| platform documents with a K-line bus | 14 |
| K-line bus records (`.iso.<bus>.route`) | 18 |
| …by protocol | `DS2` 3, `ISO` 9, `KW2000` 1, `KW2000STAR` 2, `ROSCO` 3 |
| …with a stated pin | 6 |
| node-addressed module records | 70 |
| …by protocol | `?` 8, `DS2` 15, `KW2000` 1, `NVJCOM` 43, `ROSCO` 3 |
| K-line capability records | 44 |
| rejected | 0 |
| the library | 353,626 records (from 353,450), 21 MB on disk |

The count is larger than the six buses the ADR set out from because the
older Jaguars — X100, X103, X204, X206, X350, X356, X358, X400, X404 —
describe their `ISO` bus in the same `<iso>` shape: 10400 baud, 8N1, no
wake-up, no pin, and 43 NVJCOM modules with node addresses on it. They are
recorded as the documents state them and nothing more: no hypothesis
manifest names a route for `ISO`, no capability is recorded for NVJCOM, and
the survey shows each such module with both reasons — the route unresolved,
the protocol named and not spoken — as `ADR-0029` leaves them under *What
this does not decide*. Eight node-addressed modules sit on a bus whose
document declares no protocol at all; they are shown with that reason.

Golden test: `k_line_buses_carry_pin_rate_framing_and_wakeup_and_their_modules_a_node_address`
in `f9_platform_golden.rs`, over the synthetic platform's two K-line buses —
one pinned and spoken (`DS2_PIN7`, `DS2MOD` at `0x72`), one unpinned and only
named (`KW2000STAR`, `STARMOD` at `0x12`).


## 2026-09-12 — the battery monitor (ADR-0030)

The platform documents carry more than addressing. Inside each module's own
`NET` set, under a comment that reads *Additional PIDS*, they name the
identifiers that module answers for the battery monitor — and they name them
per car, with the module, the programme and the breakpoint marker the set is
qualified by. A dedicated `type="BATT"` set exists as well, in 30 documents
for `BCM`, `GWM` and `RSJB`, and it holds exactly one identifier: the battery
current. The dataset had to be found where it lives, not where its name
suggested.

`PlatformAdapter` now records, per module, each member of any of its sets
that the battery rule recognises. The rule is `knowledge::battery`: a curated
table of identifiers with the role each plays, and a check that the name the
document gives the identifier **on that module** names the battery or the
charging system. Both tests must pass. The table alone would trust a number
that means something else elsewhere; the name alone admits `Turbocharger
valve offset values`, which contains "charge" and is not a battery parameter.

The bytes are a join. SDD's DID formatting document — the one the catalogue
is built from — describes the bytes of 18 of these identifiers, with
converters, scales, offsets and units, and names no module for any of them,
so those rows resolve as *insufficient evidence* and reach nothing on their
own. The platform document names the module and not the bytes. The exporter
collects the formatting document's battery rows in its first pass and hands
them to the platform adapter in its second, which writes one
`ParameterDefinition` per module and battery parameter carrying that encoding
and unit. Where the formatting document says nothing, the platform's own name
is recorded with no encoding and the value is shown as the bytes it is.

### Real-source run

| | |
| --- | --- |
| identifiers the rule admits | 59 |
| module-identifier pairs | 106 |
| programmes | 17 |
| modules that serve them | `GWM`, `BCM`, `RSJB`, `BECM`, `FSJB`, `IPC`, `PSCM` |
| described byte by byte by the formatting document | 18 |
| battery records written | 1,486 |
| the library | 355,112 records (from 353,626), 21M on disk |
| rejected | 0 |

What SDD does not hold, and this product therefore does not show: a state of
health, an internal resistance for the 12-volt battery, and any arithmetic
that would produce either. The nearest thing the corpus keeps is the
estimated cold cranking voltage at the present state of charge, beside the
amp-hour charge loss and the lowest calculated amp-hour value; leakage
resistance exists only for the traction battery of a hybrid.

Golden tests: `the_battery_monitors_identifiers_are_recorded_for_the_module_that_serves_them`
and `the_byte_description_joins_the_module_that_serves_the_parameter` in
`f9_platform_golden.rs`, over a fixture whose `NET` set carries a state of
charge, a quiescent current, a monitor-reset count and a turbocharger
parameter that must stay out, and whose module also declares the dedicated
`BATT` set.
