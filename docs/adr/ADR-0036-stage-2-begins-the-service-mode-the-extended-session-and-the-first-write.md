# ADR-0036: Stage 2 begins — the service mode, the extended session, and the first write

- Status: **Accepted, 2026-09-18** — drafted for the owner's review the same
  day and accepted on his word after reading it («ок, коміть»). The first
  commit of stage 2 cites it.
- Decision: stage 2 of `ROADMAP.md` — service functions — is authorised by
  the owner ("переходимо до 1.2 кроку", 2026-09-18) and is built in four
  steps, one operation class at a time, in the order of the smallest blast
  radius first: **clear the fault codes** (1.2.0), **run a routine** (1.2.1),
  **drive an actuator** (1.2.2), **change an adaptation** (1.2.3). Every
  operation exists in the interface only inside a **service mode** the
  person switches on for the session, asks for its own confirmation, runs
  in the **extended diagnostic session** the data requires and leaves it,
  and is written into the report with what it changed. Each step loosens
  the architecture guard for exactly its service, in the commit that adds
  the operation, and amends this record with what was built.
- Reason: stage 1 is built and its theory closed on our side (the owner,
  2026-09-17: "з нашого боку закрита умовна теорія"); `ADR-0035` put the
  other half of SDD's module catalogue into the library and drew every
  operation into its class without sending one. What separates the product
  from SDD's service functions is now mechanics and consent, not knowledge,
  and both can be built at the desk and run in on the bench.
- Consequence: the product gains its first operations that change what a
  module holds. `SAFETY_BOUNDARIES.md` gains a row per operation, the
  `uds`, `ds2` and `kwp2000` crates gain the constructors they were kept
  without, `uds-execution` gains a prepared **service** transaction beside
  the read-only one, the live path takes only that, the bench answers the
  new services, and the version turns 1.2.x — installers that go to the
  owner alone.

## Context

What the library says (`ADR-0035`, measured 2026-09-16): 2,294 writeable
identifiers, 124 controllable, 8,095 routine records over 253 distinct
numbers. Every writeable and controllable identifier names `session_03`,
the extended diagnostic session; 33 writeable ones also allow the default
session; nothing is writeable or controllable in the default session alone.
628 writeable identifiers, 24 controllable and part of the routines stand
behind a security level. Routine `0x0202`, the on-demand self test that
`ADR-0032` lists with SDD's own words and timings, appears 1,528 times;
`0x0404`, VIN Learn, 1,308 times.

What the code says today. `crates/uds` constructs `0x10`
DiagnosticSessionControl and `0x3E` TesterPresent already — for the
simulator and the tests — and constructs no `0x14`, `0x31`, `0x2F` or
`0x2E`. `crates/ds2` has no clear-fault request (`0x05`) and
`crates/kwp2000` no `0x14`. `crates/uds-execution` knows one intent,
`ReadOnlyUdsIntent`, and one class, `TransactionSafetyClass::ReadOnly`.
`scripts/check-architecture.mjs` fails the build when the Mongoose live UDS
path names `diagnostic_session_control`, `tester_present`, `RoutineControl`,
`WriteDataByIdentifier`, `SecurityAccess` or `EcuReset`; when the K-line
paths name `clear_fault`, `start_diagnostic_session`, `routine_control` or
`security_access`; and when the interface carries a field for a CAN
identifier, a service or a DID. `SAFETY_BOUNDARIES.md` says an operation
not in its table does not exist, and adding one means adding it there with
its class in the same change.

What the owner decided on 2026-09-18, on the plan put to him:

- the class of clearing the fault codes is `SERVICE_ROUTINE`;
- the 1.2.x builds go to him alone, as a separate installer, and he uses
  one only after his own tests of the 1.1 line;
- the wording and the term of the service mode's consent are left to this
  record ("ти такий автор як і я");
- clearing codes over K-line belongs to the first step, not a later one.

## Decision

1. **Stage 2 is authorised and staged.** Four steps, four versions:

   | step | version | operation | service | class |
   | --- | --- | --- | --- | --- |
   | 1 | 1.2.0 | clear the fault codes | UDS `0x14`; DS2 `0x05`; KWP2000 `0x14` | `SERVICE_ROUTINE` |
   | 2 | 1.2.1 | run a routine, the self tests first | UDS `0x31` | `SERVICE_ROUTINE` |
   | 3 | 1.2.2 | drive an actuator, and give it back | UDS `0x2F` | `VOLATILE_CONTROL` |
   | 4 | 1.2.3 | change an adaptation; named resets where SDD's procedure is known | UDS `0x2E`, `0x31` | `PERSISTENT_CHANGE` |

   Each step is one operation class entering the product, with its row in
   `SAFETY_BOUNDARIES.md`, its record in the bundle, its answer on the
   bench and its amendment here. A step does not start before the one
   before it is on the bench end to end.

