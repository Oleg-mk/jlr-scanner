# F11 — an application a tester can use without knowing SDD (milestone M1)

Status: `IMPLEMENTED / FIXTURE_TESTED` for slices 1–9; slices 1–3 and 6–8
are also `REAL_SOURCE_SURVEYED` against the exported SDD 169 library, offline.
Nothing in this phase touches a vehicle or requires the adapter to be plugged
in. What stays open is listed at the end.

M1, as `docs/ROADMAP.md` states it: a programme and model-year picker built from
the loaded library instead of free text; fault codes joined to the descriptions
F9 holds; identifier values decoded with units from the catalogue's encodings
and converters; one session flow with one report bundle; every unsupported
case shown with its reason; finished presentation.

## Slice 1 — the vehicle is chosen from the library

`KnowledgeLibrary` builds a `VehicleCatalogueSnapshot` in the same pass that
counts the store after every load (`build_indexes` in
`crates/diagnostic-session/src/lib.rs`). For each programme it lists:

- the SDD breakpoint markers the platform documents declare, from the
  `sdd_year_breakpoint` dimension on the ECU-family records, each with the
  model-year range the F9 timeline derived for it (`MY10 = 2010–2011` on
  X250);
- the engines the DID catalogue qualifies its parameters by, from the
  `powertrain` constraint on identifier records (`GTDI`, `V6DIESEL`, …).

The shell exposes it as `get_vehicle_catalogue`; the survey panel renders it as
three lists — programme, model years, engine — and falls back to the earlier
free-text fields only while the catalogue is empty (built-in data alone
describes no vehicle). Choosing a marker sets `model_year` to the range's
first year, so the resolver's year constraints are satisfied without the
tester knowing what a breakpoint is.

Against the exported library the catalogue holds all 18 programmes the 45
platform documents declare, 49 markers, and between 2 and 7 engines per
programme; `cargo run --release -p diagnostic-session --example catalogue --
<library dir>` prints it.

VIN decoding stays deferred; the picker is the tester's path.

## Slice 2 — fault codes carry SDD's wording

The F9 DTC index is keyed by J2012 code (3,456 `P/B/C/U` entries in
`dtcDescriptions.xml`, plus module-scoped wording in
`dtcModuleDescriptions.xml`) and by failure-type number (`dtcFaultTypes.xml`).
The same load pass indexes every `DiagnosticTroubleCode` entity: `DTC-<code>`
alias texts with the module they are scoped to when they are, and `FTB-<n>`
failure-type texts.

`KnowledgeLibrary::describe_dtc(code, failure_type_byte, module)` returns the
module-scoped wording when one exists, otherwise the programme-independent
one, and the failure-type wording; each is absent when the library has none.
The shell joins it into every `DtcSummary` after a live fault-code read, with
`description_scope` saying which entry answered. Sample joins on the real
library:

| Module | Code | Wording | Scope |
| --- | --- | --- | --- |
| PCM | P0301 | Cylinder 1 misfire detected | module |
| ABS | C0031 | Front left wheel speed sensor | generic |
| RCM | B0001 | Driver's frontal stage 1 - deployment control | module |

Failure type `00` reads "General failure information - no sub type
information". SDD's internal hexadecimal codes (`0x0000`-style, resolved
through `rdsDtcHelp` qualifiers) are ingested but not joined here: a live
read yields J2012 codes, and those join directly.

## Slice 3 — identifier values are decoded as the catalogue says

F9's converter catalogue previously retained only a converter's kind and
output unit. It now also carries the arithmetic verbatim — `multiplier`,
`offset` and `offsetFirst` for the 1,689 linear converters, the `(x, y)`
breakpoints for the 165 map converters — and the DID adapter appends it to
each parameter's encoding descriptor:

```text
bytes=0..1;size=2;mask=0xffff;converter=CVT_N_RPM_OFF_0_RES_0PT25;scale=0.25;offset=0;offset_first=true
```

`diagnostic_session::decode::decode_parameters` applies exactly that to the
data a module returned: the byte range as a big-endian integer, the mask, then
`(raw + offset) × multiplier` when `offset_first` is set. Engine speed
`0xF40C` decodes to `0.25 rpm` per count, which is SAE J1979's scaling for the
same quantity — the one corroboration the linear path has. The shell decodes
every parameter of an identifier read into `ModuleReadSnapshot.parameters`
and the read panel shows name, value, unit and note.

