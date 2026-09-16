# ADR-0035: What a module will accept — listed, not sent

- Status: Accepted, 2026-09-16 (the owner's decision to begin the desk work
  of stage 2 before any tester's report arrives: "переписати правий бік
  довідника, скласти ADR, розвести операції по класах і обкатати їх на
  стенді можна все зараз, за столом").
- Decision: the halves of SDD's module catalogue this project has so far
  left behind — every identifier a module declares **writeable** or
  **controllable**, and every **routine** it declares it can run — enter the
  knowledge base as knowledge. Each record carries the service that would
  carry it, the diagnostic session it requires, and the security level
  standing in front of it where there is one. **Nothing sends any of them.**
  The request constructors stay absent, the architecture guard stays as
  tight as it is, and the product's behaviour on a car does not change by
  one byte.
- Reason: the product today cannot answer "can this gearbox reset its
  adaptations?" — not because the answer is forbidden but because the data
  was never taken. That is a gap in knowledge masquerading as a boundary.
  Recording it does three things at once: a person reading a module learns
  what that module is for; the roadmap's stage 2 stops being an estimate and
  becomes a list with numbers; and the boundary becomes visible, because
  every such row says in the same breath that this build will not send it.
- Consequence: `sdd-ingest` reads `ACCESS_PARAMETERS` and
  `ROUTINE_IDENTIFIERS` from the per-module indexes it already opens for the
  readable half. Three claims join the store — a writeable identifier, a
  controllable identifier, a routine — each with its service, session and
  security reference. The knowledge library grows; no crate gains a way to
  transmit anything.

## What SDD says, measured

Read on 2026-09-16 from `CURRENT_JLR_XCL_XML_DATA_XML/Xml/*/MDX_*.xml` —
1,790 documents over 53 programme-years and 102 kinds of module:

| declared | entries | service |
| --- | --- | --- |
| `READABLE` | 8,004 | `0x22` ReadDataByIdentifier |
| `WRITEABLE` | 2,294 | `0x2E` WriteDataByIdentifier |
| `CONTROLLABLE` | 124 | `0x2F` InputOutputControlByIdentifier |
| `ROUTINE` | 8,144 entries, **253 distinct routine numbers** | `0x31` RoutineControl |

A first count of these, taken with regular expressions, reported 6,588
readable, 2,300 writeable and 8,279 routines. It was wrong on three of the
four: the pattern for an identifier missed documents whose element shape
differed, and the pattern for a routine spanned across neighbouring
elements. The numbers above were taken with a parser and are corrected here
rather than quietly, because a measurement that hides its own mistake is
worth less than one that does not.

The shape is one element per access, beside the identifier it belongs to:

```xml
<DID ID="did_0453">
  <NAME>High Pressure Pump Control</NAME>
  <NUMBER>0x0453</NUMBER>
  <ACCESS_PARAMETERS>
    <READABLE     SERVICE_REFS="service_22" SESSION_REFS="session_01 session_03"/>
    <CONTROLLABLE SERVICE_REFS="service_2F" SESSION_REFS="session_03"
                  SECURITY_REFS="security_level_1" IOCP_REFS="iocp_00 iocp_03"/>
  </ACCESS_PARAMETERS>
```

and one element per routine, with SDD's own name for it:

```xml
<ROUTINE ID="routine_0406" SESSION_REFS="session_03">
  <NAME>Clear adaption values</NAME>
  <NUMBER>0x0406</NUMBER>
  <MAX_ROUTINE_RUN_TIME/>
  <RESTART_WHILE_RUNNING>no</RESTART_WHILE_RUNNING>
</ROUTINE>
```

Three facts the numbers settle, which were guesses before:

- **The session is the gate, and it is not the exception.** Every one of the
  2,294 writeable and 124 controllable identifiers names `session_03`, the
  extended diagnostic session; 33 writeable also allow the default session,
  and nothing is writeable or controllable in the default session alone.
  Stage 2 therefore begins with `0x10 03` and with keeping that session
  alive, not with the write itself.
- **Security stands in front of a minority, not the majority.** Of the 2,294
  writeable identifiers, **628** name a security level; of the 124
  controllable, **24**; and 114 readable ones name one too. So about three
  writeable identifiers in four, and five controllable in six, are not
  behind JLR's seed and key. The wall is real and it is narrower than
  assumed.
- **A third of the routine entries are already known to this project.**
  Routine `0x0202` appears 1,528 times: it is the on-demand self test that
  `ADR-0032` already lists and does not run. `0x0404`, VIN Learn, appears
  1,308 times. The remaining 251 numbers are new knowledge.
- **Seven documents carry a second section.** Two `MDX_PCM.xml` — X152 and
  X351, both `201600` — hold two `<ROUTINE_IDENTIFIERS>`, and five
  `MDX_AHCM.xml` hold two `<DATA_IDENTIFIERS>`. The adapter reads every
  section rather than the first of each; the first export, which read only
  the first, lost two routines and said nothing about it.

## What this is not

It is not permission. `SAFETY_BOUNDARIES.md` is unchanged: every operation
in the product stays `READ_ONLY`, `VOLATILE_CONTROL` and `SERVICE_ROUTINE`
stay unused, and `FORBIDDEN_PROGRAMMING` stays forbidden. The guard in
`scripts/check-architecture.mjs` still fails the build on
`RoutineControl`, `WriteDataByIdentifier`, `SecurityAccess`,
`diagnostic_session_control` and `tester_present` in the live path, and this
decision does not touch one line of it. Sending any of these needs its own
ADR, its own safety class, its own confirmation in the interface, and the
guard loosened in the same commit that authorises it — visibly, in the
history, never by drift.

It is also not a claim that any of it would work. A declaration in SDD's
catalogue is `documented` evidence that JLR says a module accepts something.
Whether a given car answers is `VEHICLE_CONFIRMED`, and no car has answered
anything yet.

## Precedent

`ADR-0032` did exactly this for the self tests: 1,153 declarations listed
with SDD's own words, timings and instructions, and not one run. The class
of the operation is what keeps listing and doing apart. This decision
extends the same separation to the rest of the catalogue.
