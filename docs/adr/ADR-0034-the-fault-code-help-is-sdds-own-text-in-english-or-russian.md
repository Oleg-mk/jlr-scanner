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
- Not decided here: SDD's Russian for the code *descriptions*
  (`dtcDescriptions.xml` in the same Russian pack). It exists and is not
  ingested by this decision.