**Map converters are recorded but not applied.** Their breakpoint values are
not consistently in the declared unit: the signed engine-speed map
`CVT_N_RPM_OFF_0_RES_0PT25_SGN` (unit `rpm`) lists `y = 8 191 750` for
`x = 32 767`, three orders of magnitude beyond any engine speed, and the
largest `|y|` across all maps is 1.6 × 10¹⁰. What SDD does with these numbers
before display is not established, so a map-converted parameter is shown as
raw counts, without a unit, with the note that its scale is not established.
Establishing it — from SDD's runtime or from a tester's read compared with a
known value — is a later slice; until then the unknown stays unknown.

Parameters whose bytes the response does not contain, or which have no byte
range or no scaling recorded, are shown with the reason instead of a value.
Bit-level packing beyond byte ranges is not interpreted.

## Slice 4 — one session, one report

The application is now driven as one ordered session rather than five
independent panels. The `Session` panel at the top lists the seven steps —
connect the adapter, load the data library, choose the vehicle, survey the
modules, listen to a bus (optional), read a module, save the session report —
and marks each `Done`, `Next` or `To do` from evidence the application already
holds: the adapter's verified board communication, the library's `LOADED`
state, a chosen programme and marker, an answered survey, and the counts the
shell keeps of what it recorded. `sessionSteps` in
`apps/scanner/frontend/src/sessionReport.ts` is the pure function behind it;
the first open required step is `Next`, an optional step never blocks.

The shell's `SessionReportService` keeps every capture, module read and
calibration read as the JSON its own service produced, and
`get_session_report_json` bundles them with the adapter, the library and the
last survey answered by the current library into one document
(`schema: jlr-scanner.session-report`, version 1). The bundle states about
itself that every item carries its own validation state and none becomes
vehicle-confirmed by being bundled. Loading a different library forgets the
previous survey, so a report never pairs a survey with a library that did not
answer it. The per-panel save buttons remain for single items; the tester
programme (F13) asks for the session file.

## Slice 5 — the vehicle network, as SDD draws it, explained

The owner asked for SDD's own screen as the starting point so the transition
is painless — not copied, taken as an idea and made better. SDD's "vehicle
technical characteristics" screen draws each bus as a line with the modules
hanging from it, a status icon per module, a legend, the vehicle at the left
and the actions along the bottom. Its weaknesses are the ones this project
exists to remove: mnemonics with no explanation, a status with no reason,
and no distinction between what is documented and what is guessed.

The cockpit (`apps/scanner/frontend/src/App.tsx`) keeps SDD's mental model —
vehicle card at the left, the network in the middle, the chosen module's
detail at the right — and changes what the picture says:

- **One lane per logical bus** (`CAN_HS`, `CAN_MS`, `PT_HSCAN`, `SUB_MOST`,
  …), labelled with the adapter route the data binds it to — `hs-can · pins
  6/14 · 500 kbit/s` — or with why there is none. A documented lane is a solid
  bar; a hypothesised route (ADR-0015) is a dashed amber bar; an unbound bus
  is grey with its reason on the label.
- **One node per module**, its state in words beside the icon and its reason
  or answer in a sentence on the node and in the detail panel: *Reachable*,
  *Unverified route*, *Not reachable*, *Not on this vehicle*, and after a
  read *Answered* or *N fault codes*, *No answer*, *Declined*, *Failed*.
  `nodeStatus` in `networkMap.ts` is the pure function behind it; silence on
  a hypothesised route is explained as not confirming the route.
- **A legend in sentences**, not icons alone.
- **Check all modules** — SDD's network integrity test, read-only: one
  confirmed-fault-code request to each checkable module in lane order, the
  map marking each answer as it arrives; *Stop after this module* ends the
  run; a checkbox includes or excludes the hypothesised routes. Every read is
  recorded into the session report by the shell as before.
- **Module detail** with the read actions (fault codes, one identifier) and
  the results — wording beside every code, decoded values with unit and, for
  named ranges, the state name — and the raw exchange behind an
  "Exchange as it happened" fold rather than in front of the tester.

The header carries the adapter and library pills, the seven session steps as
numbered chips and the single *Save session report*; the earlier panels —
adapter, data library, bus capture, F8 calibration read — remain below as
tools. The survey table and the standalone module-read panel are gone; the
map and the detail panel replace them.

## Slice 6 — enumeration states from the converters

