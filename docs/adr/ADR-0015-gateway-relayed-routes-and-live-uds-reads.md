# ADR-0015: Gateway-relayed route hypotheses and live UDS reads

- Status: Accepted
- Decision: Two things, decided together because the second is what turns the
  first from a hypothesis into evidence.

  **1. A bus may be bound to an adapter route as a hypothesis.** The 2014-and-
  later architectures name five buses per vehicle and declare no gateway for
  any of them, while the adapter has two CAN pairs. Which pair carries which
  bus has no citable source. For the high-speed buses `PT_HSCAN`, `CH_HSCAN`,
  `HY_HSCAN` and `CO_HSCAN` a binding to route `hs-can` is recorded as a
  **research hypothesis**: an F5 source of type `UnverifiedResearch`, evidence
  of class `UnverifiedResearch`, records in validation state `Unverified`,
  built into the application beside the documented bindings. The medium-speed
  buses `BO_MSCAN` and `CO_MSCAN` stay unbound, because two hypotheses compete
  and recording either would be a guess.

  The survey shows a module whose route rests on such a record as a
  **hypothesis**, a status of its own beside reachable and unreachable, and
  the plan carries `Unverified`. Nothing hides that it is a guess.

  **2. A plan carries the route's rate, not the bus's.** The per-module bus
  claim the platform adapter records under `ADR-0013` keeps the bus name and
  drops the bit rate; the bus's own rate stays on the per-programme network
  record where SDD states it. The route binding supplies the rate the adapter
  opens, which for a direct binding equals the bus rate and for a relayed one
  is the diagnostic CAN's. This amends `ADR-0013` §2.

  **3. `mongoose-jlr` executes a prepared UDS read live.**
  `execute_prepared_uds_read` accepts only a `PreparedUdsTransaction` from
  `uds-execution`, exactly as the F8 path accepts only a
  `PreparedDiagnosticTransaction`; the architecture checker enforces the
  signature. It validates the transaction against the route descriptor, opens
  the route for transmission, sends the single-frame request padded to eight
  bytes, answers a First Frame with one Flow Control, waits through
  ResponsePending, reassembles, closes the route, and returns the raw
  response. Decoding lives in `uds-execution` in one function shared by the
  offline and live paths. Only `normal` 11-bit addressing is executed now.

  This amends `ADR-0012`'s consequence that nothing but the composition root
  depends on `uds-execution`: the backend depends on it for the transaction
  type, as it already depends on `diagnostic-execution` for the same reason.

- Reason: The fleet coverage measurement put 728 modules — every 2014-and-
  later programme — behind one unanswered question, and the passive capture
  slice established that listening cannot answer it. The only things that can
  are a document nobody could produce, or a read-only request that a module
  either answers or does not. The project's own validation model, decided in
  F5 and the tester programme, is built for exactly this: an
  `UnverifiedResearch` record that a `Captured` response later corroborates.
  Refusing to record the hypothesis would not make it unknown; it would keep
  the question from ever being asked of a car.

  Why hypothesise `hs-can` for the high-speed buses and nothing for the
  medium-speed ones: ISO 15765-4 puts the legislated diagnostic bus on J1962
  pins 6 and 14, SDD places the PCM on `PT_HSCAN`, and SDD addresses the
  modules of every one of these buses directly, so a gateway must relay the
  tester's frames from that pair. That reasoning reaches the high-speed buses.
  For the medium-speed buses the relay could equally run through the second
  pair; nothing distinguishes the two, so neither is recorded.

  Why the route's rate: a body module on a 125 kbit/s bus reached through a
  500 kbit/s diagnostic CAN must be opened at 500 kbit/s. With both rates in
  the plan's candidate set the resolver reported a conflict, correctly, since
  the two facts answer different questions. Separating them is a model
  correction, not a relaxation.

  Why the live path pads: ISO 15765-4 requires eight-byte frames on the
  legislated bus and JLR modules are known to be strict about it; the F8 path
  sends the shortest frame and has never been vehicle-validated. Padding with
  zero is the J2534 default SDD itself runs on.

- Consequence: `mongoose-jlr` gains a dependency on `uds-execution`; the
  checker gains a rule that `execute_prepared_uds_read` takes a
  `PreparedUdsTransaction`, and keeps every rule that forbids raw or arbitrary
  transmission. The shell gains a module-read service and commands; the UI
  gains a module-read panel that shows the route's validation and the
  response, negative responses included.

  A hypothesis is refuted as well as confirmed: a request on a hypothesised
  route that draws no answer is recorded with the same care as one that does.
  Either outcome is a `Captured` observation for the knowledge base; the
  intake that turns a tester's report into such a record is F13 work.

  `normal_fixed` (29-bit) and `enhanced` addressing remain unexecuted; the
  outbound frame encoder carries an 11-bit identifier and is not widened here.
  Nothing in this decision adds a write, a control, a session change, or a
  security service. Live vehicle status stays `NOT_YET_EXTERNALLY_VALIDATED`
  until a tester reports.

## Addendum, 2026-09-05

The medium-speed buses are now hypothesised too. A community statement
relayed by the owner (`docs/research/sdd/COMMUNITY_NOTES.md`) resolves the
tie this decision left open: `BO_MSCAN` and `CO_MSCAN` are two physical
buses joined by the gateway, which relays the tester's requests from J1962
pins 3 and 11 to the BO or CO branch. The built-in hypothesis manifest binds
both to route `ms-can` at 125 kbit/s, `Unverified`, on that statement as
`UnverifiedResearch` evidence; the mechanism of this decision is unchanged,
and the first answered read on such a car confirms or refutes the binding
per vehicle through the F13 intake.
