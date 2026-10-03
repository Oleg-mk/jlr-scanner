# ADR-0034: The fault-code help is SDD's own text, in English or Russian

Status: accepted, 2026-09-13. The owner's decision. **Supersedes `ADR-0026`**
and its amendment of 2026-09-12.

## Context

`ADR-0026` made the fault-code help — possible causes, actions required, the
conditions a module sets a code under — this project's own text in three
languages, kept in the repository as `help_texts.tsv` and joined to the
library's screens by the mnemonic's name. By 2026-09-12 the table held
27,493 rows and the remaining 13,130 sentences had no words of ours.

Two things were measured on 2026-09-12 and 2026-09-13 before this decision:

- **A machine cannot be trusted to write the rest.** A 14B model on the
  owner's card produced text that was grammatical, kept every code and every
  number, and in one of thirty sampled rows named a different component —
  *exhaust gas recirculation bypass solenoid circuit* came back as the turbine
  vane actuator. In a diagnostic tool that line sends a person to dismantle
  the wrong thing, and no check the project could write would have caught it.
  The run was stopped and nothing from it was kept.
- **The rows already written were checked against SDD for the first time.**
  Put beside SDD's own English and Russian by name, 99.2 % of the 26,893
  named rows carried the same content words; none had swapped one component
  for another. But 56 rows had replaced a specific multi-step procedure —
  *check the neutral sensor, refer to the circuit diagrams, check the
  circuit, then suspect the powertrain control module* — with one generic
  sentence, *confirm the diagnosis and any applicable service requirements
  before replacing parts*. Nothing false, and yet not what SDD says; the
  owner's words for it: a different concept altogether.

And one fact was established the same day: **SDD 169 ships this layer in
Russian.** `COMMON_SDD_DATA_DTC_HELP_LANG_RU` holds the same 6,171 documents
as the English pack, the same mnemonic names, the same screens and
selections, with only the human text changed — 42,637 named texts in each,
one Russian text per name, and a Russian text for every one of the 38,526
names the library's screens use. It is JLR's own translation, made by people
who knew which part they were naming.

## Decision

1. **The help a car is shown is SDD's own text, unchanged.** No translation
   layer: not machine, not by hand, not later. The words on the screen are
   the words SDD shows, in the language SDD wrote them in.

2. **Two languages, English and Russian, both from the issued library.** The
   Russian pack is ingested the way the English one is: the same adapter,
   told the language, writing beside each English screen the same screen in
   Russian — `sdd_help_screen.<NAME>.rus` and
   `sdd_help_screen_items.<NAME>.rus`, the language last as the text
   database already names it (`sdd_failure_type.rus`). The selection records
   — which screen a code gets on which car — are written once, from the
   English pack; the Russian pack contributes text and nothing else, because
   its structure is the English pack's structure. The adapter refuses a
   document whose own `<language isoCode>` is not the language it was told.

3. **Ukrainian is not offered for this function.** The interface stays
   Ukrainian; the help panel and the self-test descriptions carry their own
   choice, English or Russian, remembered on the machine. A Ukrainian reader
   is shown Russian by default and can switch to English. There is no
   Ukrainian help text in SDD and this project no longer writes one.

4. **`help_texts.tsv` and the module that read it leave the product.** The
   27,493 rows stay in the repository's history and nowhere else. The
   ODST descriptions, which used the same table, follow the same rule: the
   owner opened the ODST Russian pack (`COMMON_SDD_DATA_ODST_LANG_RU`, the
   same 93 documents) the same day, and its adapter, told the language,
   writes `sdd_odst_screen.<NAME>.rus` and `sdd_odst_screen_items.<NAME>.rus`
   beside the English — the items claim written for English too from this
   day, so the lines can be joined by name.

5. **The library still decides what a car is shown**, exactly as before:
   which screen a code gets on this model and year, and which lines that
   screen holds. A Russian line is joined to its English line by the
   mnemonic's name; a name the Russian screen lacks keeps English on that
   line, and a library issued before this date, which has no Russian, is
   read as it always was.

6. **Redistribution is unchanged.** The English text of every screen already
   travels in the issued library under `ADR-0019`'s stamp, restricted to the
   named tester it is issued to; the Russian text travels the same way, with
   the same status. Neither enters this repository. The synthetic fixtures
   carry invented Russian, as they carry invented English.

## Consequences

- `crates/sdd-ingest`: `DtcHelpAdapter::with_language` and
  `OdstInfoAdapter::with_language`, the language suffix on the screen claims,
  a check on the document's own language; the exporter recognises a
  `_LANG_RU` root and takes only its `rdsDtcHelp*` and `rds-odst-info` files,
  under a source id that names the language.
- `crates/diagnostic-session`: `help_text.rs` and `data/help_texts.tsv`
  removed; `help_texts` on a described code and `description_texts` on a
  self test are keyed by SDD's language code and filled from the library,
  joined to the English by the mnemonic's name.