969 SDD converters carry `quantityState` ranges — 4,438 in all, every one in
raw counts — naming what a value means ("Variant not programmed", "UK",
"Unknown/invalid" for 2–255). F9 now records them verbatim on the converter
and the DID adapter appends them to the encoding descriptor as
`states=low..high=name|…`, names escaped so they cannot break the
descriptor. The decoder names the range the raw counts fall in and the detail
panel shows the name first, the number beneath it. 7,098 of the 15,281
identifier parameters in the exported library carry states.

## Slice 7 — the VIN, decoded with SDD's own tables

`CURRENT_JLR_VIN_DECODE_XML/Xml/VINDecode.xml` is SDD's VIN decoding
document: 79 rule blocks that place a VIN in one of 33 decode models by
tests on character positions (`1,3 EQUAL SAJ`, `12 NOT_EQUAL B`, …), and per
model the attributes SDD reads off the VIN — Brand, Model (the programme),
ModelName, ModelYear (position 10), Engine, Transmission, BodyStyle, Trim,
Market, Plant, Driver, Emission — as constants or as lookups on given
characters, 2,473 rows in all, with the chart issue each model was taken
from. The document opens with an empty DOCTYPE, which the adapter removes
before parsing; a DTD with declarations would be refused.

F9 keeps the tables verbatim as text claims on a `VinDecodeModel` entity
(`VIN-MODEL-<n>`): a rule as `1..3=SAJ;12..12!=B`, an attribute as
`const=Jaguar` or `chars=6..7;01=X200|02=X200` with decoded text escaped.
Rows that decode to nothing, or whose value cannot span the characters they
are looked up on, are left out rather than taken for the table's fault; a
rule test that does not span its positions rejects the document, because
membership must not be guessed. `diagnostic_session::vin` applies exactly
those tables: ISO 3779's format check first (17 characters, no I, O, Q), then
the rule blocks in SDD's order, then the chosen model's attributes; every
model that claims the VIN is listed, the first is used, and every attribute
the table does not hold for this VIN is named as absent. The vehicle card
takes a VIN, shows what the tables read and pre-selects the programme and
the breakpoint whose year range holds the model year; the engine is left for
the tester to confirm, because the VIN table's engine wording and the DID
catalogue's engine names are different vocabularies and no mapping between
them is recorded.

Against the real tables (`cargo run --release -p diagnostic-session --example
catalogue -- <library dir> --vin <VIN>...`), two VINs *constructed* to satisfy
the rules — not real vehicles — decode as SDD's charts say: `SAJWA0FB9BRR12345`
→ Jaguar X250 (XF), model year 2011, 3.5L AJ-33, USA, LHD, automatic, from
"GlobalVIN.xls Issue:33"; `SALGA2EF4DA123456` → Land Rover L405 (New Range
Rover), model year 2013, 508PS AJ V8 5.0 SC, Solihull, automatic 8-speed. A
16-character string is named as not a VIN before any table is consulted.

## Slice 8 — module names from SDD's text database

The mnemonic-to-name table exists after all, in the multilingual text
database `CURRENT_PAG_MCP_TEXT_XML/Xml/Text/@J/`: one small document per
item, `@J_14229_M_DESC_<mnemonic>` for the ISO 14229 era (58 modules) and
`@J_M_DESC_<mnemonic>` for the earlier one (28), each with the name in twelve
languages tagged `xmlns:lang="eng"`, `"rus"`, `"deu"`, `"fra"`, `"ita"`,
`"jpn"`, `"ptg"`, `"ptb"`, `"esp"`, `"chs"`, `"kor"`, `"nld"`. F9 records
each language as a claim on the ECU family (`sdd_module_name.<lang>`, the
earlier family as `sdd_module_name_legacy.<lang>` so the ISO 14229 wording is
preferred where both exist); 86 documents, 1,032 records. The library indexes
them at load; the map and the module panel show the English name under the
mnemonic, and a programme-decorated mnemonic such as `LR_ABS_L316` is looked
up as `ABS` when it has no entry of its own. Found and fixed with M3: a name
claim carries no vehicle applicability, and the enumeration had counted any
record on an ECU family as placing it on the vehicle, so every named family
appeared in every survey as "does not state which vehicles it applies to".
Enumeration now places a family only by diagnostic claims — addressing, bus,
protocol, capability, SDD network and physical address — and the fleet
counts below are measured after that fix. Of the 114 ECU families the
platform documents declare, 60 have a name in the text database; the rest —
`CHCMB`, `FCIMC`, `HUD`, `IMC`, … — keep their mnemonic, as SDD does.

