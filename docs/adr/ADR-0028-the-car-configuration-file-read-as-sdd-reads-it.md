# ADR-0028: the car configuration file, read as SDD reads it

- Status: Accepted, 2026-09-12
- Decision: the car configuration file (CCF) is read as one operation,
  `CCF_READ`, class `READ_ONLY`, from the module SDD's own data names as its
  keeper, block by block over `ReadDataByIdentifier`, and decoded parameter
  by parameter with the layout SDD publishes for that programme and model
  year; the layout enters the knowledge base as its own kind of record, the
  block identifiers enter it in the catalogue's shape so a configuration
  read is a module read, and every value is shown as the type SDD declares
  for it — an option's text, a number, a string — never as a judgement.
- Reason: the CCF is where a car says what it is fitted with and how it is
  set: brand, market, engine, gearbox, driven axle, tyre size, every
  optional system and its personalisation. SDD shows about sixty of those
  rows to a technician and edits some of them; this product will not edit,
  but reading them is the first thing a workshop wants after the passport,
  and stage 1 has said "CCF read" since 2026-09-01 without a line of code.
- Consequence: one new knowledge slice, one new entity kind, one decoding
  rule, one service in the shell, one section of the report, one row in
  the safety registry, one panel. Nothing written; the write service SDD
  names beside every block is not recorded at all.

## What SDD says, measured

The corpus was read before this was written. Every
`CCF_DATA_<PROGRAM>[_<YEAR>].xml` under `CURRENT_JLR_XCL_XML_DATA_XML/Xml`
— 46 documents, 22 programmes, markers `MY04_5` to `MY17`, the programme
and marker taken from the document's own `<qualifier>` and never the file
name — describes one car's configuration as:

- a header, `<ccf>`: the **sync** module that keeps the master copy and
  the **source** modules that hold copies. Over 46 documents the sync is
  `GWM` (19), `BCM` (14), `IPC` (7) or `RSJB` (6); `PCM` holds a copy on 18
  cars, `FSJB` on 6. `AS_BUILT`, `AS_IS` and `OTHER` are not modules — the
  factory file and a placeholder — and are not read;
- **blocks** — `CCF` (41 documents), `EUCD` (25), `ITP` (36), `VEH_BUILD`
  (36), `MCODE` (5), `RES` (3) — each with `serviceIdRd="0x22"` and, for
  every module that holds it, an `<address>`: the data identifier, the
  offset of the block inside that identifier's payload, and its length.
  `PAD` and `SOM` blocks name service `0x00` and are padding, not data.
  On the X250 of 2010 the RSJB answers `0xF106` with the 196-byte `CCF`
  block, `0xD900` with the 252-byte `EUCD`, `0xC25F` with the 112-byte
  `ITP`, and `0xF105` with `VEH_BUILD` at offset 0 and `RES` at offset
  `0x8A`; the PCM answers `0xF106` with its copy of `CCF` and `0xF105`
  with `EUCD`, `ITP` and `VEH_BUILD` at offsets `0x0`, `0xFC` and
  `0x16C`, one address per engine. Block lengths across the corpus: 112,
  196, 96, 784, 252, 138, 256, 146 bytes;
- the **layout**: 22,185 groups, each a byte range and a title, holding
  38,576 parameters — `id="SSS_EEE_bbb_BBB"` is the start byte, the stop
  byte, the first and the last bit of the span, with a `mask` and a `type`:
  `ENUM` 24,420, `BOOL` 10,231, `BIN` 1,941, `ASCII` 1,790, `BCD` 162,
  `UNDEF` 32 — and 142,162 options for the enumerations and booleans, each
  a value, a name, sometimes a sales `code`, and a text. A `<category>`
  says whether SDD's editor displays the parameter (2,708 of 38,576; 8 to
  99 per car, 54 on the X250 of 2010), whether it edits it, and its scope
  (`base`, `personalisation`, `configuration`, `market`);
- the **texts**: 2,788 distinct text ids over the titles and options, of
  which 2,313 resolve in `CURRENT_PAG_MCP_TEXT_XML` (7,164 items, twelve
  languages, no Ukrainian) and 475 exist nowhere in SDD 169 — for those
  SDD itself shows the mnemonic.

**Two read schemes.** 28 documents address every block with an identifier,
an offset and a length, as above. 18 — the gateway cars from 2014 on:
L405 and L494 from MY14, L538 from MY14, L550, X152 MY16, X260, X351 from
MY16, X760, X761 — address the `CCF` block through `0xEE00` as a series of
*VDF blocks* (`start_vdf_block`, `stop_vdf_block`, `vdf_type`,
`vdf_offset`, `dynamic_block_in_use`, `merge_by_vdf_ids`), a paged
mechanism the corpus describes but whose request sequence this reading
does not establish. The first scheme is decided here; the second is
recorded and not read.