- Frontend: a language switch on the help and self-test panels, English or
  Russian, stored under its own key; `helpLines` takes that choice instead of
  the interface language.
- The library is re-exported with the Russian root and re-issued.
- `ADR-0026` is superseded. The disagreement it records — whether reworded
  text detaches from its source — is moot: the product now shows the source.
- SDD's Russian for the code *descriptions* — see the amendment below.

## Amendment, 2026-09-13, later the same day: the code descriptions too

The owner's word: «занеси російські описи». The same Russian pack carries
the two description indexes, `dtcDescriptions.xml` (4,033 entries against
the English 4,031) and `dtcModuleDescriptions.xml` (1,924 against 1,931),
the same codes and modules with the words changed.

1. **They enter as their own claim beside the English alias**:
   `sdd_dtc_description.rus`, the language last, from a source that names
   the language. The English description stays the alias it always was, so
   a library issued before this reads unchanged. The fault types are not
   read from the pack: the text database already gives them in every
   language SDD has.
2. **The index has no `<language>` element, so the words are checked
   instead.** An index read as Russian must be mostly Cyrillic and one read
   as English mostly not; the real indexes sit at the two ends, and a root
   mounted under the wrong name is refused.
3. **The Russian is chosen with the English's scope.** Where the English
   shown is the module's entry, the Russian shown is the module's entry;
   where it is the generic one, the generic one — and where the Russian pack
   has no entry of that scope, nothing is shown in Russian rather than a
   sentence about something else. It travels as `description_data_texts`,
   keyed by SDD's language code, apart from `description_texts`, which
   remains this project's own wording for the standard's codes
   (`ADR-0025`).
4. **On screen the code's line follows the same choice as the help.** Our
   own wording for a standard code comes first where it exists; otherwise
   SDD's description in the language chosen for SDD's text; otherwise
   English, with the English kept beside whenever something else is shown.
   The failure type's wording follows that same choice from this day;
   module names still follow the interface language, as before.

## Amendment, 2026-10-03: the parameter and state names too