## Slice 9 — the interface in English, Russian and Ukrainian

The interface switches between English, Russian and Ukrainian from the
header; the choice is remembered on the machine and English is the default.
Every word the application says is translated — the cockpit, the session, the
tools — through one dictionary per language keyed by the English text
(`apps/scanner/frontend/src/i18n.ts`), so a string without a translation
shows its English rather than a key.

The vehicle data's words are not the application's, and follow a stated
rule (`dataLanguage` in the same file): SDD's text database carries twelve
languages, Russian among them and Ukrainian not. So with the Russian
interface the module names and the failure-type wording are SDD's Russian;
with the Ukrainian interface they stay SDD's English, because inventing a
Ukrainian rendering of JLR's text would be a translation this project has no
source for. Fault-code descriptions exist in English only in the payload
(`COMMON_SDD_DATA_DTC_HELP_LANG_EN`; the other pack is Chinese) and are shown
in English whichever interface is chosen. The owner decided this on
2026-09-04: Russian as a full interface, Ukrainian kept as the deliberate
choice of those who want it.

The failure-type wording comes from the same text database
(`@J_I_ISO15031_FAULT_TYPE_<hex>`, 97 items): the suffix is the failure type
byte in hexadecimal where `dtcFaultTypes.xml` counts in decimal — `17` is
0x17, "circuit voltage above threshold", which the decimal list holds as
number 23 — so the ingest converts and the claim lands on the same
`FTB-<decimal>` entity, one claim per language. Every fault code a module
reports carries the wording in every language the data has; the panel picks
by the interface language.

## Slice 10 — files go through the shell

A tester's report has to end up in a file the tester can find. The frontend
used a browser download link for every save, which a WebView2 window may or
may not honour, and the library's location was a typed path. Both now go
through the shell: `save_text_file` opens the native "Save as" dialog with
the suggested name, writes the text itself and returns the path, or `None`
when cancelled; `pick_directory` opens the native folder dialog. Both are
`async` commands, so the blocking dialogs never sit on the main thread. The
frontend's `files.ts` routes the four report saves and the new "Choose
folder…" button to them when the shell is present, and keeps the download
and hides the button in the browser preview. The dialog plugin is used from
the Rust side only, so no capability file is needed and the webview gains no
file access of its own.

State: `IMPLEMENTED / FIXTURE_TESTED` — built, linted and tested under Linux;
three frontend tests cover the browser fallback and the button. Not yet
opened on Windows; that waits for the CI build of this branch.

## Slice 11 — a new session, and the first run on Windows

The owner's first run of the CI build (2026-09-05, his own Windows machine,
MongoosePro JLR on USB): the application started without the adapter,
found it on hot plug, offered the connection, connected and verified the
board; the vehicle chosen by hand and by VIN decoded; the library loaded
through the folder dialog; every module of the survey shown with its
explanation; reports saved through the save dialog. Nothing touched a
car. That makes the F11 application `HARDWARE_CONFIRMED` for the adapter,
library, dialog and survey paths on real Windows, and still not
`VEHICLE_CONFIRMED`.

Two things he found. A black console window opened beside the application
— the release build lacked `windows_subsystem = "windows"`; fixed. And
there was no way to start over: repeating the survey redraws the map, but
the session report in the shell keeps accumulating, so the step legend,
which counts from that report, stayed as it was. "New session" in the
header now asks in a native dialog, then the shell drops the report, the
survey and every capture and read — the adapter connection and the loaded
library stay — and the interface restarts. A repeated survey on its own
still keeps the report, deliberately: correcting the year of the same car
is not a new visit.

A third thing he found unclear: the "first supported live profile" card
of F8 — Jaguar XF X250, 2010, 5.0 Supercharged, ECM, "live validation
pending" — standing beside a survey of whatever car he had chosen. It was
the era when that profile was the only vehicle the application knew. The
card is gone; the calibration panel now says what it is — a standard
OBD-II mode 09 request to the engine module on `hs-can`, the same on every
car with CAN diagnostics — and names the X250 as the only car it is
evidence-backed on, with the answer elsewhere "following the standard,
not yet confirmed".

State: `IMPLEMENTED / FIXTURE_TESTED` for the new action and the reworded
panel (frontend tests; shell built and tested under Linux); both reach
Windows with the next CI build.

