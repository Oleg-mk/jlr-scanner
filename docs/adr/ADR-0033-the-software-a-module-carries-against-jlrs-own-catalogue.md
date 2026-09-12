# ADR-0033: the software a module carries, against JLR's own catalogue

- Status: Accepted, 2026-09-12 (the owner took the row the passport left
  open: "Software level checked against JLR's catalogue")
- Decision: the part numbers a module reports — its assembly, its software,
  its calibration, its hardware — are **compared** with the numbers JLR's own
  IVS part lineage names for that assembly on that car, and the comparison is
  shown beside each passport row in four honest states: the number agrees,
  the catalogue names a different one, the catalogue names none for that
  identifier, or the catalogue does not carry the assembly at all. Nothing is
  downloaded, nothing is applied, and no row ever says *outdated*.
- Reason: the passport (`ADR-0027`) answers "which one is it" and stops
  there. The next question a workshop asks is "is that the software this unit
  is supposed to have", and the answer is in the corpus: IVS lists, per
  programme and per module, which software and calibration part numbers
  belong to each assembly part number. The product already reads both sides of
  that comparison; all that is missing is the join.
- Consequence: one ingest slice, one lookup in the session layer, one column
  in the passport panel and one field in the report. No new operation, no new
  safety class, nothing written to a module, and the component's own
  programming material stays out of the library.

## What SDD says, measured

Read on 2026-09-12 from `COMMON_JLR_SMPACK_XML/IVS`:

| | |
| --- | --- |
| files | 20; **17 production** (`Environment` Production, `ValidationStatus` Yes, all stamped Oct-17-2022, `IVS 3.1`) |
| distinct documents | 15 — `2011_L538`, `2011_L538C` and `2011_L538JV` are byte-identical |
| programmes | L319, L320, L322, L359, L405, L494, L538, L550, X150, X152, X250, X260, X351, X760, X761 |
| part lineage blocks | 81,643 |
| ECU acronyms | 122 |
| distinct assembly part numbers | 53,364 |
| distinct software part numbers | 40,920 |
| software parts | 261,862 — Strategy 165,242, Calibration 62,478, Signal Configuration 30,213, ECU Configuration 3,486, Interim 443 |

Each `PartLineage` names an `ECUAcronym`, an `AssyPN` and the identifier that
assembly answers on (`AsDeliveredPID`: `F112` 58,661 times, `F113` 22,975),
a `HardwareComponentPart` with its own identifier (`F111` 70,586, `F191`
10,551) and, under `Node`, the `SoftwareComponentPart` entries with their
part type, their part number and the identifier each answers on — `F188`
78,125, `F124` 46,578, `F108` 26,346, `F120` 24,336, `F125` 16,108, `F121`
13,404 and a long tail.

Two measurements decide the shape of the feature:

- **The mapping is a function.** Of the 213,879 combinations of programme,
  module, assembly and identifier, exactly **244** name more than one part
  number. So for a given assembly there is one number to expect per
  identifier, and a comparison has one answer rather than a set.
- **There is no supersession chain to read.** `PartLineageId` carries a
  sequence number, but only **26** of 59,254 lineage bases appear with more
  than one. The data says what belongs together, not what replaced what, so
  this product can say *different* and must not say *newer* or *outdated*.

The join to what the product already holds is real: all 15 programmes are
among the library's 22, and 73 of the acronyms are module families the
library knows. Per car, for example: X250 37 of its 48 families, L322 38 of
55, L405 54 of 65, X351 44 of 55.

## Decisions

1. **Numbers only, in one direction.** The product reads part numbers from
   the car and compares them with the catalogue. It never downloads
   software, never writes a module, and never proposes a programming action:
   `ADR-0005` is untouched and reflashing stays excluded. The component's own
   `ServiceActions` and `CoordinatedFlashList` — service orchestration, which
   is exactly the material that must not enter this product — appear in the
   three test-environment files alone, one each, and no ingest touches them.

2. **Only what SDD marks as production enters the library.** A document is
   ingested when it says `Environment` Production and `ValidationStatus` Yes;
   anything else is refused by the adapter and skipped by the exporter with a
   line saying so. That rule is SDD's own stamp, and it excludes precisely the
   three files that carry flash lists.

3. **Four states, and none of them is a verdict.**
   `AGREES` — the number the module reports is the number the catalogue
   names. `DIFFERS` — the catalogue names a different number, which is shown.
   `NOT_NAMED` — the catalogue carries this assembly and names no part for
   this identifier. `NO_ASSEMBLY` — the catalogue does not carry the assembly
   the module reports, which is ordinary: a replaced unit, another market, a
   programme the snapshot does not cover. The interface says once that the
   catalogue is a snapshot of **2022-10-17** and that a difference is a
   difference, not a fault — the same discipline the battery card keeps.

