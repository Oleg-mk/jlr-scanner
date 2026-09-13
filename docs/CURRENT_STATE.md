# Current State

This file records what is done. `ROADMAP.md` records what is next and why in that
order, including the owner-only gates. Keep both current; neither may live only
in a chat session.

Status: **2026-09-13 — the fault-code help is SDD's own text, in English or Russian (`ADR-0034`; supersedes `ADR-0026`).** The owner withdrew the translation layer: the 27,493 rows of `help_texts.tsv` and the module that read them left the product, and no help text is translated by anyone — not by hand, not by a machine, not later. What decided it, in order: a 14B model on the owner's card had turned *exhaust gas recirculation bypass solenoid circuit* into the turbine vane actuator with every code and number intact; the rows already written were then measured against SDD's own text for the first time — 99.2 % carried the same content words and none had swapped a component, but 56 had replaced a specific procedure (*check the neutral sensor, refer to the circuit diagrams, check the circuit, then suspect the powertrain control module*) with one generic sentence about confirming the diagnosis; and SDD 169 turned out to ship the whole layer in Russian, `COMMON_SDD_DATA_DTC_HELP_LANG_RU`, the same 6,171 documents and 42,637 named texts with only the words changed, a Russian text for every one of the 38,526 names the library's screens use. The owner's scripts opened that pack from the installer (every file verified against the cabinet's MD5, the installer's SHA-256 confirmed); the exporter now takes a `_LANG_RU` root and writes each screen's Russian beside its English — `sdd_help_screen.<NAME>.rus`, `sdd_help_screen_items.<NAME>.rus` — from a source of its own, contributing text and nothing else, and refusing a document whose own `<language isoCode>` is not the one it is read as. The session joins a Russian line to its English by the mnemonic's name, so a name the Russian screen lacks keeps English and the lists stay the screen's length; a library issued without the pack reads as it always did. The help panel and the self-test list carry one switch, **English or Russian**, remembered on the machine; a Ukrainian interface reads Russian by default. Ukrainian is not offered for this text and the ODST descriptions are English until the ODST Russian pack is opened the same way. The library was re-exported the same day with the Russian root: 499,765 records over 10,298 sources, `dtc_help.json.gz` now 9,887 manifests — 6,168 English and 3,719 Russian, the 2,449 Russian documents whose screens no car selects producing no manifest rather than a rejection — with 36,890 screens in each language, every Russian screen's items in the same order as its English twin, and `rejected: 0`; the owner's copy is stamped **2E57-6A34**, valid to 2027-09-13, 29.1 MB zipped. **Build 0.9.9** was cut the same morning on `c8bdbaf` (CI run 34737814836, all four jobs green): `ProwlOne_0.9.9_x64-setup.exe`, 5,321,004 bytes, SHA-256 `99a29809fad11dd80d1d688f411864e00af8ab39e88ca5e76f317128377d40ec`; `ProwlOne_0.9.9_universal.dmg`, 14,708,795 bytes, SHA-256 `f73e43544548777072d2a651f2b8ffccb871b1b34e5cfd01182090de898ee484` — in `Downloads\prowlone-build-0.9.9` and on the owner's desk beside the stamped library. `IMPLEMENTED / FIXTURE_TESTED`. Not decided: SDD's Russian for the code descriptions, which the same pack carries.