## Slice 12 — the flow as the spine

The owner's verdict on the first Windows run: "everything mixed together,
no logic; only the legend is right". The legend was the session panel — the
seven steps with done, next and to do. So the steps became the structure.
The rail on the left is that list, sticky, each step a button that brings
its section into view; the sections on the right are the same steps in
order, each with its number, its title and its state, the next one framed:
Preparation (adapter and library), Vehicle (VIN, programme, survey),
Module network (the map and the module panel, where reads happen),
Listening and standard OBD-II (capture and the calibration read), Session
report (what was recorded, the one file). Nothing is hidden and nothing
moved between controllers; the header keeps its pills, "New session" and
"Save session report", and the browser demo shows the same page. Below
1100 px the rail sits on top and the sections stack.

State: `IMPLEMENTED / FIXTURE_TESTED` (frontend tests, browser demo); it
reaches Windows with the next CI build.

## Slice 13 — one visual system, judged live

The owner's remarks on the 9a6b08f build were about the look: buttons of
every height and colour, a squeezed rail, cards of different sizes beside
empty space, "cheap". Rather than a CI build per change, the Vite dev
server ran on the owner's own machine and he judged each change in his
browser at `http://localhost:1420/?demo=alpha`, remark by remark, the
same afternoon.

What settled, as a design layer at the end of the stylesheet, later in
the cascade on purpose: one control height (36 px) and radius for
buttons, inputs, selects and badges — a badge beside a button is the same
height and differs by colour only; rectangular chips everywhere, no
pills; every panel the same card with the same eyebrow-and-title heading;
sections as a slim heading row with a rule, no frame around the cards;
the frame of the window fixed — header, rail, footer — with only the step
column scrolling; the rail 300 px, one row per step, the hint on the next
step only, reaching the bottom of the window and showing the session's
vehicle at its foot (a side-view silhouette until model images exist,
with the decoded model, programme, model years and engine); the vehicle
card in two columns with the VIN level with the programme field; the
module network as one card, the map across its width and the module's
details underneath; the map's nodes in even columns; a chosen or hovered
thing shown by fill, never by a second border; the ground a light British
Racing Green with white cards on it.

The browser demo's vehicle became a realistic synthetic car so the look
could be judged with a full map: MY10 with two buses over the adapter and
a gatewayed sub-network, MY14 with the 2014-and-later hypothesised buses;
reads that answer with fault codes, silence or a refusal. It is called
SYNTHA and is not evidence.

The owner's verdict at the end of the pass: "splendid for a test
version". State: `IMPLEMENTED / FIXTURE_TESTED` (27 frontend tests, the
browser demo); it reaches Windows and macOS with the next CI build.

## What stays open

- **Map-converter scale.** Two paths were tried on 2026-09-04 and neither
  established it: the 122 runtime jars in the payload hold no converter
  implementation (the rendering is in native libraries this project does not
  reverse-engineer), and pairing each of the 165 map converters with a linear
  sibling of the same name yields only 9 pairs whose slope ratios disagree
  (1000, 1, 100, 0.003, …). The 404 map-converted parameters stay raw counts
  with the reason. The remaining path is a tester's read of the same quantity
  through a linear and a map identifier, compared (F13 intake).
- **Nothing here has met a vehicle.** The map's states are the survey's;
  the first live answers arrive with the tester programme (M6).

## Acceptance

- `diagnostic-session`: `decode` unit tests (6) — engine speed, offset-first
  and fractions, map withheld, mask and missing scaling, short response, no
  layout; `f10_session` gains 2 tests for the catalogue and the DTC join.
- `sdd-ingest`: `f9_did_golden` asserts the encoding descriptor now carries
  `scale`, `offset` and `offset_first`.
- `sdd-ingest`: `f9_vin_golden` (3) — every rule block and attribute kept
  verbatim, escaped text, the table version on the evidence, a rule test that
  does not span its positions refused; `f9_module_text_golden` (2) — a name
  claim per language, the earlier family kept apart, other text items
  refused. `diagnostic-session`: `vin` unit tests (3) and `f10_session` gains
  the module-name and VIN-decode tests on the synthetic fixtures.