The owner's idea for the assistant — speak Russian where there is no
Ukrainian text, and the translation problem falls away — reaches the one
data layer still shown in English only: the names of the identifiers and
parameters in the live read, the module passport and the configuration,
and the names of the decoded states beside them. SDD ships this layer in
Russian too. The owner opened `COMMON_SDD_DATA_SNAPSHOT_LANG_RU` with his
own scripts on 2026-10-01 (the project's code decrypts nothing); measured
against the English pack it is the same 1,861 files, the same `Converters`
and `Snapshot` folders, the same file names, with only the human text
changed — 4,930 parameter names and 1,575 state names carried in Cyrillic,
the rest being numbers, units and codes that are the same in both
(`600 Hz`, `Value = 7`), exactly as the English pack leaves them.

This layer is not the help layer, and the same move does not fit it. A
help screen's Russian joined to its English by the mnemonic's name,
because the name was a stable label beside the text. Here the English
*name is itself the thing translated* — a parameter's record is keyed by
its name (`ParameterDefinition { parameter }`), and a converter's state
names live inside the encoding string — so a join by the English text
would join a thing to its own translation and find nothing. What is stable
across the two packs is the structure, and it is identical: the same
`KeyedData` ids and `ReadParameter` ids (6,440 of them in the fully
qualified file, the two id sets equal), the same converter files, the same
state ranges — `lowValue`/`highValue` unchanged, only the state `<name>`
different (`Unknown/invalid` -> `Неизвестный/недействительный`).

1. **The Russian names are a channel beside the English, joined by
   structure, not by text.** The DID-formatting adapter, told the language
   (`DidFormattingAdapter::with_language`, as `DtcHelpAdapter` already is),
   writes for each parameter a record carrying only the Russian name,
   keyed by the same structural id the English record has
   (`…did.<keyed_slug>.p<parameter_id>`), under a source id that names the
   language. The English record — its key, its encoding, its applicability
   — is untouched, so a library issued before this date reads exactly as it
   does now. The converter catalogue, told the language, carries each
   state's Russian name joined by converter id and the state's range, so
   the decoded state can be shown in Russian without the English encoding
   string changing.

2. **The DID-formatting files carry their own language, and are checked.**
   Each `Snapshot/*.xml` declares `langcode="ru"`; the adapter refuses a
   file whose declared language is not the one it was told, as the help
   adapter refuses a mismatched `<language isoCode>`. The converter files
   declare no language, so — as with the description indexes of the
   2026-09-13 amendment — a pack mounted as Russian must be mostly Cyrillic
   and one mounted as English mostly not, and a converter root under the
   wrong name is refused.

3. **The name a car is shown follows the choice already there for SDD's
   text.** A parameter name and a decoded state name are SDD's own data,
   so they follow the English/Russian switch the help and the code
   descriptions already follow (`description_data_texts`'s choice), not the
   interface language: Russian when it is chosen and the pack has it,
   English otherwise — a name the Russian pack lacks stays English on that
   line rather than becoming a word about something else. Module names keep
   following the interface language, as `ADR-0034` decision 3 left them.

4. **Nothing is translated by this project, and Ukrainian is not written.**
   As with the help: the words shown are SDD's own, in the language SDD
   wrote them in; a Ukrainian reader sees Russian by default and can switch
   to English. The 2026-09-12 machine-translation finding stands — the
   reason this layer is worth having at all is that it lets the planned
   assistant quote SDD's own Russian name for a parameter unchanged rather
   than invent one.

5. **Redistribution is unchanged.** The Russian names travel in the issued
   library under `ADR-0019`'s stamp, restricted to the named tester, and
   enter neither this repository nor any committed path; the extracted pack
   stays in the owner's own archive. The synthetic fixtures carry invented
   Russian parameter and state names, as they carry invented English.

Consequences: `DidFormattingAdapter::with_language` and a language mode on
the converter catalogue; the exporter recognises a `_LANG_RU` snapshot
root and takes its `Converters` and `Snapshot` files under a language-named
source; the survey and the live-read decode carry the Russian name beside
the English and choose at display; the frontend shows the chosen language
for parameter and state names; the synthetic snapshot fixture gains a
Russian twin; the library is re-exported with the Russian snapshot root and
re-issued. Not in this amendment: the assistant itself (its own ADR, when
the owner calls it), and any language of this layer other than English and
Russian.

Built, 2026-10-03, the same day — on the owner's word («так»):

- *Decision 1.* `DidFormattingAdapter::with_language` writes, under the same
  record id the English catalogue builds from the `KeyedData` and
  `ReadParameter` ids, one record per parameter keyed `sdd_parameter.rus`,
  whose encoding text carries `name=<the Russian name>` and the converter's
  states named in Russian; the file's `langcode` and
  `ConverterCatalogue::states_mostly_cyrillic` are the two checks of
  decision 2. The resolver joins the two by the id's tail, the decoder
  names the state in Russian by the same raw count, and the survey's
  identifier lists, the module read and the live read carry the Russian
  beside the English; the interface chooses by the switch for SDD's text —
  our own wording first, then SDD's Russian, then SDD's English — and draws
  nothing new.
- *The library.* Re-exported with the Russian snapshot root: 1,854
  converters and 6 formatting files read in Russian; the DID catalogue
  doubled from 15,281 to 30,562 records, one Russian twin per parameter;
  533,208 records in all, nothing rejected. Re-issued to the owner as
  `2AFF-199F`, valid to 2027-10-03.
- *Tested.* The platform golden derives the twin by id and refuses a
  Russian file read as English and an English catalogue read as Russian;
  the session test surveys `SYNTHMOD`'s identifiers with their Russian
  names in the parameters' order; the decoder test names a state in both
  languages; the whole-stack bench test reads an identifier and finds a
  Russian name on every decoded parameter.
- *Later the same day — the passport, mileage and battery rows*, on the
  owner's question whether they could follow before the push. Each row of
  the three carries `parameter_texts`, SDD's name in its other languages
  beside the English identity, and the three panels show it by the same
  rule; no new control. The mileage rows take it from the catalogue's
  twins, as the live read does. The battery rows could not: their
  parameters come from the platform document, which exists in no Russian.
  So the exporter, which already joins the formatting document's byte
  description to the platform's row (`ADR-0030`), joins the Russian pack's
  name to the same row by the record id's tail — by structure, as decision
  1 — and the platform adapter records it as a twin under the English
  record's id with the language appended, keyed `sdd_parameter.rus` on the
  same module; the resolver joins it by the id it already joins the
  catalogue's twins by. In SDD 169 that names 38 battery rows of 18
  identifiers — the ones the catalogue describes at all: on the L322 and the
  X250 of MY10 the BCM declares fourteen, the catalogue describes two
  (`0x4028`, `0x4027`) and those two carry the Russian name; the other
  twelve SDD describes nowhere, in either language, so they stay bytes
  under their English name as before (`ADR-0030`). The mileage rows of
  both cars carry it, every one. The
  passport rows carry the name only where a module-scoped catalogue record
  of the same identifier bears the same English name, and that is nowhere
  in SDD 169: SDD names the identification identifiers in the platform
  document alone, in English, so those rows keep our own wording
  (`ADR-0027`, decision 5) or SDD's English, and no Russian is invented
  for them. The library is re-exported and re-issued as `0807-B35C`:
  533,775 records, the 567 battery twins among them.
- *Tested, later the same day.* The platform golden derives the battery
  twin by id and refuses a row that is not there and a language with no
  pack; the session test hands back the Russian name of a battery
  parameter and of a mileage, and none for an identification identifier;
  the whole-stack bench reads the battery voltage and the mileage with
  their Russian names and the passport without one; the three panels' tests
  show the Russian under the switch.
- *Not yet.* The browser demo carries no Russian names. Nothing has met a
  car.
