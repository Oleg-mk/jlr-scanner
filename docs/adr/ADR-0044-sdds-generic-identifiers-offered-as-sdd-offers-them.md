# ADR-0044: SDD's generic identifiers offered as SDD offers them, not swept

- Status: **Accepted, 2026-11-23**, on the owner's word («все інше
  зробити за твоєю попередньою пропозицією»), replacing the sweep of
  2026-11-15 that he withdrew the same day («бред … зайшли у глухий кут»).
- Scope: which catalogue identifiers the interface lets a module be read
  by, and how it marks them. No new operation, no new transmit path, no
  change to the knowledge export or the issued libraries.

## Context

The first tester's SDD offers 253 named signals under `303-00 Engine
system` for his Discovery 4 (L319 MY10, V8); ProwlOne offered the same
engine 92 identifiers, 128 parameters.

SDD's identifier catalogue comes in three classes: records qualified to a
car (programme, year, engine), records qualified to a module type, and
records qualified to nothing — 1,140 identifiers in SDD 169, 3,806
parameter names, with no test at all. The resolver offers a module the
first two only, by rule: an unqualified record is formatting knowledge,
not evidence that the module holds the identifier. All ten signals legible
on the tester's SDD screenshot are unqualified records (`0xF443`,
`0xF411`, `0xF447`, `0xF449`, `0xF44A`, `0x0412`, `0x032F`, `0x0330`,
`0x0331`, `0x0337`).

Which unqualified records belong to which module is not in SDD's XML
data: the platform document's `NET` set (59 identifiers for this PCM) and
the module index (`MDX_PCM_P`, 6) do not hold the list; the group name
`303-00 Engine system` appears only in a vocabulary; narrowing by the
modules SDD ties an identifier to anywhere else in its data drops nine of
the ten signals on the screenshot; and the number 253 matches 9,565
combinations of identifier ranges, so no range rule is behind it.

What SDD does with that list is the point: **it does not ask the module.**
It offers the generic list, the person picks, and an identifier the module
does not hold is declined at read time.

### The sweep, considered and withdrawn

On 2026-11-15 a read-only sweep was built — the module asked once for each
of the 1,140 identifiers, in batches of 25 on one opened channel — and
passed the owner's bare adapter. Its cost was named only afterwards: on the
tester's adapter one read took 150–530 ms, so a module would take minutes,
every module more, for a list SDD shows at once. The owner judged it
useless and it is removed; nothing of it remains in the product but the
lesson, recorded in `docs/DEVELOPMENT_PRINCIPLES.md`'s spirit: the cost of
a new operation is put to the owner in numbers before it is built.

## Decision

1. **One list for the library.** When a library loads, the catalogue's
   identifiers qualified to nothing are indexed once
   (`KnowledgeLibrary::generic_identifiers`), marked `generic`, in the
   catalogue's own words and in every language it has. They depend on no
   car, and they are never written into a module's survey list.
2. **Offered as SDD offers them.** In the live read's chooser every module
   gets, under its own list, a search over the generic list by name or
   address. A match is added to the set for that module with one press,
   shown beside the module's own identifiers and marked *no module in
   SDD*. The dials and the "SDD qualifies to this car" count take no
   generic identifier for the car's own.
3. **Readable, declined by the module.** A single read and the live read
   accept a generic identifier as they accept any other: the same prepared
   read-only transaction, the catalogue's own decoding. A module that does
   not hold it answers with a refusal, which is shown as every refusal is.
   Nothing is sent to find out beforehand.
4. **Nothing invented.** Only identifiers the catalogue defines can be
   asked for; a typed number the catalogue does not describe is refused at
   preparation, as before.

## Consequences

- The same list for every car and module, at no cost until a value is
  picked; what SDD's datalogger offers is reachable by name.
- The list is not SDD's per-system grouping: SDD shows 253 under the
  engine, this shows 1,140 behind a search. The grouping is not in the
  data, and no attempt is made to guess it.
- `READ_ONLY` throughout; the architecture guard is unchanged.

## Validation

`IMPLEMENTED` / `FIXTURE_TESTED`: the generic index and a generic read
through the whole stack on the bench; the chooser's search, add and mark
in the interface's tests. No car has read a generic identifier through
this product yet.
