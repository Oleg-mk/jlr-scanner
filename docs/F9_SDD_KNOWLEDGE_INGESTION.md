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
