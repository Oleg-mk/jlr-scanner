# ADR-0016: Session reports as captured evidence

Status: accepted, 2026-09-04. Milestone M2 (F13 intake).

## Context

Every claim the application acts on today is documented (SDD), synthetic, or
an unverified research hypothesis (ADR-0015). Nothing is `Captured`, and no
record has ever reached `CaptureValidated`. The tester programme exists to
change that: a tester's saved session report — the survey, the listen-only
captures, the module reads, the F8 calibration read — is the first evidence
the project will ever hold from a live vehicle. Something has to turn a
report into knowledge records, and the rules for what a report may and may
not establish must be fixed before the first report arrives, not after.

Two facts about the existing model shape the decision. The resolver groups
the applicable records of a field by value and, when they agree, reports the
field's validation state as the *weakest* among them; and the survey calls a
route a hypothesis when *any* of its evidence is unverified research. Under
those rules a captured confirmation that agrees with a hypothesis would
change nothing visible: the field would stay `Unverified` and the map would
keep the dashed line.

## Decision

1. **A session report is a `Captured` source.** The intake tool
   (`crates/report-intake`) turns one report into one F5 manifest whose
   source is `SourceType::Captured`, whose evidence is
   `EvidenceClass::DirectObservation`, and whose records are
   `ValidationState::CaptureValidated`. The report's SHA-256 is the source
   fingerprint; the manifest never copies a VIN or an adapter serial number.

2. **What a module's answer confirms.** A UDS response from the expected
   responder — positive or negative — confirms, for the vehicle the report
   describes, exactly the facts the request was built from: the module's
   diagnostic addressing, its logical bus, the bus's physical route on the
   adapter, and its protocol family. A *positive* response additionally
   confirms the read-only capability that was exercised. The intake writes
   these as records whose values are copied from the library records the
   plan resolved, so agreement is exact and the store sees one value, not a
   conflict. An answer from an unexpected responder is recorded with the
   observed identifiers instead, and the resulting conflict is the correct
   outcome: the data and the vehicle disagree, and the survey must say so.

3. **What silence does not confirm.** No answer is recorded as an
   observation — a `captured_read_attempt` claim naming the route, the
   request and the outcome — and never as a refutation. A module may be
   absent, asleep or on another bus; refuting a route takes more than one
   silent read, and that judgement stays with a person reading the attempts.

4. **What a capture does not confirm.** A listen-only capture establishes
   that a live bus is on a J1962 pair of that vehicle; it cannot name the
   logical bus. It is recorded as a `captured_bus_activity` observation on
   the vehicle programme, with frame and identifier counts, and binds
   nothing.

5. **Fault codes are vehicle state, not knowledge.** Which codes a module
   reported is not recorded as a claim; that a module answered service 19 is.
   The report keeps the codes for the tester and the programme.

6. **Applicability is the reported vehicle, no wider.** Every captured record
   applies to the programme, the model year and — when stated — the engine
   and the breakpoint marker of the report's vehicle context. One car's
   evidence generalises to nothing on its own; wider applicability, if ever,
   is a later ingest over many reports, not this tool's inference.

7. **Agreeing records take the strongest state.** When the applicable records
   of a field agree in value, the field's validation state is the strongest
   among them, not the weakest: a captured observation that agrees with a
   hypothesis makes the value `CaptureValidated`. The plan as a whole keeps
   reporting the weakest of its *fields*. A route is a hypothesis only while
   every trace behind its physical and backend route is unverified research;
   one direct observation ends the hypothesis, and the survey shows the route
   as reachable and capture-validated.

8. **The intake changes nothing in the library it reads.** It reads the
   library to copy values and writes one new manifest; the library grows
   only when the manifest is placed in the library directory, which the
   loader then reads like any other. Removing the file removes the evidence.

## Consequences

- The hypothesised routes of ADR-0015 become reachable, programme by
  programme, as reports arrive; `VEHICLE_CONFIRMED` in the documents means a
  `CaptureValidated` record exists for that vehicle context, nothing looser.
- The synthetic-versus-real rule of F5 is untouched: a captured manifest
  mixes with documented and research evidence and never with synthetic.
- The resolver's per-field aggregation changes from weakest to strongest
  among agreeing records; the F6 tests are updated to say so.
- What the tool cannot decide it reports: an ambiguous library value for a
  field, a report from a version it does not read, a read without the
  identifiers needed to name the module — each is listed in the intake
  summary and no record is written for it.