- Frontend: `Language.test.tsx` (3) — translation by English text with
  placeholders and fallback, the header switch changing the whole interface,
  and SDD's data text following the Russian interface and staying English
  for the Ukrainian one; `ModuleSurvey.test.tsx` (4) — the picker, the survey drawn as
  lanes and nodes with SDD's name under the mnemonic and the unreachable
  module's reason in its panel, and a decoded VIN pre-selecting the vehicle;
  `ModuleRead.test.tsx` (2) — a hypothesised route marked on the map, a
  fault-code read with wording, an identifier read shown decoded with its
  unit; `NetworkCheck.test.tsx` (2) — the check reads every checkable module
  in lane order and marks what answered, and the lane grouping and node states
  are pinned; `SessionFlow.test.tsx` (3). Shell: `session_report_service`
  tests (2). `decode` gains the named-range test; `f9_did_golden` asserts the
  descriptor carries `states`. 22 frontend tests, 199 (51 suites) Rust tests, 21 shell
  tests, all passing; `scripts/check-architecture.mjs` OK.
- Real library, re-exported 2026-09-04 with the VIN document and the text
  database: `vin_decode.json` (447 records), `sdd_text.json` (183
  items, 2,196 records: module names and failure-type wording in twelve
  languages); loads with the rest.
- Preview: the cockpit was driven in the browser demo mode (survey → node →
  read) and rendered as designed on 2026-09-04.
- Real library: the `catalogue` example output above, reproduced on
  2026-09-04 from the re-exported library (126,706 records, unchanged count;
  every one of the 15,281 identifier parameters now carries its converter's
  arithmetic — 14,877 linear, 404 map).

## Evidence discipline

The catalogue, the wording join and the decoder read the store; they add no
records. The converter arithmetic is copied verbatim from
`gradex/Converters/*.xml` and is `OemDocumentation` evidence like the rest of
the DID catalogue. The J1979 agreement on `0xF40C` is a corroboration noted
here, not a record: it will become one when a tester's read is compared with
a known engine speed (F13 intake).

## 2026-09-09 — the engine variant

Some engines are split further by the data, and each half answers at its own
diagnostic address: the naturally aspirated V8 of an X250 is 4.2L or 5L, and
the instrument cluster and the parking-brake module sit at different
addresses on the two. `ProgrammeEntry` therefore carries `variants` beside
`powertrains`, filled from the `variant` dimension the ingest writes, and the
vehicle card offers an **Engine variant** field — only for a programme whose
loaded data splits something, so the description of an ordinary car gains no
extra question. Choosing a programme clears it, as it clears the engine.

Without it those modules are not guessed at: they resolve to nothing and the
map says why. That is the same rule as everywhere else — a car that has not
said enough gets a reason, not an answer.

## 2026-09-09 — fault-code wording in the interface's language

SDD holds a fault code's description in English only, and holds nothing at
all in Ukrainian, so translation cannot come from the data. It comes from
us, and only for the codes SAE J2012 defines: the table
`crates/diagnostic-session/data/dtc_standard_text.tsv` maps a standard code
to our own Ukrainian and Russian wording, written from what the standard
says the code means rather than translated from any manufacturer's phrasing.
Nothing derived from SDD enters the repository this way.

- `dtc_text::standard_texts` parses the table once and answers by code;
  `describe_dtc` fills `DtcDescription::description_texts` with it and never
  touches `description`, which stays the loaded library's English.
- `DtcSummary::description_texts` carries it to the interface, where
  `codeText` picks our wording for Ukrainian or Russian and keeps the
  English beside it. In English nothing is repeated.
- The session report is unchanged: it carries the English wording whatever
  the interface language is. A report is evidence.
- A manufacturer-specific code has no entry and falls back to the library's
  English. Its wording is the manufacturer's, and translating it would put
  their text, in another language, into ours.

The table was filled in nine passes and now covers **all 1,664
standard-range codes the library holds**: the powertrain P0 and P2 ranges
including the hybrid and electric families, the whole transmission range,
the restraints B0 range, the chassis and brake C0 range, and the
communication U0 range. A code outside those ranges is manufacturer-specific
and deliberately has no entry.

Terms are kept uniform across the table so that a reader learns them once:
`коло` for a circuit, `діапазон або робота` for range/performance, `низький`
and `високий рівень` for a low and a high circuit, `переривчастий сигнал`
for intermittent/erratic, `ряд`/`датчик` for bank and sensor. The repetitive
families — the traction battery's dozen voltage senses, the cell-balancing
circuits — are generated from that vocabulary rather than typed out, so a
typo cannot hide in one row of forty.