2. **The service mode.** Stage-2 operations exist in the interface only
   while a switch the person turned on for this session is on. The switch
   is off on every start and on every new session; it is never remembered.
   Turning it on shows one consent text and takes one acknowledgement; while
   it is on, a band in the header and the footer says so in the interface
   language, the way the bench band does, so the state of the application
   is never hidden. The consent, in English (Ukrainian and Russian follow
   the same words):

   > *Service mode lets this application change what a module holds:
   > clear its fault codes, run its routines, drive its outputs, change its
   > adaptations. Each operation will ask you again, one at a time, and is
   > written into the session report with what it changed. This application
   > undoes nothing by itself. The car stands still, the ignition is on and
   > the engine is off unless a procedure says otherwise. What follows is
   > your decision for the car in front of you.*

   Once per session, not once per launch: a session is one car or the
   bench, and the consent is about that car. The bench allows the mode; on
   the bench every operation is answered, recorded, marked `SYNTHETIC`, and
   writes nothing to disk (`ADR-0020`).

3. **One confirmation per operation.** Before anything is sent the
   interface states the module (its mnemonic and SDD's name), the operation
   by name, its class, what will change, the preconditions the data carries
   in SDD's own words where it has them, and what to expect afterwards. The
   confirming control names the operation ("Clear the codes of PCM") and
   never holds the focus by default; cancelling is the plain way out. The
   one collective confirmation is "clear the codes of every surveyed
   module", which lists the modules it will ask, and even that asks each
   module in turn and records each answer.

4. **The extended session is opened, held, and left.** An operation whose
   data requires `session_03` opens it on its module with `0x10 03`, keeps
   it with `0x3E` TesterPresent while the operation runs, and returns the
   module to the default session with `0x10 01` when the operation ends,
   fails, or is stopped. A session is never left open after an operation,
   and never opened for a read. The clear over UDS is tried in the default
   session first, as ISO 14229 allows, and repeated in the extended session
   only on `serviceNotSupportedInActiveSession`.

5. **The record.** Each operation kind is a field of the session bundle
   with its own schema id, operation name and class, as the reads are:
   `dtc_clears` / `prowlone.dtc-clear` / `DTC_CLEAR` first. A row carries
   the module, the operation, the class, the answer (positive, or the
   negative response code by name), the time, and what changed — for a
   clear, the codes read before and the codes that answered after. The
   interface's raw bytes never enter a row: the request is built from the
   plan as every read is. `report-intake` accepts the new kinds and the
   readable report gains a section, "Service operations", with the worth of
   each row said beside it.

6. **The guard is loosened one service at a time, visibly.** The commit
   that adds an operation rewrites the rule that forbade its constructor
   into a rule that requires it: the live path may send the service only as
   a `PreparedUdsService` — a new type in `uds-execution` beside
   `PreparedUdsTransaction`, carrying a `TransactionSafetyClass` other than
   `ReadOnly`, a target resolved from the plan, and the session the data
   requires — and the K-line path only as its own prepared service. The
   rules that keep the interface from naming a CAN identifier, a service or
   a DID stay. The rules on `knowledge` and `sdd-ingest` stay. `SecurityAccess`,
   `RequestDownload`, `TransferData` and `EcuReset` stay forbidden
   everywhere: the first three are `FORBIDDEN_PROGRAMMING`, and a reset can
   leave a module in a state this product cannot bring it back from.

7. **What is behind security stays behind it.** JLR's seed-and-key
   algorithms are JLR's; this project does not take them and does not
   reverse them. The 628 writeable identifiers, 24 controllable ones and
   the routines that name a security level are listed, as `ADR-0035`
   lists them, as "behind a security level — not available", in every
   step of stage 2. This is the one wall stage 2 does not move.

8. **Prudence about the first car.** No operation of this product has met a
   vehicle. The first write on a car is the owner's, on a car of his
   choosing, after at least one read session on a real car has been
   reported (M6). 1.2.x installers are built by the same CI as every build
   and are not handed to testers; the testers' line is `release-1.1`,
   branched from `v1.1.0` on 2026-09-18, where a 1.1.1 can be made if they
   need one.

9. **Step 1, the clear, in full.** The operation is offered for a module
   whose fault codes were read in this session — the codes are then already
   in the bundle, and clearing erases nothing the report does not keep.
   UDS: `0x14` with the group `0xFFFFFF`, all codes. DS2: the clear-fault
   request `0x05`, in the framing `ADR-0029` already speaks. KWP2000:
   `0x14`. After a positive answer the module's codes are read again and
   shown, so what returned at once is seen for what it is — a fault that is
   present, not a fault that was missed. A negative answer is shown by its
   name and recorded. On the bench a module's fault list empties for the
   rest of the session, and the scenario's faults return with the next
   connection, so the same picture can be shown twice.

10. **Steps 2 to 4 in outline**, each to be detailed in its amendment
    before its code:

    - *Routines.* The on-demand self tests first: `0x31` start, stop and
      request-results for routine `0x0202` on the modules that declare it,
      with SDD's timings and instructions on the confirmation. A result is
      shown as the bytes it is, with the reason, until the pack that
      describes results is read (`ADR-0032` left that undecided). Routines
      that name a security level, `0x0404` VIN Learn, and the routines
      that open a gateway (MOST, SUB_CAN1, NGI) are excluded; the gateways
      are their own decision, because they change the network.
    - *Actuators.* The 100 controllable identifiers without a security
      level, with the control parameters the data names for each. Control is
      returned to the module on stop, on timeout, and when the session ends,
      without asking. The same identifier is read alongside (F15) so the
      effect is seen. The bench reflects a command in its next readings.
    - *Adaptations.* Two halves. A write to one of the 1,666 writeable
      identifiers without a security level, in the engineering level
      `SAFETY_BOUNDARIES.md` already names: the identifier is read first and
      its original kept in the record, the new value is typed in the
      identifier's own encoding and shown decoded before it is sent, and
      reverting to the original is offered. Named procedures — service
      interval, DPF, brake bleed, EPB, TPMS, steering angle — only where
      SDD's own procedure has been read into a validated sequence with its
      preconditions; those procedures are not in the library today, so that
      is a knowledge step first, and until it is done there are no such
      buttons. Fail closed.

## What this does not decide

- It does not touch `ADR-0005`: firmware, VBF, bootloader, keys, immobiliser
  and security programming stay outside the product. Stage 3 is not
  opened by this record.
- It does not claim any operation works on a car. A declaration in SDD's
  catalogue is `documented` evidence; a module's answer on a car is
  `VEHICLE_CONFIRMED`, and no car has answered anything yet. The bench proves
  the software, never the car (`ADR-0020`).
- It does not decide the results format of the self tests, the gateway
  routines, or any named service procedure. Each returns here as an
  amendment with its own evidence.

## Consequences

The first commit of stage 2, in one piece, is step 1:

- `crates/uds`: `UdsRequest::clear_diagnostic_information(group)`, the
  typed positive response `0x54`, the negative codes by name.
- `crates/ds2`, `crates/kwp2000`: the clear-fault request and its answer,
  and nothing else new.
- `crates/uds-execution`, `crates/kline-execution`: `ServiceIntent`,
  `TransactionSafetyClass::ServiceRoutine`, `PreparedUdsService` and
  `PreparedKlineService`, prepared from the same plan the reads use, with
  the session the data requires.
- `crates/mongoose-jlr`: `execute_prepared_uds_service` and its K-line
  twin, holding the session as decision 4 says; the guard's rules on
  `uds_live.rs` and the K-line files rewritten from "never names it" to
  "takes it only prepared".
- `apps/scanner/src-tauri`: the service mode as a property of the session,
  a `dtc_clear_service`, the commands to switch the mode and to clear, the
  `dtc_clears` field of the bundle, the report's section.
- `crates/report-intake`: accepts `dtc_clears`.
- `crates/bench-vehicle`: answers `0x10`, `0x3E`, `0x14`, DS2 `0x05`,
  KWP2000 `0x14`; a clear empties the module's faults for the session.
- The interface: the switch and its consent, the band, the button under a
  module's fault codes while the mode is on, the confirmation, the result
  and the re-read, the "clear every surveyed module" sequence, the report
  section — in three languages.
- `SAFETY_BOUNDARIES.md`: `SERVICE_ROUTINE` gains its first operation;
  `ROADMAP.md` F12 is written as built; `CURRENT_STATE.md`;
  `CAPABILITY_MATRIX.md` row "Clearing codes" becomes built; this record
  gains its first amendment; the version turns 1.2.0.
- Tests: the constructors and answers in the crates; the bench end to end —
  mode on, codes read, confirmation, clear, re-read, the record marked
  `SYNTHETIC`, the file refused; the interface — nothing of stage 2 visible
  with the mode off, the consent once, the confirmation naming the module,
  the guard on every commit.

## Precedent

`ADR-0012` made a read a prepared transaction the interface cannot forge;
`ADR-0020` made the bench answer the software before any car; `ADR-0022`
made a repeated read record every sample; `ADR-0035` listed what a module
accepts and sent none of it. Stage 2 is those four rules applied to a
write: prepared, run in on the bench, recorded, and listed before it is
sent.

## Amendment, 2026-09-18: step 1 built

Built the day the record was accepted, on the branch `stage-2`, so the
owner's running window was not restarted under him. What the decision said
and what the code does met in these places:

1. **The session (decision 4).** The clear is short enough to need no
   TesterPresent: the whole sequence — the clear, and where the module asks
   the session opened, the clear again, the way back — runs well inside the
   five seconds a module keeps a session without one. The keep-alive is
   therefore not sent for the clear; a routine that runs longer (step 2)
   adds it. The way back is sent whatever the clear answered, and a module
   that fell silent on the way back has left the session on its own timer.
2. **The K-line (decision 9).** DS2 `0x05` and KWP2000 `0x14` exist, are
   prepared on the same line the fault-memory read uses, and the bench
   answers both; the end-to-end test proves the UDS path on the bench, the
   K-line path is proved at the crate level. A serial protocol has no
   session to open.
3. **The collective confirmation (decision 3)** — "clear the codes of every
   surveyed module" — is not in this commit. The per-module clear is
   complete; the sequence over the surveyed modules, with its one
   confirmation listing them, follows in a later commit of the step.
4. **The record (decision 5).** `dtc_clears` carries the read that found the
   codes (`before`) and the read made after (`after`) whole, beside the
   codes themselves, the answer, the session and every exchange.
   `report-intake` takes the two reads as reads under
   `dtc_clears[n].before` and `.after`, and the clear itself as nothing: an
   action is not evidence about a route.
5. **The guard (decision 6).** Rewritten one rule at a time: the live UDS
   read's body is read and may name no session control; the service reaches
   the adapter only as a `PreparedUdsService`; the DS2 and KWP2000 crates
   have three and four constructors; `ServiceUdsIntent` has one variant and
   `ServiceKlineIntent` two; `SecurityAccess`, `RequestDownload`,
   `TransferData`, `InputOutputControl` and `EcuReset` stay forbidden in the
   live path.
6. **The bench.** Half its modules, by the parity of their family's hash,
   refuse the clear in the default session with `0x7F 0x14 0x7F` and accept
   it in the extended one, so both paths of decision 4 run on every commit;
   a cleared module holds no codes for the rest of the session, and the
   scenario's codes return with the next connection.
7. **Beside the step:** a Windows checkout gave the shell scripts carriage
   returns and the Linux container refused them; `.gitattributes` now pins
   `*.sh` to LF.
8. **The bands (decision 2) are withdrawn.** Built as decided, seen on the
   owner's screen, and refused the same day: «дві червоні полоси зверху і
   знизу роблять абсурд». The state of the mode is said by the switch
   itself, red while the mode is on, and by the lamp in the corner, red
   with it. Nothing else about decision 2 changes: one consent, off on every
   start and every new session, never remembered.

The version turns 1.2.0 with the owner's word, and the installer goes to him
alone (decision 8).

## Amendment, 2026-09-19: the collective clear, and step 1 accepted at the screen

- The owner walked the service mode and the clear in the running window and
  accepted the flow he had judged illogical the evening before: «ми
  виправили всі мої зауваження, все тепер логічно». Then, on his word
  («роби»), the one thing the built note had left for a later commit.
- **The collective confirmation (decision 3) is built.** Under the list of
  what the car answered — the modules the check found codes in — the mode
  offers "Clear the codes of all *n* modules". The one question lists every
  module it will ask with its name and its count of codes, says how many
  codes will be erased from how many modules, repeats the low-battery line
  where there is one (`ADR-0030`), and the confirming control names the
  operation and the count. Then each module is asked in turn, in the order
  of the list, and each answer is its own record — the shell knows no
  sequence, only clears, and `dtc_clears` grows by one per module. A
  refusal or a failure of one module does not stop the rest; what each came
  to is said under the list when the sequence ends, and a module whose codes
  returned at once stays on the list, because the list follows the read
  made after the clear. Tested in the interface over two modules, and on
  the bench in the whole-stack test, which now clears two modules in turn
  — one over CAN, one over the K-line — and finds two records.
- **The K-line clear is on the bench end to end.** The built note of
  2026-09-18 had it at the crate level; the whole-stack test now reads
  `DS2MOD`'s fault memory over the serial line, sends DS2 `0x05` down the
  same line, reads the memory again empty, and records the clear with no
  session, as a serial protocol has none. KWP2000 `0x14` stays at the
  crate level: the synthetic fixture has no KWP2000 module.
- Nothing else of the step moves: the per-module clear, the session, the
  guard and the record are as the amendment of 2026-09-18 says; no
  operation has met a car. With this the step is on the bench end to end,
  which decision 1 asks of a step before the next begins; the version
  turns 1.2.0 with the owner's word, given the same day.
