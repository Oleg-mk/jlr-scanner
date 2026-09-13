# ADR-0032: The self-tests a module declares — listed, not run

- Status: Accepted, 2026-09-12 (the owner took the next row from the
  capability matrix: "Перелік самотестів і процедур, які модуль уміє").
  Amended 2026-09-13 by `ADR-0034`: the words a test is described in are
  SDD's own — English, and Russian once the ODST Russian pack is ingested —
  and `description_texts` is keyed by SDD's language code, not the
  interface's; the wording table this decision leaned on left the product.
- Decision: the product **lists** every on-demand self test SDD declares for
  a module on this car — its identifier, SDD's own name for it, how long it
  takes and how long the tester waits, and the description SDD writes for
  the technician — and **runs none of them**. The list is knowledge. Running
  a test is a routine, it is stage 2, and nothing here brings it forward.
- Reason: a person looking at a module wants to know what that module can be
  asked to do, long before anything can ask it. The corpus names it plainly:
  93 module documents, 146 declarations over 119 module-and-test pairs, each
  with a help screen written for whoever runs it. Until now this project
  recorded only that such a capability exists, with no name a person reads
  and no description at all. Showing the list also makes the boundary
  visible: the product says what it will not do yet, with the reason,
  instead of leaving a blank.
- Consequence: the ODST ingest records three things beside the capability it
  already recorded — the test's timings, which help screen a given car is
  given, and what each screen says. The survey carries the list per module,
  the module panel shows it, and every row says in the same breath that this
  build does not run it.

## What SDD says, measured

Read on 2026-09-12 from `COMMON_SDD_DATA_ODST_LANG_EN`:

| | |
| --- | --- |
| module documents | 93 |
| test declarations | 146, over 119 module-and-test pairs |
| distinct help screens | 465 |
| distinct texts the screens are made of | 473, 38,180 characters, 81 on average |
| languages in the pack | English and Chinese; **no Russian** |

Each `odstTestSelection` names a `testID`, a `time` and a `timeout` in
milliseconds, a `flag`, and a `helpScreenDataNameId`; its `odstTestQualifier`
children say which models and model years it belongs to. The screens live in
the same document, with their own mnemonic list, so a test's prose needs no
other pack: it is `helpScreenDataName` → `helpScreenSelection` (by model and
year) → `helpScreen` → items → the document's own mnemonic texts. It is the
same chain the fault-code help follows, and it is recorded the same way.

## Decisions

1. **Recorded as known-but-unavailable, as `ADR-0009` permits.** Every record
   keeps `DiagnosticSafetyClass::ServiceRoutine`, which is what has kept
   these out of stage 1 since the adapter was written. Nothing gains an
   execution path: there is no `RoutineControl` constructor anywhere in the
   product, the architecture check forbids one by name, and this change adds
   no command, no service and no button that could run a test.

2. **The screen a car is given, and what the screen says, are two records.**
   One screen serves dozens of model-and-year pairs; writing its text once
   and binding cars to its name is the shape `ADR-0026` already chose for
   fault-code help, and it keeps the library from carrying the same
   paragraph forty times.

3. **SDD's English is the identity.** The pack has no Russian, so the texts
   enter in English and this project's own Ukrainian and Russian wording is
   added later, to the same dictionary the help texts use — 473 short lines,
   about twenty minutes on the owner's card. A test whose wording we do not
   have shows SDD's English, one line at a time, exactly as everywhere else.

4. **The list says what it is.** Each row carries the test's identifier, its
   name, its execution time and timeout, and its description; and the panel
   says once, plainly, that this build does not run self tests and why —
   a routine is not a read. No row is dressed as a button.