## Decisions

1. **The configuration is read as SDD reads it.** For the described car,
   the blocks with `serviceIdRd="0x22"` and the modules that hold them,
   sync first and then each copy; one `ReadDataByIdentifier` per distinct
   module and identifier, the block being `payload[offset..offset+length]`,
   so `0xF105` is asked once for `VEH_BUILD` and `RES` together. Addresses
   narrowed by a qualifier (the PCM's engine) apply to the car that states
   it; a car that does not state one is not given one engine's address. A
   VDF-scheme car is refused with the reason, in the run and on screen,
   never omitted.

2. **Recorded in two shapes.** The identifiers enter the knowledge base as
   `IdentifierParameter` records in the catalogue's shape — SDD's block
   name for the parameter, the encoding `ccf=<BLOCK>;offset=<n>;length=<n>`,
   applicable to the programme, the marker, the address's qualifier and the
   module — so the resolver's readable list and the transaction gate admit
   the read exactly as they admit a catalogue parameter (`ADR-0027`'s rule),
   and a plain module read of `0xF106` says what it is. The layout enters as
   its own kind, `ConfigurationParameter` (`ADR-0010`'s precedent: a kind is
   added, none is overloaded), one record per parameter — its block, byte
   and bit span, mask, type, display, edit and scope flags, group and
   titles, and its options with value, name, code and text — applicable to
   the programme and the marker. The texts are resolved at ingest in English
   and Russian, the two languages the interface already shows SDD's data
   text in; an id SDD has no text for keeps its mnemonic, as SDD does. The
   product's own words for these 2,313 phrases are a later dictionary under
   `ADR-0025`'s rule and not this decision.

3. **A value is what the type says, and nothing more.** `ENUM` and `BOOL`:
   the option whose value equals the masked, shifted bits, shown by its
   text and its name; a value no option lists is shown as the number with
   that fact stated. `BIN`: the number over the span. `ASCII`: the text
   with the padding trimmed, the passport's rule. `BCD`: the digits.
   `UNDEF`: the bytes. No word such as *wrong*, *corrupt* or *should be*
   appears anywhere; a configuration is what the car holds. Rows SDD's
   editor hides are in the report and behind one switch on screen, labelled
   as what SDD hides.

4. **Copies are compared, never judged.** A copy module's block is decoded
   the same way and each parameter is marked the same as the master's or
   different, with the copy's value — arithmetic over two readings. What a
   difference means, and SDD's procedures for a corrupt or missing CCF, are
   stage 3 and a person's decision.

5. **Nothing here leads to a write.** `serviceIdWr` is not recorded. The
   as-built VBF names (`t_ccf`) are not recorded (`ADR-0005`). Addresses
   with `upload="mem"` are not recorded: memory reads are not a service this
   product makes. The `edit` flag is recorded as a fact about SDD's editor
   and drives nothing. `0x2E` stays absent from every crate, as the
   architecture check has it.

6. **One operation, the passport's shape.** `CCF_READ`, class `READ_ONLY`,
   schema `prowlone.ccf-read`, bundle field `ccf_reads`. The shell plans
   the reads from the loaded library, the interface steps and can stop
   between any two, every read leaves the record a single read leaves, and
   the intake reads `ccf_reads[i].reads[j]` through the path a module read
   goes through. The decoded values are readings, not evidence about a
   route, and are not turned into knowledge.

7. **The bench answers a plausible configuration.** For a block identifier
   the bench answers a deterministic block in which every enumeration and
   boolean lands on a listed option and the VIN field holds the bench's
   VIN, from the same layout the decoder uses, every row `SYNTHETIC`
   (`ADR-0020`).

## What this does not decide

- **Describing the vehicle from its configuration.** 181 options on the
  X250 alone carry a `qualifier_map` — the model year byte maps to
  `CM_QUAL_CCF_MODEL_YEAR`, the engine byte to the engine qualifier. This
  is how SDD knows a car from its CCF, and it is the deferred "digital car"
  of `ROADMAP.md`: a read CCF could fill the picker. A separate ADR.
- The VDF read scheme of the 2014-and-later gateway cars.
- ~~The product's own words for the configuration texts.~~ **Decided by
  the owner later the same day:** the texts are this project's own in both
  languages, Ukrainian and Russian, made the way the parameter names and
  the fault-code help were — SDD's English worded first, then translated,
  then reviewed — and never taken from SDD one to one, so that no sentence
  of SDD's is reproduced. The unit is small: the 48 documents use 2,782
  distinct texts (1,436 option texts, 443 parameter titles, 428 group
  titles, 475 mnemonics SDD has no text for), and 124 of them cover half
  of all 138,683 uses. The input file is `prowlone-ccf-texts.csv` on the
  owner's desk, ordered by use; it goes through the pipeline after the
  help texts and the parameter names. Where the dictionary lives follows
  `ADR-0025` (the repository, keyed by SDD's text id).
- Comparing a car with its as-built file, which SDD holds as a VBF.

## Built, 2026-09-12

`IMPLEMENTED / FIXTURE_TESTED`; nothing has met a car.

- **Ingest.** `CcfAdapter` reads a `CCF_DATA` document into three kinds of
  record: the scheme and the sources as text claims on the programme; the
  block identifiers as `IdentifierParameter` records with the encoding
  `ccf=<BLOCK>;offset=<n>;length=<n>`, narrowed by the address's engine
  qualifier and, where the address stamps a neighbouring year, by that year;
  the layout as `ConfigurationParameter` records, one per parameter, with
  the group and titles resolved in English and Russian through a
  `TextLookup` the exporter fills from all 6,981 items of SDD's text
  database. Two things the real corpus taught: the X250 document of 2013
  stamps its reserved block's addresses `MY12`, so a document's marker is
  the one most of its qualifiers name and an address keeps its own; and the
  X351 document of 2016 writes one parameter's span stop-before-start, so a
  span that cannot be read leaves that parameter unrecorded and the
  document's other parameters stand. Real source: **48 documents** (46 under
  the XCL tree, 2 legacy Jaguars under the MCP tree), 20 programmes, none
  rejected; 335 block identifiers over six modules (`BCM`, `FSJB`, `GWM`,
  `IPC`, `PCM`, `RSJB`), 38,505 parameters of which SDD's editor displays
  2,871, 141,125 options; scheme `did` for 30 documents and `vdf` for 18.
  The library exported afresh holds 353,450 records against 314,462, its
  `ccf.json.gz` 6.8 MB — the texts of 141,125 options in two languages —
  and the whole library 21 MB on disk; the owner's copy was re-stamped.
  Golden tests: five over a fixture that mixes identification with
  padding, two blocks in one identifier, an engine-qualified copy, an
  address stamped with a neighbouring year, a span written backwards and a
  paged document.
- **Decoding.** `diagnostic_session::ccf` reads the layout back and decodes
  a block: an enumeration or a boolean to the option whose value equals the
  masked, shifted bits — its text, its name, its code — and to the number
  with the fact stated when no option lists it; a binary field to the
  number; ASCII with the padding trimmed; BCD to digits; the rest to bytes.
  A block too short for a parameter says so. `parameters_span` knows a block
  reference, so the bench answers the right length; a plain module read of a
  block identifier says what it is and leaves the decoding to the
  configuration read.
- **Library.** `ccf_scheme`, `ccf_sources` (sync first), `ccf_layout` (block
  and byte order) and `ccf_blocks` (per identifier, offset order).
- **Bench.** A module's block identifiers answer a deterministic block in
  which every enumeration and boolean lands on a listed option, the
  seventeen-character text holds the bench's VIN, and another module's copy
  differs. Every row `SYNTHETIC`.
- **Shell.** `CcfService` in the passport's shape; four commands; the
  session bundle's `ccf_reads`. The end-to-end bench run reads SYNTHMOD's two
  identifiers, slices the second block of `0xF105` at its offset, decodes
  eight parameters, counts five hidden rows, refuses OTHERMOD's copy with the
  resolver's reason, and fails if a bench row ever reads `SOURCE_BACKED` or
  the report ever says *corrupt*, *wrong*, *should be* or *invalid*.
- **Intake.** `ccf_reads[i].reads[j]` through the builder a module read goes
  through; the decoded configuration is not evidence about a route and a
  test asserts no value of it reaches a manifest.
- **Interface.** A panel under the passport's — SDD's groups as headings,
  the row's title in the interface's language where SDD has it, the block,
  the value with the option's code, a copy's difference beside the master's
  value, the rows SDD hides behind one switch that says how many — and a
  button beside the passport's. Three interface tests (88 in all).
- **Registry.** `ccf_reads` in `SAFETY_BOUNDARIES.md`, class `READ_ONLY`.