Earlier — **2026-09-12, night — the self tests speak, and the local translation is abandoned.** Two files of wording entered `help_texts.tsv`: the 143 most-used help lines that had none, and all 458 unique texts of the ODST pack, in Ukrainian and Russian, written here rather than by a model on the card. The table went from 26,893 rows to 27,493 — 27,350 by name and 143 by the fingerprint of their line — and the self-test list now shows a test's words in the reader's own language, through the same `help_text::screen` the fault-code help uses: our words where we have them, SDD's English line by line where we do not. Three lines already had a row: two took the wording that was already shipped, so one sentence does not appear in two forms, and one kept the row it had, because the table already uses that name for another sentence. **The local translation was stopped and discarded.** A 14B model on the owner's card had done 12,136 of 15,743 lines when a hand check of thirty random rows found nine with errors — and one of those was not clumsiness but a different component: *exhaust gas recirculation bypass solenoid circuit* came back as the variable-geometry turbine vane actuator, confidently and grammatically, with the codes and numbers intact, so the script's own checks passed it. In a diagnostic tool a line like that is worse than English: it sends a person to dismantle the wrong thing. Nothing from that run entered the library or the table; the cache stays in a temporary folder. What the corpus is, measured while deciding: 15,062 unique lines, 1.43 million characters, 221,717 words — and it is not templated (13,775 distinct frames, the top 5,000 covering 42%), so substitution cannot do it either. What is left, counted from the library rather than estimated: of 38,526 named help lines, 13,415 — 13,130 distinct sentences, 1,230,272 characters — carry no words of ours, and **every one of them is reachable**: the help layer names 21 vehicle programmes, and no line belongs to none of them. An earlier count of 12,412 with a said-to-be-unreachable remainder summed only the ten largest programmes and is withdrawn. How that text gets its words was decided the next day: it does not — `ADR-0034`, above. Earlier the same day — a K-line module is read, end to end (`ADR-0029` slice B complete; `IMPLEMENTED / FIXTURE_TESTED`, `HARDWARE_CONFIRMED` for the adapter's own words). The debt of slice B is paid: the shell reads a module on a serial line in the panel the reader already uses, and the bench answers it. A module the data places on DS2 or KWP2000 offers its own protocol's operations — the fault memory, and the identification on DS2 — and asks for no identifier, because a serial line has none to choose. A KWP2000 identification is refused in as many words: the local identifier SDD states per module does not reach this library yet, and this product does not send a default option to a car. Every K-line read leaves the record a single read leaves, so `report-intake` needed no change at all — the node address stands where a CAN identifier would, written as `node 0x72`. On the bench, `BenchRoute` gained the two lines, the stand-in adapter takes the K-line resources, the single-pin selection and the two line commands — answering the fast init the way the real firmware did, with a timeout when nothing is on the line — and the vehicle answers DS2 and KWP2000 in their own framing, the identification a `BN`-prefixed string no real part carries. The shell's end-to-end bench test now reads DS2MOD's identification and its fault memory through the whole stack and fails if either row reads anything but `SYNTHETIC`. What remains is not code: no K-line module has ever answered this application, and the one adapter command still unproven — the byte framing both these protocols want even parity for — is settled by the first L322 body module that answers or stays silent. Earlier the same day — the K-line is built, and the adapter has answered about it (`ADR-0029` slice B; `IMPLEMENTED / FIXTURE_TESTED`, `HARDWARE_CONFIRMED` for the words the probe settled). The owner plugged the MongoosePro in and asked for the half that needs no car first, then the probe. Both are done. The half without hardware: `kline-execution`, the fourth execution crate — the four reads of `ADR-0029` and no fifth, the architecture check counting them — compiling a resolved plan into DS2 or KWP2000 bytes and reading the answer back, refusing what it cannot know: a protocol not spoken, an addressing mode that is not a node address, a plan with a CAN identifier format on it, a bus that states no framing or no wake-up. The resolver's plan gained `serial_framing` and `serial_wakeup`, filled from the two text claims the platform ingest already wrote on the bus, which closed the last gap between what the data says about a line and what an execution layer needs to open one. **Then the adapter was asked.** On the bench, with no vehicle: resource 3 opens at 9600 and resource 4 at 10400, both status 0; `cSetPin(1, 7, 0)` is taken on both — and pin 99 and pin 0 are **refused**, with the firmware's own words *"MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8"*, which is what makes the acceptance mean something. The same run named three more of the firmware's commands from its own status texts: `0x0011` is **Fast Init** — it answered *"Fast Init: Timeout on response"* with nothing on the line — so the `kw2000_fast` wake-up the platform documents name is a command this adapter performs itself; `0x0013` is `cGetString`; and `0x0014` is `SetData`, **a write to the adapter**, which the architecture check now forbids the K-line code to so much as name. One thing is still a guess and says so: the parity. Both protocols this product speaks on a K-line want even parity — DS2 9600 8E1, KWP2000 10400 8E1 — and `0x0010` is a handled command that takes J2534's own `SCONFIG` shape, but it takes a parity of 99 and a parameter that does not exist just as readily, so its zero status proves nothing; the first L322 body module that answers settles it. Evidence and the full transcript: `docs/evidence/mongoose-probe-2026-09-12/`. What slice B still owes is the shell read through the shared record and the intake, and the bench's DS2 and KWP2000 answers. Earlier the same day — the software a module carries, against JLR's own catalogue (`ADR-0033`; `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`). The passport reads what a module says it is; the next question is whether that is the software the unit is supposed to carry, and JLR answers it in the corpus. `COMMON_JLR_SMPACK_XML/IVS` is JLR's own record of which software and calibration part numbers belong to which assembly, and every passport number is now set against it: **the same number**, **the catalogue names …** with the number and SDD's part type, **the catalogue names no part here**, or **the catalogue does not carry this assembly**. Four states and not one of them a verdict — the word *outdated* is in no language file and two tests fail if it ever appears in English, Ukrainian or Russian, because the data cannot support it: `PartLineageId` carries a sequence number but only 26 of 59,254 lineage bases appear with more than one, so the corpus says what belongs together and never what replaced what. The line under the table gives the catalogue's own date, 2022-10-17, and says a difference is a difference: a replaced unit, another market, a later update all produce one. **Nothing that could programme a module was taken.** The same component carries service actions and coordinated flash lists; a document is ingested only where SDD stamps it Production and Validated, never where it holds one of those — in SDD 169 exactly the three test files — and the attributes that exist only to drive programming (`SOTAEnabled`, `CertificationRequired`, `ProgInSvc`, `ConfigurationMethod`) never enter the library at all. `ADR-0005` is untouched and reflashing stays excluded; this compares numbers and changes nothing. Real source: 20 documents found, 5 skipped (three test files, two byte-identical copies), 15 ingested, nothing rejected — 67,755 assembly records naming 258,704 parts over 15 programmes, 122 module families and 39 identifiers. The library went to 425,799 records from 358,044, 21 MB to 24 MB packed; the owner's copy re-stamped, code 0363-3484, valid to 2026-10-12. The join was measured before it was built: the passport reads identifiers on 845 programme-and-module pairs, the catalogue names parts on 878, 581 are the same pair, and on every one of those 581 the identifier the catalogue says the assembly answers on is one the passport actually reads; of the 11,332 identifiers read on those pairs, 2,586 have a catalogue part to be set against. One assumption is named where it can be found again: nothing has met a car, so the shape a module answers a part number in is not known, and `knowledge::part_number` compares letters and digits only — a tester's capture will confirm or correct it. Earlier the same day — a module says what it can be asked to do (`ADR-0032`; `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`). SDD carries, per module, the on-demand self tests that module runs on itself when a technician commands it — a pump motor test on an ABS, a lamp check, an actuator cycle — with a name, a running time, a timeout and a screen of instructions written for whoever runs it. Until now this project recorded only that such a capability existed: no name a person reads, no description at all. The module panel now ends with that list, and the list is all it is. **This build runs none of them and the section has no button**: a self test is a routine, not a read, and routines are stage 2 with their own ADR, their own safety class and their own confirmation. Every record still carries `SERVICE_ROUTINE`, no `RoutineControl` constructor exists anywhere in the product, and the architecture check still forbids one by name. The ingest now writes the test's timings, which help screen a given car is given, and what each screen says — the same chain `ADR-0026` already uses for fault-code help — so the ODST slice went from 1,153 records to 4,085 over the same 93 documents: 1,153 test descriptions, 1,257 screen bindings, 522 screen records over 462 distinct screens, 238,504 characters of SDD's prose. The library re-exported to 358,044 records (from 355,112), nothing rejected; the owner's copy re-stamped, code CE99-19E2, valid to 2026-10-12. What a car sees: 123 module-and-test pairs over 23 programmes — an X250 shows 37 modules with a test each, an X351 43, an L322 30. One honest limit is printed above every list: SDD qualifies a test by a model-year marker (`MY10`, `MY02_5`) that the ODST and DTC ingests record on `sdd_model_year` while the DID catalogue records the same vocabulary on `sdd_year_breakpoint`, and a session states only the second — 1,242 of the pack's 1,355 qualifiers carry such a marker, so demanding a match would have listed nothing at all on any car. The list is therefore what the data does not rule out, which is the rule the fault-code help already uses, and each row shows SDD's marker verbatim. Unifying the two dimensions is named in the ADR and not done there. The texts are SDD's English: the pack holds English and Chinese only, and this project's Ukrainian and Russian wording for those 473 lines is queued behind the help file now on the card. Earlier the same day — the session becomes a document a person can read (`ADR-0031`; `IMPLEMENTED / FIXTURE_TESTED`). The bundle is written for a machine and reads like one; a tester sending results, a workshop showing an owner what was found, and this project reading a session six months later all want prose and tables. The application now renders the same session as a document: the car, the adapter, the library with its issue stamp, the modules with the reason beside each unreachable one, and a section for every kind of read, each saying in its heading what its values are worth and carrying its own caveat under the table. It prints through the platform own dialog, where a PDF writer is a printer like any other, and saves as one web page that carries its styles inside it, so it opens and prints anywhere with nothing behind it. A switch masks the VIN and the adapter serial in the document while the bundle keeps them, which is what the guide has been asking testers to request. After saving, the application says where the file went and offers to show it in the folder, so it can be attached to a message: sending stays a human act, and the application creates no account and uploads nothing. Nothing is recorded and no operation is added: the document is a rendering, the bundle stays the record, and `report-intake` reads the bundle alone. On the bench the head of the document carries a band and every table says `SYNTHETIC`, and the save button is refused with the reason. Checked by 6 interface tests (103 in all), one of which fails if the document ever draws a conclusion of its own. No build: the owner batches the work. Earlier the same night — **the live series leaves as a spreadsheet (`ADR-0022` §5, amended; `IMPLEMENTED / FIXTURE_TESTED`).** Every sample has been in the session bundle since the live read was built, and a tester who watches a value move wants it in a spreadsheet rather than inside a JSON document. The run now exports itself: one header and one row per parameter per sample, with the clock, the module, the identifier, SDD own name for the parameter, the value, the unit, the decoder note, the response bytes, and what the reading is worth in the last column of every row — `SYNTHETIC` on the bench, so a row pasted anywhere still says what it is. A sample that decoded nothing is a row all the same, with the module refusal or the failure where the value would be. Fields are quoted only where a spreadsheet would misread them, and the file is written when the person asks, from the run still in hand, never streamed during a run. It goes through the one command that writes files, so the bench still writes nothing to disk. The button sits in the live-read panel and appears once a run has samples; the path it wrote to is shown. No new operation and no new safety class: the export is a rendering of the record, not a second record. Checked by the escaping test where the rule lives, the shell end-to-end bench run (every exported row marked `SYNTHETIC`) and 2 interface tests (97 in all). No build: the owner batches the work. Earlier the same night — **the battery, read and shown as a standing indicator (`ADR-0030`, F21; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car).** The owner took the next row from the capability matrix and set its shape: the battery is not one more parameter in a list of two thousand but the rail every read depends on, so it belongs on the session column as a card of its own. The corpus was read first: the 45 platform documents name, per module and per car, the whole battery-monitor dataset — 106 module-identifier pairs over 17 programmes, served by `GWM`, `BCM`, `RSJB`, `BECM`, `FSJB`, `IPC` and `PSCM` — state of charge and the lowest it has been, estimated temperature, current, quiescent current in three windows, cumulative charge and discharge, amp-hour loss, time in service, monitor resets, estimated cold cranking voltage, battery type, the generator set point, the quiescent relay box, the odometer at the last five shutdowns, the second battery of a dual-battery car, and on a hybrid the traction battery from its own module. A dedicated `type="BATT"` set exists and holds exactly one identifier, the current, so the dataset had to be found where it actually lives: inside each module's own `NET` set. What is **not** there is as important: SDD keeps no state of health and no internal resistance for the 12-volt battery, so this product computes none. Which identifiers are the battery is a rule in the knowledge layer, `ADR-0024`'s precedent — a curated table of identifiers plus a name check, both of which must pass, which is what keeps `Turbocharger valve offset values` out of a card a word match would have let it into. The bytes come from a join no single document makes: the platform document names the module and the parameter, the DID formatting document describes the bytes of 18 of them and names no module, and joined at ingest they make a readable parameter scoped to the module that serves it; a parameter whose bytes nothing describes is shown as the bytes it is, never scaled by guess. Real source: 1,486 battery records in the exported library, which grew to 355,112 records (from 353,626), nothing rejected, the owner's copy re-stamped (code D9C9-6B50, valid to 2026-10-12). The read is one operation, `BATTERY_STATE`, class `READ_ONLY`, stepping like the passport and the configuration: one request per module and identifier, every parameter of that identifier decoded from the one answer, every read leaving the record a single read leaves, so a tester's battery read confirms a route exactly as a passport read does — and no value of the battery ever reaches a manifest. The interface is two things, after the owner saw the first drawing and said the rail's card should stay a card: in the session column, between the steps and the vehicle, a level bar with the state of charge, three tiles for voltage, current and temperature, the time it was read and the button that reads it; and in the working area, beside the passport and the configuration, a battery panel with every group the rule names, each row under SDD's own name in the user's language, a switch for the rows that answered nothing, and one line saying the application states no threshold of its own and says nothing about whether the battery is good. Thirty-odd rows are a panel's worth of reading and not a rail's; whether that panel becomes a dashboard of its own is a later pass. Nothing is written: registration and monitor reset stay out of stage 1, and `0x4020` is read as a count of them. On the bench the battery answers believable values, every row marked `SYNTHETIC`. Checked by the battery rule's own tests, two platform golden tests, the readable-identifier gate's list, the shell's end-to-end bench run, the intake's new test (no reading reaches a manifest) and 7 interface tests (95 in all). No build: the owner batches the work. Not decided: a health percentage of any kind, repeating the read at a cadence, and anything about the traction battery beyond showing what the module answers. Earlier the same day — **the K-line buses enter the product, first slice (`ADR-0029`, F14 brought forward; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met an adapter or a car).** The owner took the third row from the capability matrix — the early Range Rover's body electronics and the Defender L316's ABS and VIM, the only SDD-era modules the product cannot reach over CAN — and brought F14 forward from after M6: "робимо зараз, з внесенням необхідних правок; якщо будуть проблеми — вирішимо на діагностиці саме цих старих авто". The corpus was read first: six `<iso>` buses across the L322 MY06–07 and L316 MY07–12 documents, each stating its rate (9600 or 10400 baud), its byte framing, its wake-up (`bmw_ds2`, `bmw_kw2000_star`, `kw2000_fast`, `rosco`) and, for four of them, the J1962 pin (7 or 8) by the connector's own name; the modules on them carry one-byte node addresses. Three standings, kept apart: the buses, framing, pins and node addresses as *documented* knowledge from the platform documents — 18 bus records (nine of them the `ISO` buses of the X100–X400-era Jaguars, recorded as their documents state them and bound to no route), 70 node-addressed module records (43 of them those Jaguars' NVJCOM units, a protocol named and not spoken), 44 capability records from the real corpus, the library re-exported to 353,626 records (from 353,450), nothing rejected, the owner's copy re-stamped (code 2CD9-E6D0, superseded the same night by the battery slice’s copy); the adapter routes `k-line-7` and `k-line-8` as an `unverified_research` *hypothesis* in a sixth built-in manifest, because this adapter has never opened a K-line with a pin selected; and two protocol crates *implemented offline* against the public specifications — `ds2` (the frame, identification, fault memory, 9 golden tests) and `kwp2000` (ISO 14230 in both length forms, the link handshake, `ReadEcuIdentification`, `ReadDTCByStatus`, 11 golden tests) — with KWP2000\* and ROSCO recorded on their modules as named and not spoken. Nothing that writes: no clear-fault service in either crate, the encoders private, the architecture check naming the forbidden constructors and counting the request constructors. The resolver learned one rule: a CAN identifier format is required under a CAN addressing and not asked for under the serial node addressing `iso9141_node`, and both execution bridges refuse a plan without one. The survey resolves a DS2 or KWP2000 module against its protocol's own capabilities and shows every such module with its bus, pin, baud, protocol and node address — a hypothesis where the protocol is spoken, the reason where it is only named — and says in as many words that the line is not opened yet; the interface table shows the two routes as a *route hypothesis*, a third implementation state. No request travels on a K-line: the routes refuse every open. Checked by the platform golden test, the survey tests (six modules, two hypotheses), the F6 rule, the route tests, 20 protocol golden tests, the shell's tests and 88 interface tests. No build: the owner batches the work. Next: the second slice — the K-line execution path, the device's pin-select, wake-up and outbound commands as hypotheses, and `mongoose_kline_probe.py` to try them on the adapter alone; then the first L322 or L316. Earlier the same day — **the car configuration file, read as SDD reads it (`ADR-0028`, F20; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car).** "CCF read" had been in stage 1's own definition since 2026-09-01 without a line of code; the owner took it as the second row from the capability matrix. The corpus was read first: SDD describes one car's configuration in a `CCF_DATA` document — 48 of them, 20 programmes, `MY04_5` to `MY17` — as the module keeping the master copy (`GWM` 19, `BCM` 14, `IPC` 7, `RSJB` 6) and the modules holding copies; blocks each read with `0x22` from one identifier at an offset and a length (`0xF106` for 196 bytes on the X250 of 2010; `0xF105` carrying two blocks at two offsets); and a layout of 38,505 parameters and 141,125 options, of which SDD's own editor displays 2,871. Two schemes: 30 documents address every block plainly and are read; 18 — the gateway cars from 2014 on — page the `CCF` block through VDF blocks over `0xEE00`, and those cars are refused with the reason rather than guessed at. What entered the knowledge base: the block identifiers in the catalogue's shape (`ccf=<BLOCK>;offset;length`, 335 over six modules) so the read is a module read the gate admits; the layout as its own entity kind, `ConfigurationParameter`, one record per parameter with the titles and options resolved in English and Russian through all 6,981 items of SDD's text database; the scheme and the sources as claims on the programme. Nothing that leads to a write: `serviceIdWr`, the VBF names and the memory-read sources are left where they lie. The library exported afresh holds 353,450 records (from 314,462), `ccf.json.gz` 6.8 MB, 21 MB in all, nothing rejected — after two lessons the real corpus taught: the X250 document of 2013 stamps its reserved block `MY12`, so a document's marker is the one most qualifiers name and an address keeps its own; the X351 document of 2016 writes one span backwards, so that parameter stays unrecorded and the other 1,263 stand. The owner's copy was re-stamped. The run reads the keeper first and then each copy, slices every block at its offset, decodes each parameter as the type SDD declares — an option's text, a number, a string, digits, bytes — and never judges it; a copy's difference is two readings side by side. The interface has a panel under the passport's with SDD's groups as headings, the rows SDD hides behind one switch, and a button beside the passport's. Checked by five ingest golden tests, the decoder's tests, the bench's, the intake's (a configuration value never reaches a manifest), the shell's end-to-end bench run and 3 interface tests (88 in all). No build: the owner batches the work. Not decided: describing the vehicle from its configuration (SDD's `qualifier_map`, the "digital car"), the VDF read, the product's own words for the texts. Earlier the same day — **the module passport (`ADR-0027`, F19; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car).** The first row the owner took from the capability matrix to finish stage 1: what a module says it is — the part numbers fitted, the serial, the software and hardware levels — read as one operation, `MODULE_PASSPORT`, class `READ_ONLY`. The platform documents were read before the ADR was written: each names, per module, three identifier sets (`NET`, `SWDL`, `PDI`), and every module's own set opens with the same eight *Core PIDs* — `0xF111` core assembly, `0xF112` assembly, `0xF113` delivery assembly, `0xF18C` serial, `0xF188` software, `0xF191` hardware, `0xF190` VIN, `0xF103` active network configuration. The ingest records those members in the DID catalogue's own shape with the encoding `text=ascii`, so a passport read is a module read through the same prepared transaction, the same record and the same intake; a value is the text it is, padding trimmed, or the bytes with the reason; nothing is parsed out of a part number and nothing compared with a catalogue — whether a software level is current is a separate decision, and the report fails a test if it ever says *outdated*. Real source: 34,788 identification records over 45 identifiers, 99 module families, 21 programmes; the library re-exported to 314,462 records (from 279,674), nothing rejected, and the owner's copy re-stamped the same day. The bench answers a text identifier with a part-number-shaped string under a `BN` prefix no real part carries, and the VIN identifier with its VIN. The interface has a panel under the mileage one — one table per module, our label with SDD's name beneath, the address, the text as held — and a button beside *Read the mileage*; labels for 38 identifiers in three languages, keyed by the identifier. Checked by the platform golden test, the decoder's text test, the bench's, the survey and intake tests (22 records, 3 confirmations), the shell's end-to-end bench run and 3 interface tests (85 in all). No build: the owner batches the work. Earlier the same day — **ADR-0020 to ADR-0026 read with a fresh eye, and what the reading found is fixed (`IMPLEMENTED / FIXTURE_TESTED`). The owner asked for the review; every ADR was read beside the code it describes, and each claim checked against the machine rather than the text. Held up: the stamp signs the bundle hashes and not only the fields; the cadence is measured on hardware and recorded; both guides say the car must stand; the bench/real mode is refused at the connection; the refusal to write on the bench lives in the command; `dtc_codes_for` is ordered; the bench transport exposes nothing that transmits. Found and fixed the same day: the mileage survey subtracted readings counted in different units — the reference is now the highest reading among those sharing a unit, and a row in another unit says so instead of carrying a number; its report left every row marked real on the bench while the screen said synthetic — the report marks them, and the bench test now fails if a bench row ever reads `SOURCE_BACKED`; `asked` counted reads while `answered` counted parameters — both count reads; a survey without an adapter said the data named no mileage — it says the adapter is missing; a survey that lost its adapter kept stepping — it finishes, as the live read does; the standard-OBD panel kept its own copy of the help view, English only, under a caption that had been untrue since `ADR-0026` — one `DtcHelp` serves both panels; the transmit-API rule scanned five named files and a new one had fallen outside it — it scans every file in `mongoose-jlr` and `bench-vehicle`. **And the help layer's gap is understood and half closed** (`ADR-0026`, amendment): the ingest now writes a screen's items by name beside the text, the table carries both keys — 26,893 rows, name and fingerprint, 13.0 MB — the library was re-exported (279,674 records) and the owner's copy re-stamped as `656D-B4C9`. Measured on that library: 88.9 % of items find a row by name, 0.35 % by fingerprint, 10.8 % stay English — and the second cause is the assistant's again: the file the owner translated was built from a half copy of the help pack, 26,893 named texts of the 42,636 the full pack holds, so 15,743 were never offered. They are listed by name in the order of their use and are the first input to the translation pipeline. The synthetic fixture had borrowed two real mnemonic names and the name-keyed lookup found real wording under them; the fixture's names are `SYNTH_*` now, and the lesson is in the ADR. **Closed later the same day, as their own commit:** a live read and a mileage survey now leave, per request, the same record a single module read leaves — `read_record` is the one builder all three share — and the intake reads them under the name of the place they sat (`live_read_runs[0].set[0]`, `mileage_surveys[0].reads[0]`), through the builder a module read goes through, so a tester's live session confirms what a single read confirms and no more; `ADR-0022` §5 is true now. The bench's identifier values walk with the request count — the last byte, a triangle of amplitude 8 over 64 requests, deterministic — so smallest, largest and trend have something to show on the bench. Still open: `standard_obd_reads` are not read by the intake; the legislated path talks to an address and not a module, and needs a builder of its own. Checked by the three help suites (16 tests), the shell's end-to-end bench run and 82 interface tests. No build: the owner batches the work. Earlier — the numbers say their names in the user's language (`ADR-0025`, F17; `IMPLEMENTED / FIXTURE_TESTED`). Every reading arrived named in SDD's English in all three interfaces: 15,281 parameter definitions under 4,977 distinct names. Two translations turned out to exist, so the choice was measured rather than assumed. SDD publishes its whole catalogue in twelve languages — the English and Russian components carry the same 15,269 keys, not one left in English, no name with two renderings — and no Ukrainian, ever. The owner translated all 4,977 names himself into Russian and Ukrainian, and his file was taken over JLR's on the numbers: exact coverage including SDD's double spaces, `bank` rendered one way in 254 names against JLR's 249 `блок` and 5 `ряд`, a mean name of 69.8 characters against 78.3 for a column that shares its row with a value, and `Total distance` reading `Общий пробег` where JLR reads `Общее расстояние` — wrong for an odometer, and the very parameter F16 asks every module for. It is a dictionary and not knowledge: no evidence, no applicability, no validation state, no new source type, never in the knowledge store, and it can never move what the product believes about a car. It lives in the repository beside `dtc_standard_text.tsv` because it is the owner's work and not JLR's text. The English stays the identity and every report keeps it, because evidence does not change language; a name with no row shows its English, one name at a time. **The fall-back is counted, not hoped for:** `cargo run -p diagnostic-session --example untranslated_names` surveys every declared breakpoint of every programme in the issued library — 54 surveys — and of the 1,401 distinct names those surveys can put on screen, **0** lack a row. Checked by 5 tests in the crate, one of which fails if the English key is ever reflowed, and 5 interface tests including the language switched faster than the table arrives. No build: the owner batches the work. Earlier the same day — the odometer is read from every module (`ADR-0024`, F16; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car). A car keeps its mileage in dozens of places and a rolled-back one is rewritten only where the tool could reach, so the survey asks every module the loaded data names a distance for, once each. The catalogue was read before the ADR was written: SDD declares `0xDD01` **Total distance** for 92 module families over 21 programmes, the cluster's `0x61BB` beside it, and odometer stamps inside event histories — a gearbox stall, a failed gear selection, a delayed park engagement, the Jaguar flight recorder. The rule that tells a mileage from a counter lives in the knowledge layer, beside the catalogue that names the parameters: `Total distance` and `Odometer store` are running totals, anything else naming an odometer is a stamp from a moment in the car's life, and `Distance travelled since the malfunction indicator lamp was activated` is excluded by name because it resets with the codes. The shell plans one read per module and identifier, steps through them with the interface's timer, and does the subtraction: every row carries its difference from the highest running total on the car — the highest, not the cluster's, because the cluster is what gets rewritten. Event stamps sit in their own table, where a positive difference is an event the car has not driven to. No verdict is printed anywhere, in the interface or the report: a module that says nothing says nothing rather than zero, the honest causes of disagreement are stated once under the table, and a test fails the build if the word for the dishonest one ever appears. The whole survey joins the session bundle as `mileage_surveys`; on the bench every reading is `SYNTHETIC`. The button sits with the whole-car actions, beside the check of every module. Checked by the knowledge rule's own test, the shell's end-to-end bench run and 3 interface tests. No build: the owner batches the work. Earlier — build 0.9.8 is the current one, and it carries the whole day: commit 94afde2, tag v0.9.8, run 34479604342, all four jobs green. Windows `ProwlOne_0.9.8_x64-setup.exe`, 4.7 MB, SHA-256 `d40324436bd53d133d8d38e85447a5b57bf3c3f8f85c1e885390cef443bc5183`; macOS `ProwlOne_0.9.8_universal.dmg`, 12.9 MB, SHA-256 `a9d149ac08f54acbaa29fa3ed2ad6d5d058b7adfbb12568a3f99d961dccf93e0`; both in `Desktop/prowlone-build-0.9.8/`. What is new in it, in the order it was built: the legislated OBD-II services end to end, live reading with its measured cadence, the fault-code help SDD carries, and a library that travels compressed. The library **did** change, so every issued copy must be made again: the owner's own was re-issued the same day from the compressed export — 6,515 manifests, 242,691 records, 11 MB on disk, zip 10.2 MB, code C3E1-3BB7, valid to 2027-09-10, and the stamping tool read it back and accepted it under the issuing key. Earlier the same day — the issued library travels compressed (`ADR-0023`), and it is a twentieth of what it was. The help layer of the same day took the export from 131,501 records to 242,691 and the directory from 170 MB to 311 MB; the manifests are JSON, which is repetitive by nature, so a manifest may now be stored as `<name>.json.gz` and the loader reads packed and plain alike. Exported afresh from the real corpus: 6,515 manifests, 242,691 records, nothing rejected, **11 MB** on disk against 311 MB unpacked — `dtc_help.json` alone goes from 275.7 MB to 9.97 MB. The stamp of `ADR-0019` covers what a manifest says, not how it is packed: the hash is of the JSON, so a library stamps identically either way, while the file name in the stamp keeps the set of files exact and a Mac's AppleDouble sidecars stay ignored. The packed export was read back by the application's own loader — `Loaded`, 6,515 manifests, 242,691 records — and the help layer was measured on it: every one of the 74,218 help selections lands on exactly one screen for a described car, so the rule that refuses to guess between screens costs nothing in practice. Libraries issued before today keep loading. The issued copy has to be made again for a tester to get any of this. No build: the owner batches the work. Earlier the same day — the fault-code help SDD carries is read, and shown under the code it belongs to (`IMPLEMENTED / FIXTURE_TESTED`). The corpus was surveyed first: of 6,168 per-code help documents, 4,206 (68 %) carry real text and 1,962 only the `J_H_NO_HELP_AVAIL` placeholder; the text is structured by SDD's own headings — «Actions required:» 3,270 times, «Possible causes» 3,267, «Monitoring conditions:» 456 — and 1,613 codes (26 %) carry a number with a unit or a comparison, which is the manufacturer's own threshold for setting the code, not a norm derived from anything of ours. 223 codes name a datalogger signal with its identifier in the text, and 121 of those 157 identifiers are ones our own catalogue already carries. Every one of the 6,168 files is valid UTF-8 with no replacement character anywhere: the mojibake reported earlier in the day was this terminal's, not the data's, and the ingest reads what the corpus holds — the degree sign, the en dash, the micro sign and ten other characters — unchanged. What is recorded is SDD's chain as it stands: which screen a car is given (`sdd_help`, qualified by module, model, model-year designation and fault type) and, separately, what each screen says (`sdd_help_screen.<name>`), so a screen dozens of cars select is written once rather than dozens of times — the difference between about 45 MB and 44 MB of duplicated text. At run time `describe_dtc_with_help` chooses the screen for the car: the fault type's own screen first, one naming no fault type as a stand-in, and when the data offers several and the vehicle is not described closely enough to choose between them, none is shown and the reason is said — the neighbouring model year's causes would send someone to the wrong part. The interface shows it folded under the code, in both the module read and the standard OBD-II panel. Checked by 7 ingest tests and 3 runtime tests over a synthetic fixture that reproduces the shape, including an empty screen, an undefined mnemonic and the characters that must survive. The issued library has to be exported again for a tester to see any of it. No build: the owner batches the work. Earlier the same day — live reading is in the product (`ADR-0022`, decision 9(c); `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car). A `LiveReadService` in the shell holds a set of at most sixteen module-and-identifier pairs, each prepared by the same resolver a single read uses — an identifier the loaded library does not list for that module never joins the set, with the resolver's own reason — and reads them in turn, round after round, in the default session with no `0x10` and no `0x3E`, one request in flight, the adapter taken for that one request and released. The service owns the rules and the interface owns only the timer: a step that arrives sooner than the 100 ms floor is refused, so no interface can make the product ask a bus faster than the ADR permits; the ten-minute cap, an adapter that goes away, and an entry that fails three rounds running — silence or a refusal — end the run or drop the entry with its reason, and a run whose set empties stops itself. Every sample is kept with the milliseconds since the run began, the module, the identifier, the raw bytes and the catalogue's own decode, and the whole run joins the session bundle once as `live_read_runs`; on the bench every sample is `SYNTHETIC`. The interface has a panel in the module section: the set chosen by ticking identifiers the survey lists, a plain table — value with the catalogue's unit or its named state, the smallest and largest of the run, how many readings — the achieved round time beside it, and the reasons entries were dropped. Two departures from what the cadence measurement suggested are recorded in the ADR's `Built` section: the route is opened and closed per request, because the device refuses a second open while one is open and refuses to close the transport with a route open, so a route held across the released lock of decision 4 would break every other command and disconnect with it; and the loop's timer is the interface's, its rules the shell's. No gauges: decision 6 waits for a car. Checked by the shell's end-to-end bench run — a set of two, one of which the library cannot plan, the floor refusing a step that comes too soon, the report handed over once — and 5 interface tests. No build: the owner batches the work. Earlier the same day — the legislated OBD-II services are in the product, end to end (`ADR-0022`, decision 7; `IMPLEMENTED / FIXTURE_TESTED`, nothing has met a car). `obd-j1979` now carries the read-only half of SAE J1979 whole: current data and freeze frames (modes 01, 02) over a table of the PIDs with the standard's own formulas — seventy-odd scaled, the rest named and shown raw and labelled raw; stored, pending and permanent codes (03, 07, 0A) in the J2012 text our own Ukrainian and Russian wording joins to; on-board monitoring results (06) with the unit-and-scaling table, an unknown id staying raw; vehicle information (09): VIN, calibrations, verification numbers, in-use performance counters by name, ECU name and serial. Clearing codes (04) and control (08) do not exist here. A read is prepared from the standard alone — ISO 15765-4's eight responders, 0x7E0–0x7E7 asking and 0x7E8–0x7EF answering, 0x7DF for all, 500 kbit/s on pins 6 and 14 — with the standard as the evidence of every field and no resolver in the way; the adapter executes it on the mirror of the UDS live path; the bench's engine controller answers every service with standing-engine values, its scenario's codes behind the lamp, the session's VIN; the shell records each read in the session bundle as `standard_obd_reads`; and the interface has a panel in the listen section — read everything supported (the support maps walked, then six PIDs a request), the three code lists with our wording, the freeze frame walked PID by PID with the code that froze it first, the monitors walked six MIDs a request, the vehicle information walked InfoType by InfoType (VIN, calibrations, verification numbers, in-use counters, ECU name and serial), a responder chooser for the eight; the standard's parameter names, states and refusals worded in Ukrainian and Russian where we carry them, the standard's English otherwise; the session report counts these reads. Checked by 24 codec tests, 5 execution, 5 device through the real framing, 3 bench, the shell's end-to-end bench run, and 6 interface tests. No build: the owner batches the work. Earlier the same day — the cadence of a read is measured, both halves, as `ADR-0022` decision 3 asked before any gauge is drawn (`docs/evidence/mongoose-link-cadence-2026-09-10.md`). Hardware, adapter plugged in and no vehicle: the link to the MongoosePro on COM3 answers a board-info round trip in a median 15.6 ms (p95 22.9 ms, n=306), and an open-pins-close cycle on resource 5, listen-only, in a median 50 ms. Software, the bench answering: the product's own read path — prepared once, executed over the adapter framing through the bench transport, ISO-TP, UDS, decoded and marked — carries 155,867 reads a second over ten modules of the owner's X250 MY10 V8SC on the real library of 131,501 records, 0.01 ms a read (the SYNTHA fixtures: 250,788 a second). So the software is not a term in the cadence at all; the link is, and the module's own answer time is, which only a car can show. Expected on a car with the route kept open for the run: 30–70 ms a read, 15–30 reads a second; with the route opened and closed around each read as a single read does today, about 50 ms more, 8–12 a second. Two consequences for the service: it opens the route once for the run and closes it on stop; and the ADR's floor of 100 ms between requests binds before the hardware does — a courtesy to the bus, to be revisited once a car has been seen, not before. The measurement is `cadence_on_the_bench` in `bench_e2e.rs`, ignored by default and run on purpose; `scripts/mongoose-probe/mongoose_link_cadence.py` is the hardware half. No build: the owner batches the work. Earlier the same day — `ADR-0022` decides live reading: the same read-only reads, repeated at a stated cadence, recorded as evidence of what one read is evidence of, in the default session with no session control and no tester-present, with a floor, a cap and a stop; the library was read first and serves four of the owner's six visualisation blocks per module (15,281 identifier parameters; nothing for restraints or a traction battery), the cadence is unknown until measured on the bench, and no read has yet come from a car — so the order is ADR, cadence, service with a plain table, then the visual layer, with the first tester report outranking the last two. No code yet. The owner names it the start of the 1.1 line. Earlier the same day — build 0.9.7 is the current one, and the first that a person can sit in front of on a slow machine: commit d235cfd, tag v0.9.7, run 34446073454, all four jobs green. macOS ProwlOne_0.9.7_universal.dmg, 12.4 MB, SHA-256 6c0707f802606092748e668d4cb67dc3f3d78be272ce5e1955986e2a269ce181; Windows ProwlOne_0.9.7_x64-setup.exe, 4.6 MB, SHA-256 91a826df781abb2b1ed2960006011c60b2819cc0358594819d830d7534af4a49; both in Desktop/prowlone-build-0.9.7/. The library did not change, so every issued copy stays valid and nothing is re-issued for this build.

What the day found, in the order it found it. **The stamp refused a copy that had lost nothing.** The owner carried his own `BE5E-E564` to the Mac and the application said the data did not match its stamp. It was not the data: macOS writes an AppleDouble sidecar beside every file — `._platform.json` next to `platform.json` — whenever a folder crosses a stick or a share, invisible in Finder and ending in `.json`, and the stamp check counts the files it covers. Reproduced here by adding two empty sidecars to the owner's own copy, which was enough. The loader now skips any name beginning with a dot; the stamp still demands the exact set of real manifests, so nothing can be slipped into an issued copy. Both tester guides carry the workaround for 0.9.6 and earlier.

**The window froze, and it was every command.** Tauri runs a synchronous command on the thread that owns the window, and all twenty-four were synchronous. The owner's 2016 Mac read a 177 MB library with macOS drawing its spinning wheel, and he took the application for hung — reasonably. Measured here: about 8 seconds on the Windows desktop, so three or four times that on his machine. Every command is async now, not only the slow ones: the state a screen polls sits behind the same mutex a long command holds, so a synchronous getter would have waited there on the window's thread and frozen it exactly as before. The case that mattered most was never the library: a bus listen holds the adapter for up to fifteen seconds, in a car, with a tester watching.

**Nothing said what was happening.** While the library reads, the panel now says so and counts the seconds, with a line that tens of seconds are normal on an old machine.

**The folder is remembered between launches**, which the owner asked for: the path only, never the data. The stamp is verified on every start, so a copy still stops working on its own date, and no second copy of licensed data is written anywhere.

Earlier the same day — the macOS build has been opened by a person, for the first time: 0.9.6 on the owner's MacBook Pro of 2016 with the Touch Bar, an Intel machine, so it is the x86_64 half of the universal bundle that ran, natively. It looks and behaves as it does on Windows 11, a little slower, which is what a ten-year-old machine and a WebKit window give. Gatekeeper refused it first with "not downloaded from the App Store" — the App Store-only policy, which offers no "Open Anyway" at all and which neither guide covered; the owner allowed identified developers in Privacy & Security and it opened. Both tester guides now name that case. Nothing has been read on the Mac yet: no library copy is on it, so the bench and the survey are untried there. Earlier the same day — build 0.9.6 is the current one, the first with the bench scenario: commit 4a840c5, tag v0.9.6, run 34350488233, all four jobs green. macOS ProwlOne_0.9.6_universal.dmg, 12.3 MB, SHA-256 16374eb67cadc1aec98e5b465e75c6e4f8a6e96c3cfbaee83af45d8fbcfec5c1; Windows ProwlOne_0.9.6_x64-setup.exe, 4.6 MB, SHA-256 0d044760fa29791ac808c16976b47dd281b40f981c9acfa415ce3b44f1a86b1a; both in Downloads/prowlone-build-0.9.6/. The library did not change, so every issued copy stays valid — the owner's is `BE5E-E564` — and nothing has to be re-issued for this build. Two runs before it were red on `cargo fmt --check`, a step that lived in the CI job and in none of the local checks, which were whatever anyone remembered to type; the tag was moved off the commit that could not build. `scripts/preflight.ps1` is the answer to that: one command that runs everything CI runs — architecture, frontend lint, tests and build, then Rust fmt, clippy and tests for the portable crates and for the shell, both in Docker — and the `pnpm` filters now name a directory instead of a package, so a rename cannot switch the interface checks off in silence again. Earlier the same day — the bench draws its fault codes from the library, by a scenario number the tester chooses (`ADR-0020` amendment). Until now every module reported one or two codes from a fixed list of eight written into the crate: one picture, every time. Now `0` is the healthy vehicle — every module answers, none reports a code, a case the product could not show before — and any other number gives each module none, one or two codes drawn from those the library describes for that module's own family, so every code shown has wording. The seed is the number mixed with the family name: one number always paints one picture, on any machine, from the same library, and the number is in the BENCH band so a tester can quote it. `KnowledgeLibrary::dtc_codes_for` is the new seam. Found while testing it: `pnpm lint`, `pnpm test` and `pnpm build` had filtered the frontend by `@jlr-scanner/frontend` since the rename on 2026-09-09, matched no project, and passed without running anything — in CI too, where the frontend job was green on nothing. Fixed; the 54 interface tests and the lint run again. Earlier the same day — the signature is set aside, the free route included. SignPath Foundation signs open-source projects for nothing, but forbids any proprietary component published by the maintainer, which is precisely what an issued library is; and Microsoft's own comparison of signing options puts an OV certificate, Azure Artifact Signing and EV in one row — reputation accumulates over downloads, first prompts expected, EV's instant bypass gone since 2024 — so for five to ten testers no certificate silences anything. Nothing is applied for, nothing is bought, and `F3_1_WINDOWS_SIGNING.md` holds the three conditions that would reopen it. Whether installers are ever published publicly stays a separate, unanswered question. Same day — build 0.9.5 is the one the club testers receive, and the first to carry the whole day's work: the bench (`ADR-0020`), the product's own name and mark (`ADR-0021`), every one of the library's 1,664 standard fault codes read in Ukrainian and Russian, a module's qualifier reaching the record so its addresses stop colliding, and the engine-variant field that lets a split engine say which half it is. Commit cdf64c1, tag v0.9.5, run 34340565492, all four jobs green. macOS ProwlOne_0.9.5_universal.dmg, 12.9 MB, SHA-256 603bb3a7d665eaa0dbf60a2f45645ec43c11ad30beb81f425ad5687abfad38aa; Windows ProwlOne_0.9.5_x64-setup.exe, 4.8 MB, SHA-256 a743dbd98ec8bc57413a61a7e833d58e2564e78256c6afeea7d515a9cd64fbba; both in Downloads/prowlone-build-0.9.5/. It needs the library re-issued for the qualifier fix: the owner's copy is `BE5E-E564`, and the copy issued to the first tester on 2026-09-06 predates it — the owner decided on 2026-09-09 to leave that tester's copy alone for now. Neither file has been opened by a person yet. Earlier the same day — the qualifier defect is fixed and the library re-exported. The platform adapter now reads a module's own `<qualifier>`: `type` narrows the powertrain, `SubType` the variant, a test naming a market the market, and anything else keeps SDD's own name under `other`; the record id gains a short digest of those tests so two rows for one acronym stop colliding, while an unqualified module keeps the id it always had. The library went from 130,388 records to 131,501, nothing rejected, and the independent cross-check now reports **742 module rows agreeing and nothing disagreeing, in either direction**, with all 3,799 field widths still clean. The owner's copy was re-issued from it: code `BE5E-E564`, valid to 2027-09-09; the previous library is kept as `Downloads/prowlone-library-2026-09-07-before-qualifier-fix`, and the copy issued to the first tester on 2026-09-06 predates the fix and should be replaced before that tester meets a car. What the fix changes for a driver: an X250 of 2010 with the V6 now resolves its instrument cluster at `0x7B2/0x7BA` and its parking-brake module at `0x7B4/0x7BC`, where before both went to the 5.0's addresses and would have looked silent; the survey with the engine stated returns 28 modules reachable on the V6 and 25 with no engine stated, because a module whose address depends on the engine is now refused rather than guessed. That last gap was closed the same day: the catalogue now carries the variants a programme's data declares — for the X250 that is 2L, 4.2L and 5L — and the vehicle card offers them in an **Engine variant** field, shown only for a programme whose data splits an engine at all. With `V8NA` and `4.2L` stated, the X250 of 2010 resolves 28 modules, its cluster at `0x7B2/0x7BA` and its parking brake at `0x7B4/0x7BC`. Earlier the same day — the SDD XML was read a second time, independently, and compared with the exported library (`docs/evidence/xml-crosscheck-2026-09-09.md`). Field widths are clean: all 3,799 byte ranges and masks the library carries are ones the XML declares, so the question of whether we misread a width is closed. Addresses found one real defect of ours: where a platform document declares the same module several times under a `<qualifier>` — an engine type, a build year, a market — with different diagnostic addresses, the ingest writes one record per document and module and only the last address survives. Nine module rows in the corpus do this. It costs the X250 of 2010 with the V6 petrol or the 4.2 V8 the correct instrument-cluster and parking-brake addresses, and L405 and L494 of MY14 built in 2014 the correct ABS address; on the owner's own 5.0 supercharged X250 the surviving address happens to be the right one. The fix is not architectural — `applicability` already carries `powertrain`, `market` and `other`, and the record id needs the qualifier so two rows stop colliding — but it changes the exported library and therefore the issued copies, so it is the owner's call whether it goes into the next build. The check also named a coverage gap that is not a misreading: five platform documents declare programmes (`X101`, `X103`, `X206`, `X356`, `X358`) the ingest does not carry, 51 module rows. Earlier the same day — fault codes read in the interface's language. SDD's text database carries module names and failure-type wording in twelve languages, but a fault code's own description exists in English only and Ukrainian is not among SDD's languages at all, so the wording a Ukrainian or Russian speaker reads has to be ours. It now is, for the codes the standard defines: `crates/diagnostic-session/data/dtc_standard_text.tsv` holds code, Ukrainian, Russian — all 1,664 of them, in nine passes, written from what the standard says each code means, carrying no manufacturer phrasing and therefore safe in a public repository. `dtc_text::standard_texts` is joined into `DtcDescription`, travels to the interface as `DtcSummary::description_texts`, and the read panel shows our wording with the loaded data's English kept underneath it. The report is untouched and stays English, because a report is evidence and evidence does not change language. A manufacturer-specific code gets nothing from us and falls back to the library's English, which is where that wording belongs. Checked in the preview in all three languages, and by 50 interface tests and 58 Rust suites. Not in a build yet: the owner asked for changes to be batched, so this waits for the next one. What is left is not the standard codes — 1,664 are in the library, and every one of them is now covered — and, separately, whether the manufacturer-specific wording is ever translated inside an issued library. Earlier — build 0.9.4 opened on the owner's Windows 11 desktop: from the desktop shortcut, by double-click and by right-click. It is the first ProwlOne build a person has run. What it took, in order: the owner switched Smart App Control off (the registry value still read 1 afterwards; no block was logged for the launch); a silent NSIS install (`/S`) put 0.9.4 in place, because the interactive installer insists on running the previous unsigned uninstaller first and Smart App Control blocks that; both stale installs had been removed by hand, folder, shortcuts, the `Uninstall` entry and the NSIS key `HKCU\Software\prowlone`, which alone makes the installer say "Already Installed"; and the shortcut showed Windows's placeholder until the icon cache was refreshed, while the executable carried the right icon all along, checked by extracting it. The steps are in `OWNER_GUIDE.uk.md` section 9. The day also showed that a new unsigned build restarts the block every time, so builds are to be batched, not made per change. Earlier — Smart App Control blocks the new unsigned build on the owner's Windows 11 desktop (25H2, build 26200.9278): `Microsoft-Windows-CodeIntegrity/Operational` event 3077 against `prowlone-shell.exe` at 09:25, 09:34 and 09:37 and against `uninstall.exe` at 09:38, policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, `VerifiedAndReputablePolicyState = 1`. The first reading of this — that Smart App Control had just switched from evaluation to enforcement — was wrong, and the log says so: it holds 110 block events from 2026-08-29 onwards and exactly one audit event, so enforcement is not new. What the same log shows is stranger and is the fact worth keeping: the previous binary `jlr-scanner-shell.exe` was blocked on 29 and 30 August, on 1 September and on 6 September, and then ran on this machine unblocked, including this morning, while the newly built `prowlone-shell.exe` is blocked from its first launch. The pattern is per file, and a block seems to lapse for a file that has been on the machine a while; the mechanism behind that is not established here and should not be guessed at. The application cannot start there and cannot even be uninstalled by its own uninstaller, because that is unsigned too; both stale installs (JLR Scanner 0.9.2, ProwlOne 0.9.3) were removed by hand — folders under `%LOCALAPPDATA%`, the two Start-menu and desktop shortcuts, and the two `HKCU` uninstall entries. Nothing else on the machine was touched, and Smart App Control was left on. This is not a defect of the build: the same wall waits for any tester whose Windows 11 has it on, and it is the first hard evidence for the `F3.1` signing decision, which until now was deferred until vehicle results. The owner's Windows 10 laptop and the macOS build remain open paths. Build 0.9.4 is the current one — the product's own name and the mark the owner chose (`ADR-0021`): commit 3236785, tag v0.9.4, run 34319517717, all four jobs green. macOS ProwlOne_0.9.4_universal.dmg, 12.7 MB, SHA-256 199662eaa3e40f77e49bc485cccceadd72d39251e9c08992d7a48f41883a9b34; Windows ProwlOne_0.9.4_x64-setup.exe, 4.7 MB, SHA-256 4179d9918330791b928561aa89b70079888325659805bf7058e01c5be16a6f7e; both in Downloads/prowlone-build-0.9.4/. Build 0.9.3 was superseded before it reached anyone, so its number is spent and its files should be discarded: it carried the illustrated icon the owner judged cheap at desktop size. Earlier the same day — 0.9.3 was the first under the product's own name and mark, ProwlOne (`ADR-0021`): commit b13086d, tag v0.9.3, run 34317723468, all four jobs green, so the renamed crate, the renamed macOS binary inside the bundle and the new icon set all survive a real build. macOS ProwlOne_0.9.3_universal.dmg, 14.6 MB, SHA-256 44ba66e56233049bd3b4a873690ce233283762d5223b0e1c22faf6ec8215217f; Windows ProwlOne_0.9.3_x64-setup.exe, 4.9 MB, SHA-256 8949bec5e97eff0e4f5306024f805761c6206fc308a62ce349c258dd00dd4ae5; both in Downloads/prowlone-build-0.9.3/. This is the build the club testers receive; the bundle identifier changed with the name, so it installs beside an older build rather than over it, and the older one should be uninstalled. Neither file has yet been opened by a person. Earlier the same day — build 0.9.2 was the bench build, the one the club testers receive: commit 89a3725, tag v0.9.2, run 34313556981, all four jobs green; the footer reads 0.9.2 · 89a3725. macOS JLR Scanner_0.9.2_universal.dmg, 12.5 MB, SHA-256 d5a272d23f4e14c2d00d3b79213a9c4f6f076aa4b581e01b39621f58a9eb8e98; Windows JLR Scanner_0.9.2_x64-setup.exe, 4.7 MB, SHA-256 de8e67952c677434ad6717667be71f15f9c973f81ea482e034f5b746c594e816; both in Downloads/jlr-scanner-build-0.9.2/. Neither has yet been run by a person; the bench commit before it (8fad2f5) was green on the same four jobs, including the shell's end-to-end bench test under Windows. Same day, earlier — the bench is built (ADR-0020, `IMPLEMENTED / FIXTURE_TESTED`): a virtual vehicle answering on the adapter's own protocol — `mongoose_jlr::bench` speaks the MongoosePro framing below the whole real stack, `bench-vehicle` answers from the loaded library for whatever vehicle the session surveyed, the adapter panel offers «connect the bench» beside «detect adapter», a session is bench or real and never both, nothing is written to disk in bench mode (the report is shown on screen), every bench value is marked `SYNTHETIC` and `report-intake` refuses a bench bundle anyway, and the whole screen turns to a warm Namib Orange tint with a BENCH band top and bottom while it is connected; the shell's end-to-end test drives connect, survey, reads, listen, bundle and refusals on every commit. It is part of the tester programme (recruitment, onboarding, regression) and never reaches VEHICLE_CONFIRMED. Earlier the same day it had only been decided. 2026-09-08 — builds are numbered from here: version 0.9.N, where N is the number testers quote and grows only when a build is handed out; the footer shows version and commit (0.9.1 · abc1234) and every session report carries application_version and application_build; scripts/bump-version.mjs sets the number in the five files that hold it and prints the commit and tag to make. Retroactively, 0.9.0 is the 417d9d1 build the first tester holds; 0.9.1 is the first numbered build: commit a5e86c8, tag v0.9.1, run 34196375573, all four jobs green; the footer reads 0.9.1 · a5e86c8. macOS JLR Scanner_0.9.1_universal.dmg, 13.2 MB, SHA-256 f84936a3824685cee3be0fa82624614cc0cb9c363a6a047fe9c7b321d6bece86; Windows JLR Scanner_0.9.1_x64-setup.exe, 4.7 MB, SHA-256 61481f75e38bf5b7dcad47014102a555783e9e2b9bcfb61bb1a64439ce333094; both in Downloads/jlr-scanner-build-0.9.1/. This is the build the club testers receive with their signed library copies. 2026-09-08 — bench: the diagnostic open of resource 21 (pins 3/11) and the application's outbound record are accepted by the firmware exactly as on resource 5, so the medium-speed read path is `HARDWARE_CONFIRMED` on the bench to the same extent as the high-speed one; the jump from the bootloader measured at 241 ms; a record whose header word after the sequence is 0 draws silence, the application's 1 draws the answer (evidence Part 4). 2026-09-07 — medium-speed diagnostics analysed offline, no code changed (`F10`, dated section): on the SDD-era cars `CAN_MS` is bound from SDD's own platform documents and ready — X250 MY10 carries 14 medium-speed modules with request and response identifiers and reachable fault-code reads, L319 MY10 14, L322 MY07 7 — while on the 2014-and-later cars the medium-speed half is still the `ADR-0015` hypothesis, 25 of L405 MY14's modules and 22 of L494 MY16's. What was called the weak point was reworded on 2026-09-09 at the owner's insistence, and he was right: for the Ford-derived SDD-era cars the binding of `CAN_MS` to pins 3/11 at 125 kbit/s is known from the architecture, from the adapter's own JLR pinout, and from what third-party tooling does on these cars every day, and the adapter path is `HARDWARE_CONFIRMED` on the bench; what is missing is a capture from a car, which is a signature rather than a doubt, and it is now the first step of the tester guide. The genuinely open question is the 2014-and-later gateway. The gatewayed sub-networks `SUB_MOST`, `SUB_CAN1`, `NGI` and L322's `DS2` are a separate, stage-2 problem and should not be counted as medium-speed. Also 2026-09-07: a survey of X350 returns nothing because ten legacy Jaguar platform documents — X100, X202, X350 and X400, 216 modules — sit in the SDD component `CURRENT_JLR_MCP_XML_XML`, which no export has ever walked (`F9`, dated section); **they were ingested the same day**: the exporter now walks that component too, `parse_marker` accepts SDD's dotted mid-year fraction (`MY02.5`) beside the underscored one, and the library grew from 45 platform manifests to 55, 18 programmes to 22, 129,441 records to 130,388, nothing rejected and no failed manifest. `PLATFORM_X400.xml` and `PLATFORM_X404.xml` were not the collision they looked like: X404 is the fuller revision of the same programme-year and the two merge into the union of 19 module families. Ingesting adds knowledge rather than reach, because those cars are pre-UDS: of the four new programmes only the XJ of 2006 and 2008 gains anything usable today, 5 and 6 modules on `CAN_HS` including `PCM` at `0x7E0/0x7E8`. The library folder was replaced and the previous one kept as `Downloads/prowlone-library-2026-09-06-xcl-only`; the owner's own copy was re-issued from the new base (code `0EDF-FCBF`, valid to 2027-09-07), while the copy issued to the first tester on 2026-09-06 (`A2CC-E236`) stays valid and simply lacks the four programmes. 2026-09-06 — published: this repository, github.com/Oleg-mk/jlr-scanner, is the public continuation under AGPL-3.0 of a private archive (jlr-scanner-archive, frozen at its commit c32f8d7) whose commit hashes the older entries below still cite. Same day, earlier — after the first live test failed at channel open, the live CAN path was rebuilt on the firmware's real handshake and resource routes (ADR-0018): jump from the bootloader, resource 5 for `hs-can`, resource 21 for `ms-can`, both listen sequences `HARDWARE_CONFIRMED` on the bench; the next CI build is the first that can meet a car. 2026-09-05 — reports are saved and the library folder is chosen through the shell's native dialogs (F11 slice 10); by owner decision the tester programme runs on the unsigned CI build, with `TESTER_GUIDE.md` as the hand-out. The branch has not been pushed, so no installer of this state exists yet; nothing here has met a Windows build or a vehicle. Also 2026-09-05: the medium-speed buses of the 2014-and-later cars are hypothesised through `ms-can` on pins 3/11, on the owner's community statement (ADR-0015 addendum) — 716 module rows now sit on hypothesised routes, 14 remain on unbound sub-networks.** M3 derived `normal_fixed` identifiers are `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_SURVEYED` (ADR-0017): 73 physically addressed modules of L322 MY04.5–07, L319 MY05 and L320 MY06 now carry derived 29-bit identifiers, `Unverified` until a tester's read confirms them; the live path sends 29-bit frames (transport-fake tested, not hardware-confirmed). F13 report intake (milestone M2) is `IMPLEMENTED / FIXTURE_TESTED`: a tester's session report becomes one `Captured` manifest whose records confirm, for that vehicle, what a module's answer proved — addressing, bus, physical route, protocol, capability — and record silence, captures and calibration reads as observations; loaded with the library, a confirmed route turns from hypothesis to `CAPTURE_VALIDATED` in the survey (ADR-0016). No report from a vehicle exists yet. F11 Tester Application (milestone M1) is `IMPLEMENTED / FIXTURE_TESTED`: the vehicle picker, VIN decoding with SDD's own tables, fault-code wording join, identifier value decoding with named states, module names from SDD's text database, guided session flow, single session report, the SDD-style vehicle network map with a read-only check of every module, and an interface in English, Russian and Ukrainian with SDD's data text following into Russian (the data-derived parts also `REAL_SOURCE_SURVEYED`); the map-converter scale stays open, and nothing has met a vehicle. F10 Multi-Module Read-Only Diagnostics is `IN_PROGRESS`; enumeration, family-level resolution, offline UDS read execution and the composition root are `IMPLEMENTED / FIXTURE_TESTED`, and the vehicle survey is `REAL_SOURCE_SURVEYED` against the exported SDD corpus (offline). F9 SDD Knowledge Ingestion has fully ingested the surveyed SDD corpus; its addressing, DID-catalogue, DTC, ODST, DTC-index, and platform slices are `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`. F8 Tester-Ready Mongoose Alpha Candidate is `TESTER_ALPHA_CANDIDATE_READY / IMPLEMENTED / SIMULATOR_TESTED / REPLAY_TESTED / TRANSPORT_FAKE_TESTED / NO_VEHICLE_INTERACTION`. Live vehicle status is `NOT_YET_EXTERNALLY_VALIDATED`. Distribution is `READY_FOR_SIGNING / DISTRIBUTION_BLOCKED_BY_F3.1`. F7, F6, F5, and F4 remain PASS. F3 is `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`; F3.1: gate `G2` decided 2026-09-03 (Ukraine, individual entrepreneur); the certificate is deferred until real vehicle results, so distribution stays `DISTRIBUTION_BLOCKED_BY_F3.1`; until then the unsigned CI build runs on the owner's own hardware only. Gate `G4` decided the same day: the SDD-derived library goes to named testers only, never inside the installer.**

Capability validation states are recorded independently. `IMPLEMENTED`, `FIXTURE_TESTED`, `HARDWARE_CONFIRMED`, and `VEHICLE_CONFIRMED` are not interchangeable.

## 2026-09-09 — the product is named ProwlOne

The owner chose the name and drew the mark. Nothing of Jaguar Land Rover's
is in either: the name is our own, the icon is a spotted big cat lying on a
generic boxy off-roader with a green pulse beside it, drawn for us. The old
name put a manufacturer's marks in the product's own identity, which is
exactly what a public launch cannot carry (`ADR-0021`).

What the name reaches: the product and window title, the bundle identifier
(`com.prowlone.desktop`), the shell crate and therefore the binary inside
the macOS bundle (`prowlone-shell`), the installer and disk-image names, the
saved report file names, the issuer written into new library stamps, the
signing description, the CI artifact names, and the icon set for Windows and
macOS. The interface's title line now reads ProwlOne with
"Multi-platform vehicle diagnostics" under it, translated; the marks are
named in prose, where the README's disclaimer already stands.

What the name deliberately does not reach, because it is data and not
branding: the session-report schema id `jlr-scanner.session-report`, which
the copies already in testers' hands write and the intake checks; the
recorded source ids of evidence such as
`jlr-scanner-mongoose-jlr-route-bindings`; the crates named after what they
hold (`jlr-profiles`, `mongoose-jlr`); and the dated history in these
documents, which names files that really were called that. Library copies
issued under the former name stay valid: a stamp is verified against the
issuer written inside its own signed file, and the check was run against the
owner's own copy after the rename.

## 2026-09-09 — the bench: a virtual vehicle behind a stand-in adapter

Decided in the morning (`ADR-0020`), built the same day, with the owner's
words as the brief: a stand for testing and for showing the diagnostic
programme, data that is illustrative and never evidence, a file that can
never leave, and a screen that cannot be mistaken for a car.

What exists:

- **`transport_api::BenchBus`** — frames in, frames out, per route; the
  protocol-neutral contract a bench vehicle implements. `EmptyBench`
  answers nothing.
- **`mongoose_jlr::bench::BenchTransport`** — a `ByteTransport` that
  answers the device code with the firmware's own records: board-info from
  the bootloader until `cJumpToFirmware`, resources 5 and 21 with their
  pins and the listen-only flag, close, and the application's outbound
  record answered with inbound frames from the bus. It knows no vehicle.
  Four tests, one of them a multi-frame UDS read through the real stack.
- **`bench-vehicle`** — the vehicle, built from the loaded library and the
  session's context: one responder per module the survey can reach, on its
  bus and identifiers, answering ReadDataByIdentifier for the identifiers
  the library marks readable (with the catalogue's widths), the VIN,
  ReadDTCInformation with fault codes the library describes, and J1979
  calibration identification; ISO-TP with flow control. Five tests on the
  synthetic SYNTHA library.
- **The shell** — `connect_bench` connects it with no port and no
  discovery; the vehicle on the bench follows every survey; capture, module
  read and calibration read mark what came from it (`Synthetic` fixture
  named `bench-…`, `route_validation: SYNTHETIC`, execution source
  `BENCH_SYNTHETIC`); the session bundle carries `session_mode`; a session
  holding records of one kind refuses the adapter of the other kind
  (`SESSION_MODE_MISMATCH`, the connection kept); `save_text_file` refuses
  a bench session outright. `report-intake` refuses a bench bundle. The
  shell test `bench_e2e` drives all of it on Linux.
- **The interface** — one button, «Connect the bench (virtual vehicle)»,
  beside «Detect adapter»; while connected, the ground of the whole
  application is the owner's colour B (a warm Namib Orange tint, `#f6e3d3`)
  with a `#c8763a` band «BENCH · virtual vehicle · synthetic data» in the
  header and at the foot; the adapter panel names the bench; every save
  button is disabled with «Bench: nothing is saved»; the report step shows
  the bundle on screen instead of a file; the last step of the rail reads
  «Review the report on screen». Switching kind while records exist asks
  for a new session first. Four interface tests; 45 in all.

State: `IMPLEMENTED / FIXTURE_TESTED`. The bench proves the software, never
a car: nothing from it can become `VEHICLE_CONFIRMED`, by the project's
own rule and by three layers of refusal. Handed out as build 0.9.2 the same
day (commit 89a3725, hashes in the status line above); CI ran the shell's
end-to-end bench test under Windows, but no person has yet opened the
Windows or macOS build.

## 2026-09-05 — towards a working build for testers

The owner redirected the effort: the community gets a working product first,
explanations second, and an unsigned build is acceptable for testers who
run SDD of their own anyway. Recorded in `CLAUDE.md` and `ROADMAP.md`
("Interim runtime", revised).

What changed in the application, so that it works outside the browser:

- **Saving goes through the shell.** Session, capture, calibration and
  module-read reports are saved with a native "Save as" dialog opened by the
  shell (`save_text_file`), which writes the file itself and returns the
  path; a WebView2 download link was never verified and is no longer relied
  on. The browser preview keeps the download.
- **The library folder is chosen, not typed.** "Choose folder…" opens the
  native folder dialog (`pick_directory`); the text field remains.
- **Window** opens at 1280×860, the size the cockpit is drawn for.
- **CI would have failed** on the shell's test-only execution sources
  (`-D warnings` on unused imports outside `cfg(test)`); gated.

State: `IMPLEMENTED / FIXTURE_TESTED`. The shell with the dialog plugin
builds, lints as CI does and passes its tests under Linux in Docker; the
frontend passes 25 tests. The dialogs have not been opened on Windows: the
Windows build of this branch does not exist until the branch is pushed and
CI runs. The hand-out for testers is `TESTER_GUIDE.md` (English) and
`TESTER_GUIDE.uk.md` (Ukrainian, the one to send).

**The medium-speed buses, hypothesised.** The owner relayed a community
statement (`research/sdd/COMMUNITY_NOTES.md`): `BO_MSCAN` and `CO_MSCAN`
are two physical buses joined by the gateway, which relays the tester's
requests from J1962 pins 3/11 to the BO or CO branch. It is the sentence
ADR-0015 lacked; recorded as `UnverifiedResearch` in the built-in hypothesis
manifest (addendum to ADR-0015), it binds both buses to route `ms-can` at
125 kbit/s, `Unverified`. Fleet after the change, same 45 programme-years:
1,705 module rows, 688 reachable on documented routes (unchanged), **716 on
a hypothesised route** (every 2014-and-later high- and medium-speed bus),
287 without identifiers (unchanged), and **14 on unbound buses** — only the
gatewayed sub-networks `NGI`, `SUB_MOST` and `SUB_CAN1` remain. The
statement's provenance is being asked for; a first answered read on such a
car confirms or refutes it per vehicle.

**The tester address, re-checked.** Prompted by a published Discovery 3
reverse-engineering series, SDD's per-module protocol configuration
(`VERONA` blocks, 2,229 of them) was examined for a tester source address:
there is none; the 29-bit files' `srcId` is the module address plus eight
in all 152 cases. ADR-0017's `0xF1` hypothesis stands (addendum), with the
"+8" reading recorded as the one alternative to try on silence. IIDTool
and the series are placed in `research/sdd/COMMUNITY_NOTES.md`.

**A Mac build, 2026-09-05.** Asked by the owner. The macOS CI job no longer
only checks compilation: it builds an unsigned, un-notarised universal disk
image (`jlr-scanner-macos-unsigned-alpha-<sha>`, Apple silicon and Intel).
The tester guides say how the first launch is allowed ("Open Anyway", or
removing the quarantine mark). Adapter discovery on macOS goes through the
serial crate's USB enumeration, not the Windows registry path, and is
`IMPLEMENTED` only: nobody has run the application on a Mac, and no clone
adapter has been seen on one. First image: run 33957488920 on `27b8900`,
all jobs green, `JLR Scanner_0.9.0_universal.dmg`, 6.7 MB, SHA-256
`5b847b18933e3fdaebd00055b5e037230352b5aa9429252773336bf184fcf7c0`, in `Downloads/jlr-scanner-macos-27b8900/`.

**Library copies are stamped, signed and dated (owner's decisions,
2026-09-05 and 2026-09-06; ADR-0019).** Each copy handed to a tester is
made by `stamp_library` (`crates/diagnostic-session/examples/
stamp_library.rs`): `[issued CODE]` goes into every manifest's source
notes, the stamped bundles are written afresh, and `issued_to.json`
records the name, the issue date, the last valid day (thirty days on by
default), the code, each bundle's SHA-256, and the owner's Ed25519
signature over all of that. The application loads a folder **only** under
a stamp signed by a key in its trusted list, matching every bundle and not
past its date; otherwise the built-in data stays, the state is `Failed`,
and the panel says why in the tester's language — expired, unsigned, not
the owner's signature, data changed, stamp removed, no stamp — and to ask
for a new copy. A week before the date the panel asks for a renewal. The
owner's private key is `C:\Users\<you>\.jlr-scanner\library-issuer.key`
(made 2026-09-06, never in the repository; public key id `382146ce` in
`TRUSTED_ISSUER_KEYS`). Tools and tests keep the reporting loader, which
refuses nothing. Setting the clock back is not defended against: the aim
is traceable, stale copies, not an unbreakable lock. The owner's own
working copy is stamped for a year the same way. *Built as `8587697` — the first build of the public
repository and the first that carries both this check and the localized
capture verdict (run 34023004882, all four jobs green, on free public
minutes): macOS `JLR Scanner_0.9.0_universal.dmg`, 8.1 MB, SHA-256
`4f219a569cd8bf621296a56cb42cf3801b042bbc2fe0bec7f489fb4a4358d520`;
Windows `JLR Scanner_0.9.0_x64-setup.exe`, 2.5 MB, SHA-256
`a3e2a4a29d30fca7093befb28b6084a1aab955d92293305c4c33cd7cf3748056`; both
in `Downloads/jlr-scanner-build-8587697/`. This build loads only a signed,
dated library copy.*

**The owner's pictures of the models (2026-09-06).** Twenty-eight pictures,
one per model, drawn in one style, trimmed and stored as WebP under
`apps/scanner/frontend/public/vehicles/` (1.7 MB in all). The rail shows
the picture of the described car by SDD programme, refined by model year
where the model changed face (X250 from 2012, X260 from 2021, X351 from
2016, L319 Discovery 4 from 2010) and by body where the model came as more
than one (X150 and X152 coupe or convertible, X260 Sportbrake). Covered:
S-Type, XF (both), XJ (X350, X358, X351), XK, F-Type, Defender L316 and
L663, Range Rover L322, L405, L460, Range Rover Sport L320, L494, L461,
Evoque L538 and L551, Velar L560, Discovery 3, 4 and 5; and, from the
owner's second batch the same day, Discovery Sport (L550), Freelander 2
(L359), X-Type (X400; the X404 estate wears the saloon's picture), XK8
(X100), XE (X760) and F-Pace (X761): 34 files, 2.2 MB. Only the XK8
convertible (X103) still draws the rail's mark. Verified in the browser
preview: X250 shows the 2008 face at model year 2009 and the 2012 face at
2013. *Built as
`189d9ec` (run 34024659188, all four jobs green): macOS
`JLR Scanner_0.9.0_universal.dmg`, 11.6 MB, SHA-256
`4ff12927c52fde566979865d344f079196539ea0548041c89ef38ac0c76910dd`;
Windows `JLR Scanner_0.9.0_x64-setup.exe`, 4.3 MB, SHA-256
`a04065287f7decdf81291e61670fb19153908881f79339fcdebaa5f30c867c10`; both
in `Downloads/jlr-scanner-build-189d9ec/`. The installers grew by the
pictures.* *Then `417d9d1` with all 34 (run 34025743774, all four jobs
green): macOS dmg 12.4 MB, SHA-256
`a43bd0a3acc7d04ce22e2131abfe60ef908b78b32f2a326eca9dc259ae3b8eb3`;
Windows setup 4.7 MB, SHA-256
`cc361d4482678593cfca82921e4d77d1ab5d37a57f8a178eb91f0b42e53ba6bc`; in
`Downloads/jlr-scanner-build-417d9d1/`.* **Owner's check of `417d9d1` on his Windows PC, same evening:** his own
XF's VIN decoded to the right model and the rail drew the X250 picture;
capture on the bus listened and stayed silent with no car, as it should;
the library panel accepted his signed copy and named its 365 days. The
signed-issue path (ADR-0019), the model pictures and the localized verdict
are therefore `HARDWARE_CONFIRMED` in the product on Windows; the Mac
check of the same build is still his to do.

```bash
# a signed copy for one tester, valid 30 days (Docker; library and key mounted)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads:/downloads" -v "C:/Users/<you>/.jlr-scanner:/issuer" rust:1.98-slim sh -c 'cargo run --release -p diagnostic-session --example stamp_library -- /downloads/prowlone-library "/downloads/prowlone-issued/Ім_я" "Ім'"'"'я Прізвище" 30'
```

```bash
# check a folder exactly as the application will
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads:/downloads" rust:1.98-slim sh -c 'cargo run --release -p diagnostic-session --example stamp_library -- --check "/downloads/prowlone-issued/Ім_я"'
```

`scripts/stamp-library.ps1 -IssuedTo "Ім'я Прізвище" [-Days 30]` wraps the
first command and zips the result. The owner's own hand-book, in Ukrainian,
is docs/OWNER_GUIDE.uk.md: key, issuing, renewals, reports, builds.

What happens next, in order:

1. Push `f8/mongoose-alpha-candidate`; CI builds the unsigned installer
   (`jlr-scanner-windows-unsigned-alpha-<sha>`). *Done 2026-09-05 on the
   owner's word: run 33949327482 on `b0fb46a`, all four jobs green
   (frontend, Rust on Windows, installer, macOS check). Artifact
   `jlr-scanner-windows-unsigned-alpha-b0fb46a…`, expires 2026-09-19;
   downloaded to `C:/Users/<you>/Downloads/jlr-scanner-installer-b0fb46a/`
   as `JLR Scanner_0.9.0_x64-setup.exe`, 2.3 MB, SHA-256
   `901f6341409ac18b147aa6f2a514eea056fa4a6494eabf4166ee906175ae4280`. Unsigned, so Windows will prompt as `TESTER_GUIDE.md`
   describes. Later builds the same day: `cb21a7a` (no console window,
   run 33955685712) and `9a6b08f` (step-driven layout, "New session", the
   calibration panel explained; run 33956980507, all jobs green;
   `Downloads/jlr-scanner-installer-9a6b08f/`, SHA-256
   `355a59878c8f5005ee669126b82dfb95412e7e9168298e37dae9c8ac1847e067`).
   The owner's verdict on 9a6b08f the same day: the structure ("the
   philosophy") is right, but the look is not yet something to give to
   testers; his detailed remarks follow after his own test. Until they are
   in and acted on, no build goes to a tester. The remarks came the same
   afternoon and were worked through live on the dev server, remark by
   remark (F11 slice 13); his verdict at the end: "splendid for a test
   version". Built as `8cfd571` (run 33965860856, all four jobs green) with
   the gateway explanation and the library stamp: Windows
   `JLR Scanner_0.9.0_x64-setup.exe`, 2.3 MB, SHA-256
   `fd7c541d663f266950149b10b77b7e4840dcb64e10ef02f8fde6de9145b31449`;
   macOS `JLR Scanner_0.9.0_universal.dmg`, 6.7 MB, SHA-256
   `95b6a35c69be40b751e89d3e2f7aeddeee8afbdb72e0ebb74f1d520f267f0cea`;
   both in `Downloads/jlr-scanner-build-8cfd571/`. The owner's second
   round on that build: the ground read blue rather than green — greener
   now; the MOST modules stood in a column while the others lay in rows —
   the sub-network's note had taken the nodes' grid cell, fixed; the rail
   chips differed in size and the state word wandered — every chip is now
   48 px with the state in its own right-aligned column and no hint text;
   the car silhouette is gone, a ring-and-hooks mark after the owner's
   emblem stands where the vehicle image will be. Rebuilt as `adf7eba`
   (run 33967630251, all four jobs green): Windows SHA-256
   `02ef8f1cc0b8644958fc8f62f10e9cf9fea87a14ac21f0c32178dfcf58a4c1a0`,
   macOS SHA-256
   `439b0867e9e8c35cab40976d99e7d496387d607be520af574556614b800f0618`,
   both in `Downloads/jlr-scanner-build-adf7eba/`. These supersede
   8cfd571 as the builds to hand to testers. The owner's verdict on his
   PC the same evening: "simply superb; ideal for now". Still to run:
   the Windows build on the laptop. The macOS image was tried the same
   evening and refused outright — Finder's "cannot be opened", OK only,
   no "Open Anyway": Apple silicon does not start a bundle with no
   signature at all. The macOS job now seals the bundle with an ad-hoc
   signature (identity "-") and builds the disk image from the sealed
   bundle; the tester guides carry the two-command repair for a copy
   already downloaded. A second cause found in the CI log the same night:
   Tauri warns that the bundle identifier `com.jlrscanner.app` ends in
   `.app`, which macOS confuses with the bundle extension — a known way to
   get exactly Finder's terse refusal. The identifier is now
   `com.jlrscanner.desktop` (a fresh install on Windows, not an in-place
   upgrade of the alpha; nothing was stored under the old name). Built as
   `7a38b68` (run 33987521013, all four jobs green; the log shows
   `Identifier=com.jlrscanner.desktop`, `Signature=adhoc`): macOS
   `JLR Scanner_0.9.0_universal.dmg`, 7.6 MB, SHA-256
   `385cf98ba54813f479b4dec126a2ad864223d300bbd2257adf41a80d7e9af342`;
   Windows `JLR Scanner_0.9.0_x64-setup.exe`, 2.3 MB, SHA-256
   `abcdf809556ab5b80fd34a1bd107156180c8058b02a596af73932aa6b867237e`;
   both in `Downloads/jlr-scanner-build-7a38b68/`. The image opened on
   the owner's Mac.*

**First live test, 2026-09-06 — the live CAN path does not work on real
hardware, and the reason is now known.** On the Mac and then on Windows,
every capture and read failed at channel open with device status 1. A
direct byte-level probe of the genuine adapter (`docs/evidence/
F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`) showed: the adapter boots in
its **bootloader** (1.1.8; firmware 1.1.16 both visible in board-info);
in the bootloader every working command is "Invalid or Unhandled command
type"; `cJumpToFirmware` (0x0103) moves it to the firmware cleanly; and in
the firmware our `cOpenChannel` is rejected as "Unsupported or Invalid
Resource ID 31". Consequences: F1/F3 `HARDWARE_CONFIRMED` was bootloader
board-info only; F2's channel-open encoding, `STATICALLY_CONFIRMED`, is
contradicted on hardware. Two gaps: the application never leaves the
bootloader, and the firmware's channel-open resource sequence is unknown
to us. Everything without an adapter — library, VIN, survey, UI — is
unaffected. Reads cannot work against a car until both gaps are closed and
validated on this device. The exact sequence is best obtained from a USB
capture of genuine SDD opening a channel on the owner's Windows PC with
this adapter; blind probing is not the way to a trustworthy fix. No build
goes to testers for live reads until then. **Update, same day:** probing
found a hardware-validated listen-only HS-CAN sequence (jump to firmware,
open a CAN resource, `cSetPin(6,14)`, listen — all status 0); the board
refuses pins 3/11 ("only supports CAN on pins 6 and 14"), casting doubt
on ms-can as a route; the canonical resource ids and the read sequence
still want a genuine vendor-library capture, which needs the 2012 driver
loaded (blocked by driver-signature enforcement on this Windows 11). See
the Part 2 evidence. **Update, same evening:** the owner objected that SDD
reaches MS-CAN through this very adapter, and a sweep of every resource id
proved him right — resource 21 (CAN2) takes pins 3/11 and refuses 6/14,
resource 5 (CAN1) the reverse; `ms-can` listen-only runs end to end, and
the application's own `cOutboundData` record is accepted (status `0x100`
on a bench where nothing can acknowledge it). Part 3 of the evidence holds
the resource map. **ADR-0018 is implemented** in `crates/mongoose-jlr`:
the transport reads board-info at the first route open, sends
`cJumpToFirmware` when the bootloader answers, opens the route's resource
word and demands it back as the token, and puts the firmware's own words
into a refusal. The workspace passes in Docker, with the transport-fake
tests scripting the board-info exchange and the jump. Validation: the
command sequence is `HARDWARE_CONFIRMED` on the bench by the probe, the
Rust encoding `FIXTURE_TESTED` against the same bytes; what a car does
with it — frames in non-listen mode without a filter, the meaning of
`0x100` — is `VEHICLE_CONFIRMED` territory, still empty. The
vendor-library capture, and the driver-signature change it needed, are no
longer required. *Built as `5d9840e` (run 34017617970, all four jobs
green; the shell compiles against the new crate): macOS
`JLR Scanner_0.9.0_universal.dmg`, 7.9 MB, SHA-256
`3e3d8c055d5f1e5d903e4f05a3471a2e36a73aaba7d18c9d868b88199a828591`;
Windows `JLR Scanner_0.9.0_x64-setup.exe`, 2.4 MB, SHA-256
`aa5ca6d8773d508626d0a6f04444f0e7b10ca7ed926c1ddc7da847799c6ea248`;
both in `Downloads/jlr-scanner-build-5d9840e/`. **Bench check passed the
same evening on the owner's Windows PC:** with the adapter and no car,
capture on HS-CAN (pins 6/14, 500 kbit/s, 5.0 s) and on MS-CAN (pins
3/11, 125 kbit/s, 5.2 s) each finished "Silent", zero frames, no error —
first with the adapter already in firmware, then again after unplugging
and replugging it, so the jump from the bootloader and its polling ran
inside the application too. The F2 live path is therefore
`HARDWARE_CONFIRMED` in the product itself on Windows (channel open, pins,
listen, close, both buses, the jump); frames from a vehicle remain
`VEHICLE_CONFIRMED` territory, still empty. The same build carries the
owner's third round on the ground colour (leaf green, not blue-grey) and
the tester guide's new bench step. One defect seen in the test: the
capture verdict was English in the Ukrainian interface — it was composed
in the shell; the frontend now composes it in the interface language
from the snapshot's counts. **That fix (`c1a6c33`) has no build yet:**
from 07:05Z on 2026-09-06 GitHub starts no job on this private
repository — "recent account payments have failed or your spending limit
needs to be increased" — the month's Actions minutes went on two days of
pushes, each billing the macOS universal build at ten times its wall
clock. Only the owner can lift this (GitHub → Settings → Billing: raise
the spending limit or fix the payment; or make the repository public,
where standard runners are free). The workflow now skips pushes that
touch only documents and cancels a superseded run, so the next month's
allowance lasts. The `5d9840e` builds remain valid for testing; once
minutes exist, `gh run rerun 34018679986` builds `c1a6c33`.*
2. The owner installs it on the Windows 10 laptop with the MongoosePro JLR
   plugged in — the first time since F3 — and walks `TESTER_GUIDE.md` to the
   end without a car: install prompt, library, adapter. Anything that fails
   there is fixed before a tester sees it. *First result, 2026-09-05: the
   owner installed and ran the `b0fb46a` build on his own Windows machine;
   everything he tried behaved as expected. One defect: a black console
   window opened beside the application, because `main.rs` lacked the
   `windows_subsystem = "windows"` attribute for release builds. Fixed in the
   next commit; the next CI build carries it. His fuller report: started
   without the adapter, plugged the MongoosePro in, the application found it,
   offered the connection, connected and verified the board; vehicle chosen
   by hand and by VIN; library loaded through the folder dialog; every
   module of the survey shown with its explanation; reports saved through
   the save dialog; the step legend praised. No car. The F11 application is
   therefore `HARDWARE_CONFIRMED` on real Windows for the adapter, library,
   dialog and survey paths, not `VEHICLE_CONFIRMED`. His remarks: the
   layout mixes everything at once and lacks a logic — restructured around
   the steps (F11 slice 12); no
   way to start over — a repeated survey redrew the map but the legend
   stayed — fixed by "New session"; the F8 "first supported live profile"
   card made no sense beside a survey of another car — removed, the
   calibration panel now explains itself (F11 slice 11).*
3. The library folder is zipped for named testers (`G4`). *Done
   2026-09-05: `C:/Users/<you>/Downloads/jlr-scanner-library-2026-09-04.zip`,
   4.9 MB (166 MB unpacked), the eight bundles plus a Ukrainian read-me;
   it stays with the owner and goes to each tester by hand.*
4. First cars: the owner's circle, then the community.

## Where things stand at the break of 2026-09-04

Resume from here. Seven commits landed on 2026-09-04, all on
`f8/mongoose-alpha-candidate`:

| Commit | What |
| --- | --- |
| `191bb2e` | M1: vehicle picker from the library, fault-code wording, decoded values |
| `c747882` | M1: one session flow, one session report |
| `8e9ff42` | M1: the vehicle network map as SDD draws it, explained; check all modules; named states |
| `611513d` | M1: VIN decoding from SDD's own tables, module names from SDD's text database, Ukrainian interface |
| `25351d0` | M1: Russian interface, data text following it |
| `898ccc0` | M2: session reports become captured evidence (`report-intake`, ADR-0016) |
| `0dc27be` | M3, first half: derived `normal_fixed` identifiers, 29-bit live reads (ADR-0017) |

**Milestones.** M1 done (`F11_TESTER_APPLICATION.md`). M2 done
(`F13_REPORT_INTAKE.md`), awaiting the first report. M3 first half done;
the medium-speed bindings of the 2014-and-later cars wait on a tester's
evidence. M4 (signing) and M5 (library to testers) deferred by the owner
until real results, the unsigned build running on the owner's own hardware
meanwhile. M6 (tester programme, first live tests) is next and is the first
step that needs the MongoosePro plugged in and a car.

**Open, and why.** The map-converter scale (404 parameters shown as raw
counts; no converter runtime in the payload, paired siblings disagree);
the tester address `0xF1` for normal-fixed buses (a hypothesis until a
module answers); the 29-bit transmit encoding (flag set in both status
words, not hardware-confirmed); the 2014-and-later medium-speed bus
bindings (454 module rows on hypothesised routes, 276 on unbound buses);
K-line for the early Range Rover's body modules (F14); aggregation rules
across many reports (F13, after reports exist); Ukrainian data text (SDD has
none); an "import a report" action inside the application.

**Nothing has met a vehicle.** Every state above is `IMPLEMENTED` and
`FIXTURE_TESTED`; the data-derived parts are `REAL_SOURCE_SURVEYED` against
the exported library; adapter discovery and board identity remain the only
`HARDWARE_CONFIRMED` facts (F3); `VEHICLE_CONFIRMED` is still empty.

**The exported library** lives on the owner's machine at
`C:/Users/<you>/Downloads/prowlone-library` (6,500 manifests, 6,505
sources, 129,437 records, exported 2026-09-04 from the eight roots below).
It is not in the repository (`G4`: named testers only) and loads in about
eight seconds.

**How to reproduce today's numbers.** Rust work runs in Docker on this host
(Smart App Control blocks fresh binaries); the corpus is mounted read-only.

```bash
# workspace lint and tests (207 tests, 55 suites)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry rust:1.98-slim sh -c 'cargo fmt --all; cargo clippy --workspace --exclude prowlone-shell --exclude transport-serial --all-targets; cargo test --workspace --exclude prowlone-shell --exclude transport-serial'
```

```bash
# the Tauri shell under Linux (21 tests)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v jlr-apt-cache:/var/cache/apt -e CARGO_TARGET_DIR=/w/target/linux-shell rust:1.98-bookworm sh -c 'apt-get update -qq && apt-get install -y -qq libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libudev-dev pkg-config; cd apps/scanner/src-tauri && cargo test'
```

```bash
# re-export the library from the ten corpus roots (about 100 s)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads/prowlone-library:/library" -v "C:/Users/<you>/Downloads/SDD169_XML:/corpus_d" -v "C:/Users/<you>/Downloads/SDD169_PAYLOAD/SDD_XML_PAYLOAD:/corpus_e" rust:1.98-slim sh -c 'cargo run --release -p sdd-ingest --example export_manifests -- /library /corpus_d/CURRENT_JLR_XCL_XML_DATA_XML /corpus_d/CURRENT_JLR_MCP_XML_XML /corpus_d/CURRENT_JLR_VIN_DECODE_XML /corpus_d/COMMON_JLR_SMPACK_XML /corpus_e/COMMON_SDD_DATA_SNAPSHOT_LANG_EN /corpus_e/COMMON_SDD_DATA_DTC_HELP_LANG_EN /corpus_ru/COMMON_SDD_DATA_DTC_HELP_LANG_RU /corpus_e/COMMON_SDD_DATA_ODST_LANG_EN /corpus_e/CURRENT_PAG_UTILS_RUNTIME /corpus_e/CURRENT_PAG_MCP_TEXT_XML'
# — with the Russian help pack mounted as well, e.g. -v "<where the owner's scripts wrote it>/SDD_original_help/RU:/corpus_ru" (`ADR-0034`)
```

```bash
# fleet coverage over the 45 programme-years (docs/research/sdd/fleet_programme_years.txt)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads/prowlone-library:/library" rust:1.98-slim sh -c 'cargo run --release -p diagnostic-session --example fleet_coverage -- /library $(tr "\n" " " < docs/research/sdd/fleet_programme_years.txt)'
```

The frontend runs natively: `corepack pnpm lint`, `corepack pnpm test`
(22 tests), `corepack pnpm build` in `apps/scanner/frontend`; the browser
demo is `corepack pnpm dev` at `http://localhost:1420/?demo=alpha`. The
catalogue and VIN tools are `cargo run --release -p diagnostic-session
--example catalogue -- <library> [--vin <VIN>...]`; the intake is `cargo run
-p report-intake --example intake -- <report.json> <library dir>`.

**What M6 needs before the first car.** Testers with a two-bus-era car
(X250, X150, X351 MY10–13, L319/L320 MY10 onward, L405 MY13) and a
2014-or-later car; the safety and consent text; a build they can run — the
owner's unsigned build on the owner's hardware, or the signed one once M4 is
taken up; the library folder handed to each named tester (`G4`); and the
MongoosePro JLR plugged in, for the first time since F3.

## M3 derived normal-fixed identifiers

ADR-0017. The platform adapter derives, on request, 29-bit CAN identifiers
for every physically addressed module on a `normal_fixed` bus — the
platform's prefix and address, ISO 15765-2's layout, and the tester address
`0xF1` as a research hypothesis — and the record stays `Unverified` until a
module answers (F13 intake). Two built-in manifests carry the standard and
the hypothesis; the exporter loads them first. The MongoosePro live path
accepts `normal_fixed` and sends 29-bit frames with the device's 29-bit flag
in both status words, not yet hardware-confirmed. Fleet-wide after
re-export: 73 derived records, reachable rows 617 → 688, "no
identifiers" 360 → 287 (the gatewayed sub-network modules remain).
Surveyed after re-export: L319 MY05 27 modules, 19 reachable; L320 MY06 28, 20; L322 MY04.5 10, 2; L322 MY06 33, 10 (ACM on ms-can at 0x18DA80F1/0x18DAF180, the rest gatewayed sub-network or K-line modules); L322 MY07 36, 20. Every one of the 45 programme-years now has at least one reachable module (40 before).

## F13 report intake (milestone M2)

F13's intake is `IMPLEMENTED / FIXTURE_TESTED`; see `docs/F13_REPORT_INTAKE.md`
and ADR-0016. `crates/report-intake` turns one session report and the
library into one `Captured` manifest (`capture-session-<id>.json`) that the
application's loader reads like any other. What a module's answer confirms
is copied from the library so agreement is exact; silence, listen-only
captures and calibration reads are observations that confirm nothing; fault
codes are not knowledge. With ADR-0016 the resolver rates a fact by the best
of its agreeing records and calls a route a hypothesis only while every
trace behind it is research, so one direct observation makes the route
`REACHABLE` and `CAPTURE_VALIDATED`. The tool is a command line for the
programme's maintainer; nothing has met a vehicle.

## F11 tester application (milestone M1)

F11 is `IN_PROGRESS`; see `docs/F11_TESTER_APPLICATION.md`. Done on
2026-09-04, all offline and without the adapter:

- **Vehicle picker.** `KnowledgeLibrary::catalogue()` lists, per programme,
  the SDD breakpoint markers with derived model years and the engines the
  DID catalogue is qualified by; the shell's `get_vehicle_catalogue` feeds
  three lists in the survey panel. On the exported library: all 18
  programmes, 49 markers, 2–7 engines each.
- **Fault-code wording.** `KnowledgeLibrary::describe_dtc` joins a J2012
  code and failure-type byte to SDD's wording, module-scoped first, generic
  otherwise; every live fault-code read carries it.
- **Value decoding.** Converter arithmetic is now exported verbatim into each
  parameter's encoding; `diagnostic_session::decode` applies linear
  converters (engine speed `0xF40C` agrees with SAE J1979) and withholds map
  converters, whose breakpoint values are not consistently in the declared
  unit — shown as raw counts with the reason.

- **One session, one report.** The `Session` panel orders the seven steps
  and marks them from evidence; `SessionReportService` bundles every capture,
  module read and calibration read with the adapter, library and last survey
  into one `jlr-scanner.session-report` document that says of itself that
  bundling confirms nothing.
- **The vehicle network, as SDD draws it, explained.** Buses as lanes with
  their adapter route or the reason there is none, modules as nodes whose
  state is in words with a reason, a legend in sentences, a detail panel with
  the read actions and results, and *Check all modules* — SDD's network
  integrity test, read-only, one fault-code request per module in lane order.
- **Named states.** The 4,438 `quantityState` ranges of 969 converters are
  exported into the encoding descriptors and shown by name beside the value.

- **VIN decoding.** SDD's `VINDecode.xml` — 79 rule blocks, 33 decode
  models, 2,473 lookup rows — is ingested verbatim and applied by
  `diagnostic_session::vin`; the vehicle card decodes a typed VIN and
  pre-selects programme and breakpoint, naming what the tables do not hold.
- **Module names.** SDD's text database names 60 of the 114 ECU families in
  twelve languages; the map and the module panel show the English name.
- **Interface language.** English, Russian or Ukrainian from the header.
  SDD's data text — module names, failure-type wording — follows the Russian
  interface (SDD's text database has Russian) and stays English for the
  Ukrainian one (it has no Ukrainian); fault-code descriptions are English
  only in the payload. Owner decision 2026-09-04.

Still open: the map-converter scale (no converter runtime in the payload's
jars; paired linear siblings disagree), so 404 parameters stay raw counts.

## F10 multi-module read-only diagnostics

F10 is `IN_PROGRESS`. Three slices are `IMPLEMENTED / FIXTURE_TESTED`; see
`docs/F10_MULTI_MODULE_DIAGNOSTICS.md`.

- **Enumeration.** `DiagnosticEnvironmentResolver::enumerate_ecu_families`
  reports which ECU families the knowledge associates with a vehicle, whether
  each has a request and a response identifier, and which bus it sits on. A
  report, not a permission.
- **Family-level plans and route bindings (`ADR-0013`, 2026-09-03).** A plan
  may now omit the diagnostic implementation when the target names only a
  family, which is all SDD ever describes. The platform adapter records bus
  rate, protocol and the two read-only UDS capabilities per module. Which
  adapter route reaches an SDD bus is a documented knowledge manifest,
  `fixtures/knowledge/documented/mongoose_jlr_route_bindings.json`; only
  `CAN_HS` and `CAN_MS` are bound, and every other bus resolves INDETERMINATE
  with the missing route named. `resolve_ecu_family` and
  `readable_identifiers` are the F10 entry points.
- **UDS read execution (`ADR-0012`, 2026-09-03).** `crates/uds-execution`
  prepares and executes `0x22` ReadDataByIdentifier, gated by the module's
  readable-identifier catalogue, and `0x19` ReadDTCInformation by typed status
  mask, against simulator and replay sources only. Negative responses are
  results; NRC `0x78` is waited through; DTCs decode to SAE J2012 codes with
  failure type. `enhanced` addressing is refused. The architecture checker
  guards the crate like `diagnostic-execution`, plus session-control and
  programming constructors.

- **Composition root (`ADR-0014`, 2026-09-03).** `crates/diagnostic-session`
  loads a data library — the built-in documented manifests plus a directory of
  exported F5 manifests or bundles, each ingested atomically and reported by
  name when it fails — and surveys a vehicle into reachable and unreachable
  modules with the resolver's reasons. The shell exposes it as three commands;
  the UI gains a data-library panel and a module table. `KnowledgeStore::ingest`
  is now atomic without cloning. The `sdd-ingest` example `export_manifests`
  exported the real corpus in 30 seconds: 6,316 manifests, 126,706 records,
  none rejected. Surveyed offline: X250 MY2010 → 39 modules, 28 reachable and
  11 gatewayed ones shown with the reason; L405 MY2014 → 52 modules, none
  reachable, each naming its unbound bus; L322 MY2006 → 33 modules, none
  reachable, CAN ones for want of derived `normal_fixed` identifiers and DS2
  ones for want of K-line. The survey exposed that the platform adapter had
  skipped physically addressed modules entirely; they are now recorded
  verbatim (`sdd_physical_address`) and seen, never routed. Distributing the
  exported library is the open owner gate `G4`. Fleet-wide, `fleet_coverage`
  counts 1,705 module rows over all 45 programme documents, 617 reachable on 23
  programmes; 728 wait on bus bindings for the 2014-and-later architectures and
  360 on `normal_fixed` identifier derivation.

- **Listen-only capture (2026-09-03).** `MongooseJlrDevice::capture_route`
  records a route with the listen-only flag for a bounded time; the shell's
  `capture_bus` command summarises it and saves a `captured` replay fixture
  with its provenance; the UI gains a "Bus capture" panel. The verdict states
  what listening proves — traffic present or absent on a pair — and that it
  cannot name which vehicle bus was heard. It is the tester's zero-risk first
  contact, not a bus binding. The search for public evidence on where the
  2014-and-later diagnostic connector attaches found leads but no citable
  source; nothing was recorded as a fact.

- **Route hypotheses and live UDS reads (`ADR-0015`, 2026-09-03).** The
  2014-and-later high-speed buses are bound to `hs-can` as an
  `UnverifiedResearch` hypothesis the survey shows as such; a plan's rate is
  the route's. `MongooseJlrDevice::execute_prepared_uds_read` executes a
  prepared read live — padded single frame, flow control, ResponsePending,
  route closed whatever happens — and the shell's `read_module` command with
  the "Module read" panel drives it and keeps a report. Fleet-wide: 617
  modules reachable, 454 on a hypothesised route, 360 without identifiers,
  274 on unbound buses; 40 of 45 programmes have something to attempt.

Two earlier claims in this file and in the F10 document were wrong and are
corrected there: MDX `ECU_DATA` access parameters were never ingested, and the
early Range Rover is not "unreachable over CAN"; only its body modules are on
K-line. Bus coverage is decided in `ROADMAP.md`: CAN first, K-line as F14.

Workspace: 207 (55 suites) Rust tests, 21 shell tests (run under Linux in Docker; the
shell compiles there), 22 frontend tests; clippy clean, formatted,
architecture boundaries pass. Live vehicle status is unchanged.

## F9 SDD knowledge extraction and ingestion

F9 has ingested the surveyed SDD corpus across six slices, each
`IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`. The `sdd-ingest`
crate implements the existing F5 `IngestionAdapter` and depends only on
`knowledge` plus a read-only XML parser; nothing depends on it. ADR-0009
records the decision.

Evidence class and validation state are derived from the registered source
type, so a synthetic fixture cannot be presented as JLR evidence. Ingesting the
extracted SDD `CANLinkMonitorData.xml` produced 161 evidence and 161 knowledge
records: 25 vehicle addressing claims and 136 module aliases. `X250` MY2010
resolves to `Standard11Bit`, independently corroborating the F8 prepared
transaction; the global `7E0` alias resolves as `InsufficientEvidence` for a
specific vehicle, which is the intended fail-closed outcome.

Real data revealed two cases the synthetic fixture did not: prose model-year
markers (`Post MY10`) whose boundary is unstated and therefore left `Unknown`,
and ten reused mnemonics that now surface as `ConflictReport` rather than
overwriting each other. Both are golden-tested.

The DID slice adds a converter catalogue and a formatting-catalogue adapter.
Real ingestion loaded 1,854 converters and produced 15,281 parameter definitions
across 1,385 distinct DIDs, every one carrying an engineering unit, with nothing
rejected. SDD qualification maps onto applicability: module to ECU family, model
to vehicle program, type and subType to powertrain and variant. Model-year
breakpoints such as MY10 are recorded verbatim on their own dimension and leave
model_year unknown, because the published data never states what a breakpoint
spans. Unrecognised qualification, disjunction, and any non-ReadParameter
element stop ingestion rather than being skipped.

The DTC slice parses the 6,168 per-code help documents into 93,521 records over
6,168 fault codes, 119 modules, and 28 vehicle programs, with 897 conflicts in
SDD's own data surfaced rather than resolved, with none rejected: a malformed identifier is
normalised while the raw code stays verbatim in the evidence. ADR-0010 adds EntityKind::DiagnosticTroubleCode for this. The
corpus spans MY94 to MY17, which is why model-year designations are recorded
verbatim and never mapped onto calendar years: 2000+## would have placed 1990s
Jaguars in the 2090s.

The ODST slice is the first to touch material that is not read-only. An
on-demand self test commands an ECU to act, so it is stage-2 material: all 1,153
records from 93 module documents carry DiagnosticSafetyClass::ServiceRoutine and
none is ReadOnly, asserted by both a golden test and the real-source example.
Recording them as known-but-unavailable is what ADR-0009 permits; nothing can
execute them, because the execution surface accepts only typed read-only intent.

ADR-0011 adds opt-in model-year resolution. Supplying a ModelYearTimeline
expresses an SDD marker additionally as a calendar year range while always
retaining the marker verbatim, so the derivation is reversible. Two-digit years
resolve by validation against SDD's closed 1994-2017 window rather than by an
assumed century, which keeps 1990s Jaguars out of the 2090s. Deriving the ranges
exposed a separate defect: because SDD qualification is a conjunction, a
dimension its expression does not test is unconstrained rather than
undetermined, and leaving those Unknown had made every qualified record
unresolvable. Asking for X250, model year 2010, PCM, V6 diesel now returns 8
applicable parameter definitions with names, units and byte layouts.

The platform documents of the corpus became available to the ingestion on
2026-09-02; the provenance record notes that this
assistant predicted the addressing would be in the MDX PROTOCOL sections and was
wrong twice before finding it. The addressing is in the platform documents:
1,418 module addressing claims over 89 modules and 22 vehicle programs from 110
files, none rejected. PLATFORM_X250_201000 declares PCM at 0x7E0/0x7E8 on CAN_HS,
a third independent confirmation of the route F6 and F8 already used.
Programming-session addressing is deliberately not ingested.

Derived knowledge is documented evidence only. Nothing here is vehicle
confirmed, and ingesting a manufacturer document never makes it so. Firmware is
never ingested and programming-session addressing is never recorded. See
`docs/F9_SDD_KNOWLEDGE_INGESTION.md`.

## F8 tester-ready Mongoose alpha candidate

F8 adds the first real desktop diagnostic workflow for exactly one supported
profile: 2010 Jaguar XF/X250 5.0L Supercharged ECM/PCM. The only operation is
the typed read-only Calibration Identification request. The frontend sends no
CAN ID, bytes, service, DID, route, or protocol values; its Tauri intent has no
user payload. Startup, automatic USB discovery, adapter connection, and
board-info do not open CAN or send a diagnostic request.

The product profile supplies an F6 evidence-backed RESOLVED plan to the existing
F7 compiler. `mongoose-jlr` accepts only the resulting
`PreparedDiagnosticTransaction`, validates the confirmed HS-CAN pins 6/14,
500000 bit/s and 11-bit physical `0x7E0` / independently evidenced `0x7E8`, and
keeps active open, outbound frames, flow control, ACK correlation, and close
private. Shared ISO-TP and SAE J1979 decode the response. Arbitrary application
CAN TX API remains NO.

The same application service returns `CX23-14C204-ZAD` through simulator and
replay. A transport fake covers the fixed Mongoose exchange without hardware.
Machine-readable reports are local-only and omit adapter serial/COM, VIN, and
unrelated personal information.

No vehicle was connected and no live CAN channel was opened for acceptance.
The live path is `NOT_YET_EXTERNALLY_VALIDATED`. CI produces an explicitly
unsigned NSIS alpha and checks macOS compilation. Public distribution remains
blocked by the unchanged F3.1 trusted-signing dependency; macOS Mongoose runtime
is not validated. See `docs/F8_MONGOOSE_ALPHA_CANDIDATE.md`.

## F7 offline read-only diagnostic execution

F7 adds the independent `obd-j1979` and `diagnostic-execution` crates.
The architecture audit confirmed that the existing F4 `diagnostics-core` is
UDS-specific, so SAE J1979 remains a separate application protocol rather than
being forced into UDS request/result types. ADR-0007 records the decision.

The execution bridge accepts only a RESOLVED F6 plan and typed
`ReadOnlyDiagnosticIntent::CalibrationIdentification` for an explicit ECU
family and implementation. It rejects INDETERMINATE, CONFLICT, target or
capability mismatch, unsupported protocol/addressing, incomplete routes, zero
bitrate, invalid CAN ID width, and missing observed calibration provenance.

The prepared transaction retains HS-CAN, J1962/C2DB04B pins 6/14, 500000 bit/s,
the explicit physical request `0x7E0`, independently evidenced response
`0x7E8`, functional `0x7DF` metadata, Mode 09 InfoType 04 identity, target
ECM implementation, READ_ONLY safety, and field-level F6 traces. It contains no
request/response arithmetic and exposes no arbitrary payload or transmit API.

The SAE J1979 codec validates SID `0x49`, InfoType `0x04`, the item count,
fixed 16-byte printable-ASCII calibration fields, and trailing NUL padding. The
real F6 environment drives both a synthetic multi-frame simulator golden and a
machine-readable deterministic replay golden through the existing ISO-TP
reassembler. Both return exactly `CX23-14C204-ZAD`; synthetic execution bytes
remain distinct from real environment evidence.

F7 opens no vehicle, Mongoose adapter, live CAN channel, ECU discovery, Mode 22,
DTC workflow, write/control/security operation, frontend, or signing path.
Application CAN TX API remains NO. See
`docs/F7_READ_ONLY_DIAGNOSTIC_EXECUTION.md`.

## F6 diagnostic environment resolution

F6 adds the independent `diagnostic-environment` crate. It consumes an
explicit F5 VehicleContext and DiagnosticTarget and returns typed RESOLVED,
INDETERMINATE, or CONFLICT data. A complete CAN plan represents applicability,
ECU family and implementation, logical and physical routes, an evidence-backed
backend route descriptor, bitrate, protocol, addressing mode, CAN-ID width,
physical request/response IDs, optional functional address, READ_ONLY
capability, optional implementation markers, and field-level provenance.

The resolver depends only on knowledge. It cannot construct UdsRequest, call
diagnostics-core, open Mongoose/CAN, transmit, or execute diagnostics. F4
production behavior is unchanged, frontend is unchanged, and application CAN
TX remains absent.

The existing X250 C2DB04B pins 12/13 wiring source is a mandatory real negative
golden and remains INDETERMINATE. It supplies no inferred bitrate, protocol,
addressing, or Mongoose route.

The real positive X250 Service 09 InfoType 04 golden is accepted through a
field-level evidence join. A minimal normalized public-index snapshot preserves
the OBD Fusion report's 2010 Jaguar XF Supercharged context, VIN
`SAJWA0HE4AMR59890`, observed response `0x7E8`, and calibration ID
`CX23-14C204-ZAD`. The unavailable original forum attachment is not
represented as locally captured evidence.

OEM and standards sources independently establish X250 5.0L ECM applicability,
HS-CAN through J1962/C2DB04B pins 6/14 at 500 kbit/s, ISO 15765-4 11-bit
addressing, functional request `0x7DF`, physical request `0x7E0`, and SAE
J1979 Mode 09 InfoType 04 semantics. The production source descriptor
independently establishes the `mongoose-jlr` `hs-can` backend route. The
golden keeps documented request roles separate from the directly observed
response and never derives one CAN ID from another.

The evidence matrix, locators, fingerprints, acquisition limitation, and join
are recorded in `docs/evidence/F6_EXTERNAL_EVIDENCE_2026-08-31.md`. The
existing X250 pins 12/13 CCP negative, synthetic conflicts, and deterministic
coverage remain intact. No production execution mapping or CAN TX API was
added.

Windows Application Control previously blocked rebuilt unsigned local test
binaries with error 4551; signing remains out of scope. The evidence-bearing F6
commit is validated once locally for the relevant changed crates and by the
checks attached to PR #6, which are the CI source of record. See
`docs/F6_DIAGNOSTIC_ENVIRONMENT_RESOLUTION.md`.

## F5 evidence-first JLR knowledge

F5 implements `SOURCE -> INGESTION -> EVIDENCE -> NORMALIZED KNOWLEDGE ->
APPLICABILITY -> QUERY` in the standalone `knowledge` crate. The source
registry, SHA-256 identity, strict schema/parser versions, typed records,
multi-dimensional fail-closed applicability, discrete validation states,
conflict preservation, deterministic transactional JSON ingestion, structured
query, and exact source trace-back are implemented.

The real-source golden fixture references the user-provided X250 Electrical
Wiring Diagrams, JLR publication `13 56 10_1E`, SHA-256
`406e07bfc3a8afc2caaa7384795210d911d44b3e23ca53ed7aab663c2782bee3`.
PDF page 181 / printed page 133 / section `418-00` identifies diagnostic
connector `C2DB04B` pins 12/13 as `HS_CAN_POS_CCP` / `HS_CAN_NEG_CCP`. The
restricted PDF is not committed; only metadata, hash, exact locator, and a
minimal excerpt are retained. The extracted route is `source_backed`, scoped to
X250, and all unproven applicability dimensions remain `unknown`. This is
vehicle-side knowledge, not a Mongoose capability; F2 still prohibits touching
pins 12/13.

Synthetic fixtures are generic, visibly synthetic, and cannot be promoted or
mixed with real evidence. Conflicting claims coexist and query returns an
explicit conflict. `UNKNOWN != ANY`; insufficient source scope and missing
vehicle context cannot become an exact match.

Focused formatting, clippy with warnings denied, compile-only workspace tests,
and architecture checks pass locally. Local test executables remain subject to
the previously documented Windows Application Control error 4551. GitHub
Baseline validation run `33359988532` passed frontend lint/tests/build and the
Windows Rust fmt, workspace clippy, all workspace tests, both F5 source-to-query
golden tests, the conflict scenario, and Tauri shell check for implementation
commit `1e7890f`. F5 satisfies its real-evidence and CI gates. See
`docs/F5_JLR_KNOWLEDGE_SYSTEM.md`.

Mongoose device transport is `HARDWARE_CONFIRMED` on Windows 11:

- MongoosePro JLR USB VID/PID `18E1:0104` is discovered dynamically; no COM port is hardcoded.
- The F1 accepted device was `AOLHE00000xxxxxx` on dynamically resolved `COM3`, Microsoft `usbser`, PnP `OK`.
- Production `cGetBoardInfo` request/response framing and its real golden fixture remain unchanged.

F2A Mongoose CAN Backend is `IMPLEMENTED`, `FIXTURE_TESTED`, `STATICALLY_CONFIRMED`, and `NOT_VEHICLE_VALIDATED`. It provides:

- a corrected two-route production inventory that separates logical protocols from physical pins;
- HS-CAN pins 6/14 at 500 kbit/s: backend route implemented, `NOT_VEHICLE_VALIDATED`;
- MS-CAN pins 3/11 at 125 kbit/s: backend route implemented, `NOT_VEHICLE_VALIDATED`;
- X250 vehicle-side `CCP_HS_CAN` on 12/13 classified `UNSUPPORTED_BY_MONGOOSE_JLR` and not an F2 blocker;
- MongoosePro JLR pin 12 classified as PS GND and pin 13 as FEPS; both are prohibited in F2;
- atomic CAN `cOpenChannel` with `DT_LISTEN_ONLY`;
- dynamic device channel correlation and exact `cSetPin`/`cCloseChannel` setup restricted to production route descriptors 6/14 and 3/11;
- raw inbound CAN parsing for DLC 0..8, standard and 29-bit IDs, raw device timestamp, and route identity;
- fail-closed route/status/sequence/channel/length/verifier validation;
- close/reopen support and cleanup after setup failure;
- safe probe modes `--enumerate`, `--board-info`, `--routes`, and `--passive-route <route>`.

Production boundaries now include:

- `transport-api`: protocol-neutral byte stream plus validated raw classic-CAN frame and read-only source contracts;
- `transport-serial`: Windows/serial device adapter;
- `mongoose-jlr`: transport-independent identity and passive raw-CAN receive subset;
- `transport-replay`: deterministic/real-time machine-readable CAN replay without protocol logic;
- `diagnostic-simulator`: deterministic scripted offline ECU responses emitted as raw CAN;
- `isotp` and `uds`: vehicle-independent protocol core;
- `diagnostics-core`: shared source-to-ISO-TP-to-UDS orchestration;
- `mongoose-jlr-probe`: fixed allowlisted hardware operations only.

There is no public CAN TX, raw hardware command, live diagnostic request, KWP, ECU discovery, signal decoding, firmware, bootloader, security, or programming API. F4 ISO-TP/UDS requests are offline correlation/simulation inputs only. No `cOutboundData` serializer was added.

Validation state:

- implementation: `IMPLEMENTED`;
- static reverse-engineering evidence: `STATICALLY_CONFIRMED`;
- golden-fixture and workspace tests: `FIXTURE_TESTED`;
- vehicle validation: `NOT_VEHICLE_VALIDATED`;
- local `cargo fmt --check`, scoped Rust clippy, compile-only F2 test targets, frontend lint/test/build, and architecture checks pass;
- GitHub `Baseline validation` run `33293377659` passed frontend and Windows Rust jobs for corrective commit `08495c2`, including workspace clippy, workspace tests, the pins 12/13 regression test, and shell check;
- Windows Application Control can block newly built local executables and build scripts (`os error 4551`). On 2026-08-31, supported temporary SAC control allowed the F3 native debug acceptance; SAC was restored to `ON`, and Defender, VBS, Code Integrity, and other controls were not weakened;
- no hardware test is part of CI.

F2B vehicle validation state:

- status: `DEFERRED_BY_OWNER_DECISION`, not failed and not an F2A blocker;
- reason: the available X250 is operationally critical and is not a development test bench;
- physical vehicle test: `NOT_RUN`;
- no F2 Mongoose CAN command has been sent;
- Mongoose was not connected to X250 by this phase work;
- frames received: 0;
- raw F2 vehicle fixtures: none;
- application CAN data TX during F2: 0.

F2A is complete on implementation, static evidence, and fixtures without claiming vehicle confirmation. F2B retains the vehicle criterion: all CAN networks physically accessible through the current MongoosePro JLR backend must be captured; known vehicle networks unsupported by this VCI are documented as `UNSUPPORTED_BY_BACKEND` and are not blockers. F2B is deferred. Pins 12/13 remain prohibited.

## F4 replay, simulation, ISO-TP, and UDS

F4 implements an independent offline path: Replay or Simulator to raw CAN
frames to ISO-TP to UDS to typed diagnostic result. Replay and simulator both
implement the same read-only production CanFrameSource boundary consumed by
diagnostics-core. A future live backend can feed that boundary without
duplicating protocol logic, but F4 adds no live adapter and no transmit API.

Implemented scope:

- classic-CAN standard/extended ID and DLC validation, timestamp, route, source kind, malformed-data rejection, and explicit end of stream;
- schema-versioned JSON replay with deterministic and real-time modes;
- metadata axes for open vehicle program, year range, architecture generation, ECU family, powertrain, variant, market, diagnostic implementation, evidence, and validation;
- strict synthetic/documented/captured fixture separation;
- deterministic simulator behaviors for positive, negative, timeout, multi-frame, malformed sequence, and delayed response;
- ISO-TP SF, FF, CF, FC, segmentation, reassembly, sequence wrap, block size, STmin, timeouts, padding, and length validation;
- generic UDS positive/negative parsing, NRC handling, correlation, raw payload preservation, and typed 0x10, 0x19, 0x22, and 0x3E support;
- simulator and replay golden end-to-end tests through the same production orchestration.

The golden fixture is synthetic and generic. No JLR-specific fixture, Mongoose
device, vehicle, CAN channel, or physical interaction was used. Protocol and
source code contain no vehicle-program branch. The product model covers the
whole Jaguar, Land Rover, and Range Rover SDD era; X250 is only one reference
program. SDD-era KWP, ISO 9141, and other legacy protocols remain possible
future peers rather than being forced into UDS.

Focused local Rust tests passed before Windows Application Control again blocked
a newly rebuilt unsigned test executable with known error 4551. Compilation,
focused clippy with warnings denied, formatting, and architecture checks pass.
GitHub Baseline validation run `33318590434` passed both jobs for implementation
commit `a166033`: frontend lint/tests/build, workspace Rust fmt, clippy, all
workspace tests, both F4 golden E2E paths, and Tauri shell check. F4 therefore
meets all acceptance criteria. See `docs/F4_REPLAY_SIMULATION_ISOTP_UDS.md`.

## F3 application integration

The F3 production vertical slice now connects the adaptive React UI to a small Tauri application service and the existing `transport-serial` / `mongoose-jlr` production path. It dynamically discovers `18E1:0104`, never hardcodes COM3, requires explicit selection for multiple adapters, opens the serial transport only after **Connect**, and requests only allowlisted board info. The UI does not expose capability rows before a verified connection and does not open a CAN channel.

F3 status is `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`. On 2026-08-31, Windows 11 Pro 25H2 build `26200.9278` launched the real native Tauri shell after SAC was temporarily disabled through Windows Security. With no vehicle, the UI passed adapter-absent, `18E1:0104`/dynamic `COM3` discovery, production `0x8109` board-info, disconnect/reconnect, physical unplug fail-closed, and physical replug rediscovery. The 168-byte response is retained with SHA-256 `EF7C4F017856A4954B43239FFC5C0815F08345E8D6410E7EBDF67375457732C6`. SAC was restored to `ON`; no other security control was disabled. See `docs/F3_MONGOOSE_APP_INTEGRATION.md` and `docs/evidence/F3_NATIVE_USB_ACCEPTANCE_2026-08-31.md`.

Vehicle validation remains deferred. No X250 connection, vehicle CAN open/capture, CAN transmission, ISO-TP, UDS, ECU discovery, or diagnostic request is part of F3.

## F3.1 Windows trusted distribution

The exact local blocker is Smart App Control Code Integrity policy `VerifiedAndReputableDesktop`, not a generic Cargo or Tauri failure. It denied unsigned JLR Scanner, Rust test, Cargo build-script, and proc-macro PE files with status `0xc0e90002` / Windows error `4551`. The product executable had Authenticode status `NotSigned`; no matching AppLocker denial was found. See `docs/F3_1_WINDOWS_EXECUTION_AUDIT.md`.

The selected production path changed on 2026-09-01. Microsoft restricts Artifact
Signing Public Trust certificates to organizations in an explicit region list and
to individual developers in the US or Canada. Nothing in this repository
establishes the owner's registered country, so `OWNER_SIGNING_REGION = UNKNOWN`
and Azure Artifact Signing is now `BLOCKED_PENDING_OWNER_REGION_ELIGIBILITY`
rather than selected. The planning baseline is `OPTION_B_OV_CERTIFICATE`: an RSA
OV code-signing certificate from a CA in the Microsoft Trusted Root Program,
which has no region gate. Resolving the region gate is the first reactivation
step because it decides whether the cheaper Azure path is available at all.

The committed pipeline remains Artifact Signing shaped and must be adapted to the
chosen CA before a release build can run. `verify-windows-release.ps1` depends
only on `JLR_EXPECTED_PUBLISHER`, Authenticode, and SignTool, so it survives that
change unmodified. That committed baseline provides:

- manual GitHub Windows release-candidate workflow only;
- pinned Rust `1.98.0`, Node `24.15.0`, pnpm `11.19.0`, and Artifact Signing CLI `0.11.0`;
- Tauri 2 environment-driven custom `signCommand` with no committed credentials or private keys;
- NSIS installer plus standalone application executable;
- fail-closed Authenticode, expected-publisher, timestamp, SHA-256, and SignTool verification;
- no unsigned artifact upload and no GitHub Release publication.

Current F3.1 state is `DEFERRED_EXTERNAL_DEPENDENCY — requires trusted signing identity`, with `OWNER_SIGNING_REGION = UNKNOWN` and `SELECTED_PATH = OPTION_B_OV_CERTIFICATE`. The signing-ready baseline remains committed but inactive. No Azure account/resource, identity verification, billing, signing profile, external account, or certificate was created or purchased, and none is currently required to continue unrelated development. Trusted distribution can be resumed later by explicit owner decision; until then the signing workflow, trusted-builder bootstrap, and external resources remain untouched. See `docs/F3_1_WINDOWS_SIGNING.md`.

Local development is no longer gated on that decision. Disabling Smart App
Control was historically irreversible without a PC reset, but Microsoft shipped a
reversible toggle in the March 2026 update (`KB5079391`, superseded by
`KB5086672`) and the April 2026 security update; the development host on build
`26200.9278` is past both, which is why the 2026-08-31 disable/restore worked and
`VerifiedAndReputablePolicyState` reads `1` today. Toggling per build is still
discouraged. Only `transport-serial` and `jlr-scanner-shell` require Windows, so
`cargo test --workspace --exclude prowlone-shell --exclude transport-serial`
runs every F4–F8 golden test, including the F8 transport-fake Mongoose exchange,
under WSL2 or Linux with Smart App Control uninvolved.

Existing F3 automated validation evidence remains valid: frontend lint/architecture, all 6 UI tests, frontend production build, Rust fmt, workspace clippy, all 46 pre-F4 Rust unit tests, doc tests, and shell compile check passed with pinned Rust `1.98.0`; baseline runs `33306889477` and `33306891173` passed frontend and Windows Rust jobs for commit `af39df4`. The real native USB gate is now `NATIVE_USB_VALIDATED`; no automated suite was repeated because no code changed. `SIGNED_PRODUCT_ARTIFACT = NOT_AVAILABLE` and F3.1 remains deferred. PR #3 remains draft/unmerged; no `v0.4.0-f3` tag exists. F4 does not alter the signing pipeline.