5. **The list is what the data does not rule out, and it says so.** SDD
   qualifies a test by a model and a model-year marker — `MY10`, `MY02_5`,
   `BASE`. The programme is a dimension a session states, so it rules out
   what belongs to another car. The marker is not: the ODST and DTC ingests
   record it on `sdd_model_year` and the DID catalogue records the same
   vocabulary on `sdd_year_breakpoint`, one SDD attribute written down under
   two names, and the vehicle context states only the second. Measured on
   2026-09-12: of 1,355 test qualifiers in the pack, 1,242 carry a marker
   and 19 say `BASE`, and the marker tokens are the same set in both
   corpora (`MY16`, `MY10`, `MY14`, `MY12`, `MY02_5`, …). Requiring a match
   would therefore list nothing at all on any car. So a test is listed when
   the data does not rule it out — the rule `describe_dtc_with_help` already
   uses for a help screen, for the same reason — and each row shows SDD's
   own marker verbatim beside the identifier, with one line above the list
   saying that this session does not check it. Unifying the two dimensions
   is a knowledge-model change with its own blast radius; it is named here
   and not done here.

## What this does not decide

- Running a self test, reading its result, or clearing what it logs. That is
  stage 2 and needs its own ADR, its own safety class and its own
  confirmation.
- The `flag` field: SDD's own meaning for it is not documented in the pack
  and is not guessed at here.
- Whether `sdd_model_year` and `sdd_year_breakpoint` become one dimension.
  They hold one vocabulary, but merging them rewrites the applicability of
  every ODST and DTC record in the library and deserves its own decision.
- The Chinese pack, and any language beyond the three the interface carries.

## Built

**2026-09-12 — `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`.**
Nothing has met a car, and nothing here can be run on one.

- *The ingest.* `OdstInfoAdapter` writes three claims beside the capability
  it already wrote: `sdd_odst_test` with the test's identifier, SDD's name
  for it and its `time`/`timeout` in milliseconds; `sdd_odst_help.<screen>`
  on the test's own entity, saying which screen a given car is given; and
  `sdd_odst_screen.<screen>` once per module, holding what that screen says
  — SDD's items in order, a blank item and an item with no text left out.
  Real source: the ODST slice went from 1,153 records to **4,085** over the
  same 93 documents — 1,153 test descriptions, 1,257 screen bindings and 522
  screen records covering 462 distinct screens and 238,504 characters of
  SDD's own prose. The library re-exported to **358,044** records (from
  355,112), nothing rejected; the owner's copy re-stamped (code CE99-19E2,
  valid to 2026-10-12).
- *The session.* `KnowledgeLibrary::self_tests` and the free
  `self_tests_for` over a store; the survey fills
  `ModuleSurveyEntry::self_tests`, so the interface needs no second call.
  One row per test, SDD's markers gathered onto it, sorted by identifier,
  every row carrying `SERVICE_ROUTINE`.
- *What a car sees.* 123 module-and-test pairs over 23 programmes. An X250
  shows 37 modules with a test each, an X351 43, an L322 30, an X150 23.
- *The panel.* A section in the module's own panel: the heading, one
  paragraph saying this build runs none of them and why, one line saying
  the year markers are not checked, then a row per test — SDD's name, its
  identifier, its markers, its time in seconds, its class, and its
  description line by line. Nothing in the section is a control: a test
  cannot be clicked, because there is nothing to click.
- *Execution.* Unchanged, deliberately. No `RoutineControl` constructor
  exists, `scripts/check-architecture.mjs` still forbids one by name, and
  this change adds no command, no service and no button.
- *Tests:* the six F9 goldens, a new F10 session test (the claim literals,
  the timings, the markers, the words, the module with none, the car with
  none, and the survey carrying the list), and an interface test that
  asserts the list renders and that no button can run one.

**Departures from the text above.** Decision 5 was written after the rest:
the first build filtered on a strict applicability match and would have
listed nothing at all on a real car, because SDD qualifies almost every test
by a model-year marker the session does not state. The rule is now "what the
data does not rule out", with the marker shown, which is what the fault-code
help already does.

See `CURRENT_STATE.md` for the state and `F9_SDD_KNOWLEDGE_INGESTION.md` for
the ingest slice's real-source numbers.
