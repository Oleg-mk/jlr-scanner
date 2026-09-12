# Capability matrix: SDD 169 against ProwlOne

Written 2026-09-12 at the owner's request: what a person gets from SDD at its fullest, what ProwlOne does today in stage 1 (read-only), what the roadmap adds in stages 2 and 3, and what the product could reach if the goal were the most function possible. Rows are not only SDD's functions: some things only ProwlOne does. This is a planning matrix, not a promise; `ROADMAP.md` decides order, `SAFETY_BOUNDARIES.md` decides what never happens.

## How to read it

| word | meaning |
| --- | --- |
| `yes` | SDD does it (seen in the corpus or in SDD's own scripts) |
| `built` | ProwlOne does it today, `IMPLEMENTED / FIXTURE_TESTED` at least |
| `partly` | part of it exists; the note says which part |
| `stage 1, open` | inside the declared stage-1 scope, not built |
| `stage 2` | stage 2 of `ROADMAP.md` — service functions, its own ADR and safety class first |
| `stage 3` | stage 3 — configuration; `ADR-0005` still forbids firmware |
| `possible` | technically reachable with this adapter and this data; not decided |
| `other hardware` | needs hardware this product does not target (VMM, a scope) |
| `JLR online services` | lives in JLR's online services, not in SDD's own data |
| `excluded` | excluded by the safety rules; a decision, not a technical limit |
| `no` | does not do it |
| `likely` | SDD very likely does it; not verified in the corpus |
| `not found in the data` | not found in SDD 169's data; may still exist in its runtime |
| `—` | not applicable, or already answered by the column before |

**Counts.** 99 rows. SDD does 63 of them. ProwlOne today: 36 built, 5 partly, 3 open inside stage 1. The roadmap: 20 in stage 2, 8 in stage 3. The ceiling: 53 possible, 3 need other hardware, 8 excluded for good. 10 rows are things SDD does not do and ProwlOne already does; 8 rows are things SDD does that ProwlOne will never do.

## Vehicle, modules, network

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Vehicle chosen from a programme and model-year picker | **yes** — by VIN or by hand | **built** — 55 programme-years from the library | **—** | **possible** — filled from the VIN read off the car | PLATFORM_*.xml (55); F11 |
| VIN decoding with SDD's tables | **yes** | **built** — a typed VIN | **—** | **built** | CURRENT_JLR_VIN_DECODE_XML (33 decoders) |
| VIN read from the vehicle (J1979 mode 09) | **yes** | **built** | **—** | **built** | obd-j1979 vehicle_info |
| Network map: buses, modules, gateways | **yes** | **built** — drawn as SDD draws it; every state explained | **—** | **built** | PLATFORM_*.xml; F11 M1 |
| Module survey with the route and the reason when unreachable | **partly** — shows what it reaches | **built** | **—** | **built** | F10, ADR-0013/0014 |
| Network check: who answers, codes from every reachable module | **yes** | **built** | **—** | **built** | CANLinkMonitorData.xml; F11 |
| Module passport: part numbers, serial, software level (F111/F112/F113/F18C) | **yes** | **built** — ADR-0027, F19: one screen across all modules; 34,788 identification records | **—** | **possible** — + software currency from the IVS lineage — a separate decision | PLATFORM data_identifier_set NET/SWDL/PDI; ADR-0027 |
| Software level checked against JLR's catalogue | **yes** — IVS part lineage, SOTA | **no** | **—** | **possible** — compare numbers; never download software | COMMON_JLR_SMPACK_XML/IVS (17 programmes) |
| CCF read (car configuration file) | **yes** | **stage 1, open** — in stage-1 scope | **—** | **possible** | PLATFORM ccf_source; CCF_DATA_*.xml (96) |
| Listen-only capture, nothing transmitted | **not found in the data** | **built** | **—** | **built** | F8/F10 capture |
| Mileage from every module in one click, differences, event stamps | **no** — one module at a time; and it can write one | **built** — 92 module families declare 0xDD01 | **—** | **possible** — + compared with earlier sessions | ADR-0024; SA_OdoWrite, F2_CAL_ODO on SDD's side |
| K-line and DS2: the early Range Rover, ABS/VIM on the base L316 | **yes** | **stage 1, open** — F14, after the first live tests | **—** | **possible** | CURRENT_JLR_KCODEC_RUNTIME; PLATFORM networks KW2000/DS2/ISO |
| SCP (J1850 PWM) and NVJCOM (JAGCAN) of the older Jaguars | **yes** | **no** — modules shown with the reason | **no** — not in the plan | **possible** — the adapter has the hardware; protocol stacks needed | PLATFORM_X100..X404 networks; F9 2026-09-07 |
| Modules behind a gateway (MOST, SUB_CAN1, NGI) | **yes** | **no** — gateway access is ROUTINE_CONTROL | **stage 2** | **possible** | PLATFORM gateway access_method |
| Battery state during the session | **yes** | **partly** — voltage as a live-read parameter | **—** | **possible** — a standing indicator with a warning | PLATFORM data_identifier_set BATT; VCON BATTERYCHECK |

## Fault codes

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| DTC read from one module (UDS 0x19) | **yes** | **built** | **—** | **built** | uds 0x19 02; F10 |
| Code and failure-type descriptions | **yes** — 93,521 descriptions, 257 failure types | **built** | **—** | **built** | rdsDtcHelp*.xml; dtcFaultTypes.xml |
| DTC status byte | **yes** | **built** — status mask | **—** | **built** | uds dtc.rs |
| Occurrence counters, extended data (0x19 06) | **not found in the data** | **no** | **—** | **possible** | — |
| Freeze frame, legislated OBD (mode 02) | **not found in the data** | **built** | **—** | **built** | obd-j1979 modes |
| DTC snapshot records per module (0x19 04) | **not found in the data** | **no** | **—** | **possible** | — |
| Legislated OBD codes (modes 03, 07, 0A) | **not found in the data** | **built** — 04 and 08 deliberately absent | **—** | **built** | obd-j1979 |
| OBD readiness monitors (mode 01) | **not found in the data** | **built** | **—** | **built** | obd-j1979 current_data |
| Monitor test results (mode 06) | **not found in the data** | **built** | **—** | **built** | obd-j1979 |
| Clearing codes (UDS 0x14, mode 04) | **yes** | **no** — the product's first write | **stage 2** — F12: own ADR, class, confirmation | **possible** — the report records what was cleared | SAFETY_BOUNDARIES; ROADMAP stage 2 |
| Fault history across sessions | **not found in the data** | **no** | **—** | **possible** | — |

## Live data

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Live parameters with units | **yes** — 15,281 definitions, 4,977 names | **built** | **—** | **built** | COMMON_SDD_DATA_SNAPSHOT catalogues |
| Parameter names in Ukrainian and Russian | **partly** — Russian yes, Ukrainian no | **built** — the product's own dictionary | **—** | **built** | ADR-0025 |
| Repeated reads at a stated cadence (16 pairs, 100 ms floor, 10 min cap) | **yes** — datalogger | **built** | **—** | **built** | ADR-0022 |
| Several modules in one live set | **likely** | **built** | **—** | **built** | ADR-0022 |
| Graphs and gauges | **yes** | **stage 1, open** — F15 visual layer; the first real report outranks it | **—** | **possible** | ROADMAP F15 |
| Recording to a file, CSV export | **likely** | **partly** — every sample in the report; no CSV yet | **—** | **possible** | ADR-0022 §5 |
| Legislated live data (mode 01) | **likely** | **built** | **—** | **built** | obd-j1979 |
| Derived channels, triggers, two-signal compare | **likely** | **no** | **—** | **possible** | — |

## Knowledge and guidance

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Help screen per code: causes, actions, set conditions | **yes** | **built** — the product's own words, 26,893 lines | **—** | **built** | ADR-0026; COMMON_SDD_DATA_DTC_HELP |
| Help and names in Ukrainian | **no** | **built** | **—** | **built** | ADR-0025/0026 |
| Symptom-driven tree without a code, pinpoint tests | **yes** — RULES: 56 documents, 78 MB | **no** | **—** — needs its own ADR | **possible** | COMMON_SDD_DATA_RULES |
| Symptom help pages | **yes** — 15 pages | **no** | **—** | **possible** | COMMON_SDD_DATA_SYMPTOM_HELP |
| Code leads to a parameter ('watch this signal') | **partly** — named in the prose | **no** — 223 codes name an identifier | **—** | **possible** | F9 2026-09-10 |
| What tests and routines a module declares | **yes** | **partly** — 1,153 ODST records known, unavailable | **—** | **built** | COMMON_SDD_DATA_ODST (93 modules, 123 tests) |
| Bulletins, campaigns, recalls | **JLR online services** — TOPIx | **no** | **no** | **possible** — from open sources only | — |
| Wiring diagrams | **JLR online services** — TOPIx | **no** | **no** | **possible** — from open sources only | — |
| Reference documents (heated seats, oscilloscope…) | **yes** — 13 PDFs | **no** | **—** | **possible** — the product's own texts | COMMON_SDD_DATA_PDF |
| Every unsupported case shown with its reason | **no** | **built** | **—** | **built** | F10; DEVELOPMENT_PRINCIPLES |

## Service functions

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Service interval reset | **yes** | **no** | **stage 2** | **possible** | SA_ServiceInt, FT_OILRESET |
| Brake bleed (ABS) | **yes** | **no** | **stage 2** | **possible** | CFG_BRAKEBLEED, SA_ArcAutoBleed, ABSBrakeBleedBridge |
| EPB service mode and release | **yes** | **no** | **stage 2** | **possible** | CFG_EPBRELEASE, SA_ParkBrkRelease, CFG_EPB_CLCHCAL |
| DPF regeneration, replacement, counters | **yes** | **no** | **stage 2** | **possible** | SA_DieselRegeneration, SA_DieselReplaceDPF, SF_DPFRESET, SA_DieselCounterReset |
| Adaptation resets: fuel trims, transmission | **yes** | **no** | **stage 2** | **possible** | SA_FuelAdapt*, ZF_TCM_ADAPTION_LEARN, SA_GM5ECUResetAdaptions |
| Air suspension and ride-height calibration | **yes** | **no** | **stage 2** | **possible** | SA_AirCal, AIR_SETUP, SA_L405_AirCal, SA_SuspCalib |
| Steering angle and DSC sensor calibration | **yes** | **no** | **stage 2** | **possible** | SA_SteerColCalib, SA_ABSSASSENSORCALIB, SA_DSCYawSensorCalib |
| Seat, door-glass, headlamp sensor calibration | **yes** | **no** | **stage 2** | **possible** | SA_DriverSeatCalibration, SA_DrvDoorGlassCalib, SA_HeadlampAxleSensorCal |
| TPMS sensor learn | **yes** | **no** | **stage 2** | **possible** | CFG_TPMS, SA_TpmWheelSensorTest, SA_TyrePressureConfirmation |
| Idle speed, purge valve, fuel-pump inhibit tests | **yes** | **no** | **stage 2** | **possible** | CFG_IDLESPEED, CFG_PURGEVALVE, CFG_FUELPUMPINHIBIT |
| Fuel and EVAP leak checks | **yes** | **no** | **stage 2** | **possible** | LEAKCHECK, SA_FuelLeakCheck, DW_ECM_P0442..P0456 |
| ARC hydraulics, active exhaust, active driveline tests | **yes** | **no** | **stage 2** | **possible** | SA_Arc*, SA_ActiveExhaustValve, SA_ActiveDriveline |
| Stop-start diagnostic, battery check and registration | **yes** | **no** | **stage 2** | **possible** | SA_StopStartDiagnostic, BATTERYCHECK |
| Injector coding and grading | **yes** | **no** | **stage 3** | **possible** | CFG_INJDISPLAY, DDE4InjectorGraderBridge |
| Settings: Terrain optimisation, Servotronic curves, disc-wear level | **yes** | **no** | **stage 3** | **possible** | SA_TerrainOptimisation, SA_ServotronicCurves, SA_DiscWearProtectionLevel |
| Flight recorder: read, capture, clear | **yes** | **partly** — event reads with mileage in F16 | **stage 2** — capture and clear | **possible** | SA_FlightRec*, CFG_FLIGHTREC |
| Guided drive cycles | **yes** — 7 profiles | **no** | **—** | **possible** — reads while driving | DRVCYCLE, DRIVE_CYCLE_*.xml |

## Actuators and self tests

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| On-demand self test (ODST, routine 0x0202) | **yes** — 123 tests, 93 modules | **no** — known, unavailable | **stage 2** | **possible** | COMMON_SDD_DATA_ODST; MDX ROUTINE_IDENTIFIERS |
| Actuator control (0x2F) | **yes** — ≈120 controllable identifiers in the data | **no** | **stage 2** | **possible** | MDX_*.xml CONTROLLABLE |
| Routines (0x31): calibrations, resets, checks | **yes** | **no** | **stage 2** | **possible** | MDX_*.xml ROUTINE; UMF scripts |
| Extended diagnostic session (0x10 03) | **yes** | **no** — everything is read in the default session | **stage 2** — most routines need it | **possible** | PLATFORM diag/prog sessions |
| Security access (0x27, seed/key) | **yes** — AlgData.dll | **no** | **no** — not in the plan | **possible** — only with the algorithms, which are JLR's; without them, unavailable | MDX SECURITY_REFS; CURRENT_JLR_ALGDATA_RUNTIME |

## Vehicle configuration

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| CCF editing: options, market, features | **yes** — 96 CCF descriptions | **no** | **stage 3** | **possible** | CCF_DATA_*.xml; config_model_support.xml |
| WriteDataByIdentifier (0x2E): module settings | **yes** — ≈2,300 writeable identifiers in the data | **no** | **stage 3** | **possible** | MDX_*.xml WRITEABLE |
| Legacy Jaguar configuration menus (X100…X400) | **yes** — 10 menus | **no** | **stage 3** | **possible** — after the SCP/NVJCOM stacks | CONFIG_MENU_*.xml; FEATUREPROG, RETROFIT |
| Set-up after a module replacement | **yes** | **no** | **stage 3** | **possible** | SA_*ECURenewal, SA_LcfToEcu, MODULE_SETUP |
| VIN write into a module | **yes** | **no** | **stage 3** — only as part of a module replacement | **possible** | SA_KVMVinLearn, CFG_VINLEARN |
| TPMS module or receiver replacement | **yes** | **no** | **stage 3** | **possible** | SA_TyrePressureModuleReplace, SA_TyrePressureReceiverReplace |
| Odometer write | **yes** | **excluded** | **excluded** | **excluded** — ADR-0024: never | SA_OdoWrite, F2_CAL_ODO |
| Navigation map activation (InControl, NGI) | **JLR online services** | **no** | **no** | **no** — needs JLR's services | ict-mapupdater.exe, SA_NGImapUpdate |

## Keys, security, programming

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Key programming, passive entry | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | SA_KeyProg*, SA_PassiveKey*, PATS |
| Erase and recover keys, history | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | SA_EraseKeys, SA_RecoverKeys, SA_KeyProgHistory |
| Immobiliser target IDs, start authorisation | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | SA_Write*Sid, SA_StartAuthorisation, SA_PcmSecurityWrite |
| Legacy ECU coding (Omitec) | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | CURRENT_JLR_MCP_APP_OMITEC_* |
| Module reflashing (VBF), software update | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | COMMON_JLR_VBF_UPDATES_FLASH (479); SWDOWNLOAD, FLASH*, IVS |
| PDI programming, software service actions | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | SA_PDI; IVS VehicleServiceActions |
| Arbitrary CAN transmit | **yes** | **excluded** | **excluded** | **excluded** — safety rule, ADR-0005 | engineering tools |

## Measurement (separate hardware)

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Oscilloscope and multimeter (VMM) | **yes** | **no** | **no** | **other hardware** — a USB scope; not in the plan | COMMON_VMM_SYSTEM; OSCILLOSCOPE.pdf |
| Vibration analyser (JVA), NVH tests, wheel balance | **yes** | **no** | **no** | **other hardware** | CURRENT_JLR_JVA_RUNTIME; idu_nvh_* |
| Sensor probe tester (UST), pipe tester | **yes** | **no** | **no** | **other hardware** | CURRENT_IDS_UST_UST (542 pages); PipeTester |

## Reports, history, evidence

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Session report | **likely** | **built** — one bundle: survey, captures, reads, live runs, mileage; stamped | **—** | **built** | F11; ADR-0019 |
| Evidence mark on every row (SYNTHETIC / SOURCE_BACKED / VEHICLE_CONFIRMED) | **no** | **built** | **—** | **built** | DEVELOPMENT_PRINCIPLES |
| A tester's report becomes evidence (intake) | **no** | **built** | **—** | **built** | F13; ADR-0016 |
| Unknown stays unknown: validation states | **no** | **built** | **—** | **built** | DEVELOPMENT_PRINCIPLES |
| Vehicle history across sessions, trends | **likely** — dealer database | **no** | **—** | **possible** | CURRENT_JLR_DEALER_DATABASE_RUNTIME |
| Print, PDF, sharing | **likely** | **partly** — the bundle file | **—** | **possible** | — |

## Platform and ease of use

| Function | SDD 169 | ProwlOne now (stage 1) | Roadmap (stages 2–3) | Ceiling | Source |
| --- | --- | --- | --- | --- | --- |
| Windows 10/11 and macOS (Apple Silicon) without a virtual machine | **partly** — officially no; unofficial builds install | **built** | **—** | **built** | ROADMAP positioning 2026-09-05 |
| Automatic adapter discovery, no install ritual | **no** | **built** — inbox usbser driver | **—** | **built** | F3; F8 |
| MongoosePro clones | **likely** | **not found in the data** — USB ids unknown; first thing collected from a tester | **—** | **possible** | ROADMAP positioning |
| Interface languages | **partly** — 12 languages, no Ukrainian | **built** — English, Russian, Ukrainian | **—** | **built** | F11; F17/F18 |
| Bench: a virtual vehicle for practice and CI | **no** | **built** | **—** | **built** | ADR-0020 |
| One safety class per operation; unknown is unavailable | **no** | **built** | **—** | **built** | SAFETY_BOUNDARIES |
| No JLR subscription or account | **no** — a subscription to talk to a car | **built** | **—** | **built** | ROADMAP F9 access path |
| Works offline | **partly** — authentication and updates online | **built** | **—** | **built** | CURRENT_JLR_AUTHENTICATION_RUNTIME |
| Knowledge library separate from the app, stamped and dated | **—** | **built** | **—** | **built** | ADR-0019; G4 |

## The theoretical ceiling — could the excluded rows be done at all?

Recorded 2026-09-12 from the owner's question, so the analysis is not run twice. Of the rows SDD does and ProwlOne will never do, what is a true technical wall and what is only a line this project draws? **For information; it decides nothing. `ADR-0005` and `ADR-0024` stand.**

The honest answer is that almost all of it is technically possible. The hardware transmits — the adapter already sends frames on the read path (`transmit_diagnostic_frame` in `mongoose-jlr`); the protocol is standard UDS, so a write is the same machinery with a different service (`0x2E` write, `0x31` routine, `0x34`/`0x36`/`0x37` download, `0x27` security access); and the data is mostly on disk — 371 `.vbf` firmware payloads, the DID access classes, the routine ids, even `AlgData.dll`. Three barriers stand, and only one is a true technical wall:

1. **Security access (`0x27`, seed→key) — the real wall.** Keys, immobiliser, most coding, reflashing, PDI and odometer write all sit behind it: the module issues a seed and the tool must return the correct key or is refused. That key is computed by JLR's secret algorithm. It exists physically on the owner's disk as `CURRENT_JLR_ALGDATA_RUNTIME/Runtime/AlgData.dll` (104 KB, a compiled JLR binary), but using it means reverse-engineering a proprietary secret — the exact *security programming* line `ADR-0005` draws. This project has never touched it. Without it, everything secured is unreachable no matter what is built.
2. **Arbitrary CAN transmit — no wall at all.** Technically trivial: the wire write already exists, and a `send_raw` would only expose it. It is blocked purely by this project's own architecture checker.
3. **Risk and policy — decisions, not limits.** A wrong or interrupted flash bricks a module (recovery is a bench or a tow); an odometer write is the fraud the mileage survey exists to expose (`ADR-0024`). These are chosen lines, not technical ceilings.

| Excluded capability | Technically possible? | What stands in the way |
| --- | --- | --- |
| Arbitrary CAN transmit | **trivial, now** | The wire write already exists (transmit_diagnostic_frame); send_raw would only expose it. Blocked solely by our own rule (check-architecture.mjs). |
| ECU coding (0x2E) | **partly now** | Unsecured writes are reachable now; secured ones sit behind security access. |
| Odometer write | **technically yes** | A simple write/routine protocol-wise (SDD's own SA_OdoWrite). It is the exact fraud the mileage feature exists to expose. |
| Keys | **yes, behind 0x27** | The protocol is simple (0x27 + routines/writes); the key computation is JLR's secret. |
| Immobiliser | **yes, behind 0x27** | The same target IDs and start authorisation, all behind security access. |
| VBF reflashing | **yes, behind 0x27** | 0x34/0x36/0x37 + routines; the .vbf format is known and 371 files sit on disk. An interrupted flash bricks the module irreversibly; recovery is a bench job. |
| PDI programming / software service actions | **yes, behind 0x27** | A superset of reflashing plus IVS orchestration (part lineage, SOTA). |

So the answer to *could we, in theory* is **yes — except for the secured operations, where a secret that is JLR's stands in the way**. The ceiling is not in the code and not in the hardware; it is in the three lines this project chooses not to cross.

## Where the SDD column comes from

Read on 2026-09-12 from the locally extracted SDD 169 corpus (never committed, see `research/sdd/PROVENANCE.md`), by component:

- `CURRENT_JLR_XCL_XML_DATA_XML/Xml/PLATFORM_*.xml` — 55 programme-year documents: buses, gateways with `access_method ROUTINE_CONTROL`, identifier sets `NET`, `SWDL`, `PDI`, `BATT`, the `ccf_source` of each car;
- `…/<PROGRAM>_<YEAR>/MDX_*.xml` and `<ECU>.xml` — per-module data indexes: over the CURRENT tree about 8,000 `READABLE`, 2,300 `WRITEABLE` and 120 `CONTROLLABLE` identifiers, routines, `SECURITY_REFS`, `swdl` properties, `securitymethods`;
- `CURRENT_JLR_MCP_APPS_DA_RUNTIME` (507 scripts) and `CURRENT_JLR_VCON_RUNTIME` (173) — SDD's applications by name: `SA_*` service actions, `CFG_*` procedures, `SF_*` service functions, `FLASH*`, `SWDOWNLOAD`, `MODULE_PROG`, `SA_KeyProg*`, `SA_OdoWrite`; `CURRENT_JLR_MCP_APP_OMITEC_*` (27) — brake bleed, injector grading, ECU renewal and coding; `CURRENT_IDU2_APPLICATIONS` (43) — NVH and component tests;
- `COMMON_JLR_SMPACK_XML/IVS` — 17 programme documents of part lineage and software levels; `SERVICE_RULES_*` (70);
- `CURRENT_JLR_MCP_XML_XML` — `CCF_DATA_*` (96 with the XCL tree), `CONFIG_MENU_*` (10), `DRIVE_CYCLE_*` (7);
- `COMMON_SDD_DATA_*` — `SNAPSHOT` (15,281 parameter definitions), `DTC_HELP` (6,168 documents), `ODST` (93 modules, 123 tests), `RULES` (56 documents, 78 MB), `SYMPTOM_HELP` (15), `PDF` (13), `QUAL` (472);
- `COMMON_VMM_SYSTEM`, `CURRENT_JLR_JVA_RUNTIME`, `CURRENT_IDS_UST_UST` — the measurement hardware's software; `CURRENT_JLR_VIN_DECODE_XML` — 33 decoders; `CURRENT_JLR_ALGDATA_RUNTIME` — the seed/key algorithms; the text database `CURRENT_PAG_MCP_TEXT_XML` for the names SDD gives things.

What SDD's interface does at run time — printing, recording, history — is not in the data and is marked `likely` rather than asserted. The ProwlOne columns come from `CURRENT_STATE.md`, `ROADMAP.md`, `SAFETY_BOUNDARIES.md` and the code on 2026-09-12.
