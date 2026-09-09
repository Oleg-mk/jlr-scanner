# Current State

This file records what is done. `ROADMAP.md` records what is next and why in that
order, including the owner-only gates. Keep both current; neither may live only
in a chat session.

Status: **2026-09-09 — build 0.9.6 is the current one, the first with the bench scenario: commit 4a840c5, tag v0.9.6, run 34350488233, all four jobs green. macOS ProwlOne_0.9.6_universal.dmg, 12.3 MB, SHA-256 16374eb67cadc1aec98e5b465e75c6e4f8a6e96c3cfbaee83af45d8fbcfec5c1; Windows ProwlOne_0.9.6_x64-setup.exe, 4.6 MB, SHA-256 0d044760fa29791ac808c16976b47dd281b40f981c9acfa415ce3b44f1a86b1a; both in Downloads/prowlone-build-0.9.6/. The library did not change, so every issued copy stays valid — the owner's is `BE5E-E564` — and nothing has to be re-issued for this build. Two runs before it were red on `cargo fmt --check`, a step that lived in the CI job and in none of the local checks, which were whatever anyone remembered to type; the tag was moved off the commit that could not build. `scripts/preflight.ps1` is the answer to that: one command that runs everything CI runs — architecture, frontend lint, tests and build, then Rust fmt, clippy and tests for the portable crates and for the shell, both in Docker — and the `pnpm` filters now name a directory instead of a package, so a rename cannot switch the interface checks off in silence again. Earlier the same day — the bench draws its fault codes from the library, by a scenario number the tester chooses (`ADR-0020` amendment). Until now every module reported one or two codes from a fixed list of eight written into the crate: one picture, every time. Now `0` is the healthy vehicle — every module answers, none reports a code, a case the product could not show before — and any other number gives each module none, one or two codes drawn from those the library describes for that module's own family, so every code shown has wording. The seed is the number mixed with the family name: one number always paints one picture, on any machine, from the same library, and the number is in the BENCH band so a tester can quote it. `KnowledgeLibrary::dtc_codes_for` is the new seam. Found while testing it: `pnpm lint`, `pnpm test` and `pnpm build` had filtered the frontend by `@jlr-scanner/frontend` since the rename on 2026-09-09, matched no project, and passed without running anything — in CI too, where the frontend job was green on nothing. Fixed; the 54 interface tests and the lint run again. Earlier the same day — the signature is set aside, the free route included. SignPath Foundation signs open-source projects for nothing, but forbids any proprietary component published by the maintainer, which is precisely what an issued library is; and Microsoft's own comparison of signing options puts an OV certificate, Azure Artifact Signing and EV in one row — reputation accumulates over downloads, first prompts expected, EV's instant bypass gone since 2024 — so for five to ten testers no certificate silences anything. Nothing is applied for, nothing is bought, and `F3_1_WINDOWS_SIGNING.md` holds the three conditions that would reopen it. Whether installers are ever published publicly stays a separate, unanswered question. Same day — build 0.9.5 is the one the club testers receive, and the first to carry the whole day's work: the bench (`ADR-0020`), the product's own name and mark (`ADR-0021`), every one of the library's 1,664 standard fault codes read in Ukrainian and Russian, a module's qualifier reaching the record so its addresses stop colliding, and the engine-variant field that lets a split engine say which half it is. Commit cdf64c1, tag v0.9.5, run 34340565492, all four jobs green. macOS ProwlOne_0.9.5_universal.dmg, 12.9 MB, SHA-256 603bb3a7d665eaa0dbf60a2f45645ec43c11ad30beb81f425ad5687abfad38aa; Windows ProwlOne_0.9.5_x64-setup.exe, 4.8 MB, SHA-256 a743dbd98ec8bc57413a61a7e833d58e2564e78256c6afeea7d515a9cd64fbba; both in Downloads/prowlone-build-0.9.5/. It needs the library re-issued for the qualifier fix: the owner's copy is `BE5E-E564`, and the copy issued to the first tester on 2026-09-06 predates it — the owner decided on 2026-09-09 to leave that tester's copy alone for now. Neither file has been opened by a person yet. Earlier the same day — the qualifier defect is fixed and the library re-exported. The platform adapter now reads a module's own `<qualifier>`: `type` narrows the powertrain, `SubType` the variant, a test naming a market the market, and anything else keeps SDD's own name under `other`; the record id gains a short digest of those tests so two rows for one acronym stop colliding, while an unqualified module keeps the id it always had. The library went from 130,388 records to 131,501, nothing rejected, and the independent cross-check now reports **742 module rows agreeing and nothing disagreeing, in either direction**, with all 3,799 field widths still clean. The owner's copy was re-issued from it: code `BE5E-E564`, valid to 2027-09-09; the previous library is kept as `Downloads/prowlone-library-2026-09-07-before-qualifier-fix`, and the copy issued to the first tester on 2026-09-06 predates the fix and should be replaced before that tester meets a car. What the fix changes for a driver: an X250 of 2010 with the V6 now resolves its instrument cluster at `0x7B2/0x7BA` and its parking-brake module at `0x7B4/0x7BC`, where before both went to the 5.0's addresses and would have looked silent; the survey with the engine stated returns 28 modules reachable on the V6 and 25 with no engine stated, because a module whose address depends on the engine is now refused rather than guessed. That last gap was closed the same day: the catalogue now carries the variants a programme's data declares — for the X250 that is 2L, 4.2L and 5L — and the vehicle card offers them in an **Engine variant** field, shown only for a programme whose data splits an engine at all. With `V8NA` and `4.2L` stated, the X250 of 2010 resolves 28 modules, its cluster at `0x7B2/0x7BA` and its parking brake at `0x7B4/0x7BC`. Earlier the same day — the SDD XML was read a second time, independently, and compared with the exported library (`docs/evidence/xml-crosscheck-2026-09-09.md`). Field widths are clean: all 3,799 byte ranges and masks the library carries are ones the XML declares, so the question of whether we misread a width is closed. Addresses found one real defect of ours: where a platform document declares the same module several times under a `<qualifier>` — an engine type, a build year, a market — with different diagnostic addresses, the ingest writes one record per document and module and only the last address survives. Nine module rows in the corpus do this. It costs the X250 of 2010 with the V6 petrol or the 4.2 V8 the correct instrument-cluster and parking-brake addresses, and L405 and L494 of MY14 built in 2014 the correct ABS address; on the owner's own 5.0 supercharged X250 the surviving address happens to be the right one. The fix is not architectural — `applicability` already carries `powertrain`, `market` and `other`, and the record id needs the qualifier so two rows stop colliding — but it changes the exported library and therefore the issued copies, so it is the owner's call whether it goes into the next build. The check also named a coverage gap that is not a misreading: five platform documents declare programmes (`X101`, `X103`, `X206`, `X356`, `X358`) the ingest does not carry, 51 module rows. Earlier the same day — fault codes read in the interface's language. SDD's text database carries module names and failure-type wording in twelve languages, but a fault code's own description exists in English only and Ukrainian is not among SDD's languages at all, so the wording a Ukrainian or Russian speaker reads has to be ours. It now is, for the codes the standard defines: `crates/diagnostic-session/data/dtc_standard_text.tsv` holds code, Ukrainian, Russian — all 1,664 of them, in nine passes, written from what the standard says each code means, carrying no manufacturer phrasing and therefore safe in a public repository. `dtc_text::standard_texts` is joined into `DtcDescription`, travels to the interface as `DtcSummary::description_texts`, and the read panel shows our wording with the loaded data's English kept underneath it. The report is untouched and stays English, because a report is evidence and evidence does not change language. A manufacturer-specific code gets nothing from us and falls back to the library's English, which is where that wording belongs. Checked in the preview in all three languages, and by 50 interface tests and 58 Rust suites. Not in a build yet: the owner asked for changes to be batched, so this waits for the next one. What is left is not the standard codes — 1,664 are in the library, and every one of them is now covered — and, separately, whether the manufacturer-specific wording is ever translated inside an issued library. Earlier — build 0.9.4 opened on the owner's Windows 11 desktop: from the desktop shortcut, by double-click and by right-click. It is the first ProwlOne build a person has run. What it took, in order: the owner switched Smart App Control off (the registry value still read 1 afterwards; no block was logged for the launch); a silent NSIS install (`/S`) put 0.9.4 in place, because the interactive installer insists on running the previous unsigned uninstaller first and Smart App Control blocks that; both stale installs had been removed by hand, folder, shortcuts, the `Uninstall` entry and the NSIS key `HKCU\Software\prowlone`, which alone makes the installer say "Already Installed"; and the shortcut showed Windows's placeholder until the icon cache was refreshed, while the executable carried the right icon all along, checked by extracting it. The steps are in `OWNER_GUIDE.uk.md` section 9. The day also showed that a new unsigned build restarts the block every time, so builds are to be batched, not made per change. Earlier — Smart App Control blocks the new unsigned build on the owner's Windows 11 desktop (25H2, build 26200.9278): `Microsoft-Windows-CodeIntegrity/Operational` event 3077 against `prowlone-shell.exe` at 09:25, 09:34 and 09:37 and against `uninstall.exe` at 09:38, policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, `VerifiedAndReputablePolicyState = 1`. The first reading of this — that Smart App Control had just switched from evaluation to enforcement — was wrong, and the log says so: it holds 110 block events from 2026-08-29 onwards and exactly one audit event, so enforcement is not new. What the same log shows is stranger and is the fact worth keeping: the previous binary `jlr-scanner-shell.exe` was blocked on 29 and 30 August, on 1 September and on 6 September, and then ran on this machine unblocked, including this morning, while the newly built `prowlone-shell.exe` is blocked from its first launch. The pattern is per file, and a block seems to lapse for a file that has been on the machine a while; the mechanism behind that is not established here and should not be guessed at. The application cannot start there and cannot even be uninstalled by its own uninstaller, because that is unsigned too; both stale installs (JLR Scanner 0.9.2, ProwlOne 0.9.3) were removed by hand — folders under `%LOCALAPPDATA%`, the two Start-menu and desktop shortcuts, and the two `HKCU` uninstall entries. Nothing else on the machine was touched, and Smart App Control was left on. This is not a defect of the build: the same wall waits for any tester whose Windows 11 has it on, and it is the first hard evidence for the `F3.1` signing decision, which until now was deferred until vehicle results. The owner's Windows 10 laptop and the macOS build remain open paths. Build 0.9.4 is the current one — the product's own name and the mark the owner chose (`ADR-0021`): commit 3236785, tag v0.9.4, run 34319517717, all four jobs green. macOS ProwlOne_0.9.4_universal.dmg, 12.7 MB, SHA-256 199662eaa3e40f77e49bc485cccceadd72d39251e9c08992d7a48f41883a9b34; Windows ProwlOne_0.9.4_x64-setup.exe, 4.7 MB, SHA-256 4179d9918330791b928561aa89b70079888325659805bf7058e01c5be16a6f7e; both in Downloads/prowlone-build-0.9.4/. Build 0.9.3 was superseded before it reached anyone, so its number is spent and its files should be discarded: it carried the illustrated icon the owner judged cheap at desktop size. Earlier the same day — 0.9.3 was the first under the product's own name and mark, ProwlOne (`ADR-0021`): commit b13086d, tag v0.9.3, run 34317723468, all four jobs green, so the renamed crate, the renamed macOS binary inside the bundle and the new icon set all survive a real build. macOS ProwlOne_0.9.3_universal.dmg, 14.6 MB, SHA-256 44ba66e56233049bd3b4a873690ce233283762d5223b0e1c22faf6ec8215217f; Windows ProwlOne_0.9.3_x64-setup.exe, 4.9 MB, SHA-256 8949bec5e97eff0e4f5306024f805761c6206fc308a62ce349c258dd00dd4ae5; both in Downloads/prowlone-build-0.9.3/. This is the build the club testers receive; the bundle identifier changed with the name, so it installs beside an older build rather than over it, and the older one should be uninstalled. Neither file has yet been opened by a person. Earlier the same day — build 0.9.2 was the bench build, the one the club testers receive: commit 89a3725, tag v0.9.2, run 34313556981, all four jobs green; the footer reads 0.9.2 · 89a3725. macOS JLR Scanner_0.9.2_universal.dmg, 12.5 MB, SHA-256 d5a272d23f4e14c2d00d3b79213a9c4f6f076aa4b581e01b39621f58a9eb8e98; Windows JLR Scanner_0.9.2_x64-setup.exe, 4.7 MB, SHA-256 de8e67952c677434ad6717667be71f15f9c973f81ea482e034f5b746c594e816; both in Downloads/jlr-scanner-build-0.9.2/. Neither has yet been run by a person; the bench commit before it (8fad2f5) was green on the same four jobs, including the shell's end-to-end bench test under Windows. Same day, earlier — the bench is built (ADR-0020, `IMPLEMENTED / FIXTURE_TESTED`): a virtual vehicle answering on the adapter's own protocol — `mongoose_jlr::bench` speaks the MongoosePro framing below the whole real stack, `bench-vehicle` answers from the loaded library for whatever vehicle the session surveyed, the adapter panel offers «connect the bench» beside «detect adapter», a session is bench or real and never both, nothing is written to disk in bench mode (the report is shown on screen), every bench value is marked `SYNTHETIC` and `report-intake` refuses a bench bundle anyway, and the whole screen turns to a warm Namib Orange tint with a BENCH band top and bottom while it is connected; the shell's end-to-end test drives connect, survey, reads, listen, bundle and refusals on every commit. It is part of the tester programme (recruitment, onboarding, regression) and never reaches VEHICLE_CONFIRMED. Earlier the same day it had only been decided. 2026-09-08 — builds are numbered from here: version 0.9.N, where N is the number testers quote and grows only when a build is handed out; the footer shows version and commit (0.9.1 · abc1234) and every session report carries application_version and application_build; scripts/bump-version.mjs sets the number in the five files that hold it and prints the commit and tag to make. Retroactively, 0.9.0 is the 417d9d1 build the first tester holds; 0.9.1 is the first numbered build: commit a5e86c8, tag v0.9.1, run 34196375573, all four jobs green; the footer reads 0.9.1 · a5e86c8. macOS JLR Scanner_0.9.1_universal.dmg, 13.2 MB, SHA-256 f84936a3824685cee3be0fa82624614cc0cb9c363a6a047fe9c7b321d6bece86; Windows JLR Scanner_0.9.1_x64-setup.exe, 4.7 MB, SHA-256 61481f75e38bf5b7dcad47014102a555783e9e2b9bcfb61bb1a64439ce333094; both in Downloads/jlr-scanner-build-0.9.1/. This is the build the club testers receive with their signed library copies. 2026-09-08 — bench: the diagnostic open of resource 21 (pins 3/11) and the application's outbound record are accepted by the firmware exactly as on resource 5, so the medium-speed read path is `HARDWARE_CONFIRMED` on the bench to the same extent as the high-speed one; the jump from the bootloader measured at 241 ms; a record whose header word after the sequence is 0 draws silence, the application's 1 draws the answer (evidence Part 4). 2026-09-07 — medium-speed diagnostics analysed offline, no code changed (`F10`, dated section): on the SDD-era cars `CAN_MS` is bound from SDD's own platform documents and ready — X250 MY10 carries 14 medium-speed modules with request and response identifiers and reachable fault-code reads, L319 MY10 14, L322 MY07 7 — while on the 2014-and-later cars the medium-speed half is still the `ADR-0015` hypothesis, 25 of L405 MY14's modules and 22 of L494 MY16's. What was called the weak point was reworded on 2026-09-09 at the owner's insistence, and he was right: for the Ford-derived SDD-era cars the binding of `CAN_MS` to pins 3/11 at 125 kbit/s is known from the architecture, from the adapter's own JLR pinout, and from what third-party tooling does on these cars every day, and the adapter path is `HARDWARE_CONFIRMED` on the bench; what is missing is a capture from a car, which is a signature rather than a doubt, and it is now the first step of the tester guide. The genuinely open question is the 2014-and-later gateway. The gatewayed sub-networks `SUB_MOST`, `SUB_CAN1`, `NGI` and L322's `DS2` are a separate, stage-2 problem and should not be counted as medium-speed. Also 2026-09-07: a survey of X350 returns nothing because ten legacy Jaguar platform documents — X100, X202, X350 and X400, 216 modules — sit in the SDD component `CURRENT_JLR_MCP_XML_XML`, which no export has ever walked (`F9`, dated section); **they were ingested the same day**: the exporter now walks that component too, `parse_marker` accepts SDD's dotted mid-year fraction (`MY02.5`) beside the underscored one, and the library grew from 45 platform manifests to 55, 18 programmes to 22, 129,441 records to 130,388, nothing rejected and no failed manifest. `PLATFORM_X400.xml` and `PLATFORM_X404.xml` were not the collision they looked like: X404 is the fuller revision of the same programme-year and the two merge into the union of 19 module families. Ingesting adds knowledge rather than reach, because those cars are pre-UDS: of the four new programmes only the XJ of 2006 and 2008 gains anything usable today, 5 and 6 modules on `CAN_HS` including `PCM` at `0x7E0/0x7E8`. The library folder was replaced and the previous one kept as `Downloads/prowlone-library-2026-09-06-xcl-only`; the owner's own copy was re-issued from the new base (code `0EDF-FCBF`, valid to 2027-09-07), while the copy issued to the first tester on 2026-09-06 (`A2CC-E236`) stays valid and simply lacks the four programmes. 2026-09-06 — published: this repository, github.com/Oleg-mk/jlr-scanner, is the public continuation under AGPL-3.0 of a private archive (jlr-scanner-archive, frozen at its commit c32f8d7) whose commit hashes the older entries below still cite. Same day, earlier — after the first live test failed at channel open, the live CAN path was rebuilt on the firmware's real handshake and resource routes (ADR-0018): jump from the bootloader, resource 5 for `hs-can`, resource 21 for `ms-can`, both listen sequences `HARDWARE_CONFIRMED` on the bench; the next CI build is the first that can meet a car. 2026-09-05 — reports are saved and the library folder is chosen through the shell's native dialogs (F11 slice 10); by owner decision the tester programme runs on the unsigned CI build, with `TESTER_GUIDE.md` as the hand-out. The branch has not been pushed, so no installer of this state exists yet; nothing here has met a Windows build or a vehicle. Also 2026-09-05: the medium-speed buses of the 2014-and-later cars are hypothesised through `ms-can` on pins 3/11, on the owner's community statement (ADR-0015 addendum) — 716 module rows now sit on hypothesised routes, 14 remain on unbound sub-networks.** M3 derived `normal_fixed` identifiers are `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_SURVEYED` (ADR-0017): 73 physically addressed modules of L322 MY04.5–07, L319 MY05 and L320 MY06 now carry derived 29-bit identifiers, `Unverified` until a tester's read confirms them; the live path sends 29-bit frames (transport-fake tested, not hardware-confirmed). F13 report intake (milestone M2) is `IMPLEMENTED / FIXTURE_TESTED`: a tester's session report becomes one `Captured` manifest whose records confirm, for that vehicle, what a module's answer proved — addressing, bus, physical route, protocol, capability — and record silence, captures and calibration reads as observations; loaded with the library, a confirmed route turns from hypothesis to `CAPTURE_VALIDATED` in the survey (ADR-0016). No report from a vehicle exists yet. F11 Tester Application (milestone M1) is `IMPLEMENTED / FIXTURE_TESTED`: the vehicle picker, VIN decoding with SDD's own tables, fault-code wording join, identifier value decoding with named states, module names from SDD's text database, guided session flow, single session report, the SDD-style vehicle network map with a read-only check of every module, and an interface in English, Russian and Ukrainian with SDD's data text following into Russian (the data-derived parts also `REAL_SOURCE_SURVEYED`); the map-converter scale stays open, and nothing has met a vehicle. F10 Multi-Module Read-Only Diagnostics is `IN_PROGRESS`; enumeration, family-level resolution, offline UDS read execution and the composition root are `IMPLEMENTED / FIXTURE_TESTED`, and the vehicle survey is `REAL_SOURCE_SURVEYED` against the exported SDD corpus (offline). F9 SDD Knowledge Ingestion has fully ingested the surveyed SDD corpus; its addressing, DID-catalogue, DTC, ODST, DTC-index, and platform slices are `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`. F8 Tester-Ready Mongoose Alpha Candidate is `TESTER_ALPHA_CANDIDATE_READY / IMPLEMENTED / SIMULATOR_TESTED / REPLAY_TESTED / TRANSPORT_FAKE_TESTED / NO_VEHICLE_INTERACTION`. Live vehicle status is `NOT_YET_EXTERNALLY_VALIDATED`. Distribution is `READY_FOR_SIGNING / DISTRIBUTION_BLOCKED_BY_F3.1`. F7, F6, F5, and F4 remain PASS. F3 is `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`; F3.1: gate `G2` decided 2026-09-03 (Ukraine, individual entrepreneur); the certificate is deferred until real vehicle results, so distribution stays `DISTRIBUTION_BLOCKED_BY_F3.1`; until then the unsigned CI build runs on the owner's own hardware only. Gate `G4` decided the same day: the SDD-derived library goes to named testers only, never inside the installer.**

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
# re-export the library from the eight corpus roots (about 100 s)
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads/prowlone-library:/library" -v "C:/Users/<you>/Downloads/SDD169_XML:/corpus_d" -v "C:/Users/<you>/Downloads/SDD169_PAYLOAD/SDD_XML_PAYLOAD:/corpus_e" rust:1.98-slim sh -c 'cargo run --release -p sdd-ingest --example export_manifests -- /library /corpus_d/CURRENT_JLR_XCL_XML_DATA_XML /corpus_d/CURRENT_JLR_MCP_XML_XML /corpus_d/CURRENT_JLR_VIN_DECODE_XML /corpus_e/COMMON_SDD_DATA_SNAPSHOT_LANG_EN /corpus_e/COMMON_SDD_DATA_DTC_HELP_LANG_EN /corpus_e/COMMON_SDD_DATA_ODST_LANG_EN /corpus_e/CURRENT_PAG_UTILS_RUNTIME /corpus_e/CURRENT_PAG_MCP_TEXT_XML'
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