4. **One record per lineage block.** The parts are carried as three parallel
   lists — identifiers, part numbers, part types — in the escaped
   `key=value;…` text of `ADR-0028`, rather than one record per part: 81,643
   records instead of 343,505, on a library that a ten-year-old machine
   already takes seconds to load. Applicability is the programme and the
   module family; the file's baseline year is written into the text and not
   matched, for the reason `ADR-0032` gives about SDD's year markers.

5. **The comparison is made on a normalised form**, and the interface says
   so: both sides uppercased with everything but letters and digits removed,
   so `8X23-18C808-CE` and `8X2318C808CE` are the same number. Nothing has met
   a car, so the exact shape a module answers with is not yet known; the
   normalisation is an assumption, named as one, and a tester's capture is
   what will confirm or correct it.

## What this does not decide

- Whether a difference matters. Age, market, a replaced unit and a dealer
  update all produce one, and this product does not rank them.
- `SOTAEnabled`, `CertificationRequired`, `ProgInSvc`, `ConfigurationMethod`
  and the other programming attributes of a lineage block. They are not
  ingested: they carry no meaning for a read-only comparison and every one of
  them belongs to the programming world this product stays out of.
- The three test-environment documents, and the service actions inside them.
- Anything about JLR's online services. This is SDD 169's own snapshot, the
  one on the machine, and it ages from its own date.

## Built

**2026-09-12 — `IMPLEMENTED / FIXTURE_TESTED / REAL_SOURCE_INGESTED`.**
Nothing has met a car, and the comparison is exactly as good as that.

- *The ingest.* `IvsLineageAdapter`: one record per assembly, entity kind
  `ModuleAssembly`, claim `sdd_ivs_assembly`, the parts as three parallel
  lists. It refuses a document SDD does not stamp `Environment` Production
  and `ValidationStatus` Yes, and any document carrying `ServiceActions` or
  `CoordinatedFlashList`; the exporter asks first, so SDD's own test files
  are skipped rather than rejected. A part the catalogue ties to no
  identifier is not recorded — supporting bootloaders, and the 2,518 whose
  identifier is `SECX` or `N/A` — because nothing a module answers could
  ever meet it.
- *Real source.* 20 documents found, **5 skipped** (three test files, two
  byte-identical copies of the L538 document), 15 ingested, nothing
  rejected: **67,755 assembly records** naming **258,704 parts** over 15
  programmes, 122 module families and 39 identifiers — `F188` 61,763 times,
  `F111` 58,624, `F124` 40,811, `F120` 19,475, `F108` 18,856. The library
  went to **425,799 records** from 358,044, 21 MB to 24 MB packed; the
  owner's copy re-stamped (code 0363-3484, valid to 2026-10-12).
- *The join, measured before it was built.* The passport reads identifiers
  on 845 programme-and-module pairs; the catalogue names parts on 878; **581
  are the same pair**, and for every one of those 581 the identifier the
  catalogue says the assembly answers on is one the passport actually reads.
  On those pairs the passport reads 11,332 identifiers, of which **2,586**
  have a catalogue part to be set against; the rest say the catalogue names
  no part here, which is what they are.
- *The rule.* `knowledge::part_number`: two numbers are the same number when
  their letters and digits are, ignoring case and separators. One module,
  two tests, and the assumption written where it can be found again.
- *The session.* `KnowledgeLibrary::catalogue_assemblies` reads the lineage
  once when a run is planned; `catalogue_comparisons` sets every reading
  against it afterwards, recomputed as answers arrive because the assembly
  can come in after a number it explains.
- *The panel and the report.* A fifth column: *the same number*, *the
  catalogue names …* with the number and SDD's part type, *the catalogue
  names no part here*, or *the catalogue does not carry this assembly*. One
  line under the table gives the catalogue's own date and says a difference
  is a difference. The readable report carries the same column and the same
  sentence. The word *outdated* is in no language file, and two tests fail
  if it ever appears in English, Ukrainian or Russian.
- *The intake.* Untouched: it reads `module_passports[i].reads[j]` and knows
  nothing of the comparison, which is a reading of the library and not
  evidence about a car.
- *Tests:* six F9 goldens (the safety one first: nothing that could
  programme a module is ingested), the part-number rule's two, one F10
  session test over all four states with the claim literal asserted against
  the ingest's, and an interface test that asserts the four states, the
  catalogue's date and the absence of a verdict in three languages.

**Departures from the text above.** None in substance. Decision 4 said 81,643
records; the ingest writes 67,755, because the three test documents and the
two duplicate copies are skipped and a module's assembly is one record
however many nodes declare it.

See `CURRENT_STATE.md` for the state and `F9_SDD_KNOWLEDGE_INGESTION.md` for
the ingest slice.
