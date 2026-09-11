# ADR-0025: The parameter names are the product's own words

Status: accepted, 2026-09-11. Decided by this project's assistant at the
owner's delegation, after both candidate translations were measured.

## Context

Every number this product reads from a car arrives with a name, and the name
comes from SDD's snapshot catalogue in English: `Total distance`, `Short term
fuel trim sensor 2 bank 1`, `Turbocharger/supercharger waste gate solenoid A
duty cycle`. The library holds 15,281 such parameter definitions under 4,977
distinct names, and the live-read table, the mileage tables and every module
read show them raw, in English, whatever language the interface is set to.

The application's own words have been translated since F11. The vehicle
data's words followed a narrower rule, stated in
`docs/F11_TESTER_APPLICATION.md`: SDD's text database carries module names and
failure-type wording in twelve languages, so the Russian interface shows
SDD's Russian for those, and the Ukrainian interface shows English, because
inventing a Ukrainian rendering of JLR's text is a translation this project
had no source for. Parameter names were outside even that: English in all
three interfaces.

Two sources for a translation turned out to exist, so this had to be decided
rather than assumed.

**SDD's own, measured 2026-09-11.** The corpus is published once per language
— `SNAPSHOT`, `DTC_HELP`, `ODST`, `RULES`, `QUAL`, `APP_HELP`, `PDF`,
`SYMPTOM_HELP`, each in `DE EN ES FR IT JA KO NL PT PT_BR RU ZH`. The English
and Russian parameter catalogues were walked and compared on the documents'
own keys — `KeyedData id` plus `ReadParameter id`, identity rather than text
similarity:

| | |
|---|---:|
| Parameters in the English component | 15,269 |
| Parameters in the Russian component | 15,269 |
| Keys present in both | 15,269 (100 %) |
| Text left in English | 0 |
| Distinct English names | 4,976 |
| …carrying more than one Russian rendering | 0 |

There is no Ukrainian, and there never will be: the twelve are what JLR
published.

**The owner's own.** All 4,977 names translated into Russian and Ukrainian,
delivered as one workbook on 2026-09-11. Measured against the library and
against JLR's Russian:

| | |
|---|---:|
| Names in the library | 4,977 |
| Names in the file | 4,977 |
| Present in one and not the other | 0 in both directions |
| Blank, or left in English | 0 |
| Ukrainian rows identical to the Russian | 14, and each a genuine coincidence (`Реле стартера`, `Клапан продувки`) |
| Rows where the two Russians disagree | 4,607 of 4,976 (92.6 %) |
| `bank` rendered one way, owner | 254 of 254 (`банк`) |
| `bank` rendered one way, JLR | 249 `блок`, 5 `ряд` |
| Mean name length, owner / JLR | 69.8 / 78.3 characters |
| Names over 70 characters, owner / JLR | 2,448 / 2,987 |

Where the two were read side by side, the owner's was the more accurate:
`Total distance` reads `Общий пробег` against JLR's `Общее расстояние`, which
is wrong for an odometer and is precisely the parameter `ADR-0024` reads from
every module; `Power mode` reads `Режим питания` against `Режим мощности`.
JLR's `блок` for a cylinder bank would also sit on the same screen as `блок
управления` for a control module, where it reads as the other thing.

## Decision

1. **Both languages come from the owner's file, and JLR's Russian is not
   used.** This rejects a complete, free, manufacturer-authored translation,
   so the grounds are recorded above and are all measured: exact coverage
   including SDD's double spaces, one rendering per term where JLR has two,
   shorter names for a column that shares its row with a value, better
   meaning where the two were compared, and one voice across Ukrainian and
   Russian — which JLR cannot give at any price, having no Ukrainian.

2. **It lives in the repository, not in the issued library.** The table sits
   beside `crates/diagnostic-session/data/dtc_standard_text.tsv`, the same
   shape and the same mechanism: a tab-separated file, the English name then
   Ukrainian then Russian, compiled in with `include_str!`. It can be
   committed, reviewed and diffed because it is the owner's own work and not
   SDD content — the rule that keeps the corpus out of the repository is
   about JLR's text, and this is not JLR's text. 4,977 rows, 1.49 MB, 0.15 MB
   compressed on the wire, against the 326 KB the existing table already
   costs.

3. **It is a dictionary, not knowledge.** It never enters the knowledge
   store. It carries no evidence, no applicability, no validation state and
   no source type, because it makes no claim about any car: it says how an
   English phrase reads in Ukrainian, and nothing about what any module does.
   No new `SourceType` is added. A translation must never be able to raise or
   lower what the product believes about a vehicle.

4. **The English is the identity and never leaves.** The report records the
   English name whatever the interface language, exactly as `codeText`
   already does for fault codes: a report is evidence, and evidence does not
   change language. A tester quoting a parameter to another tester quotes the
   English.

5. **Fail closed, one name at a time.** A name absent from the table shows
   its English. No machine translation, no word-by-word substitution, and no
   falling back to the Russian when the Ukrainian is missing — a missing
   translation is visible, not papered over.

6. **Text only.** A translation replaces one displayed string. It never
   touches a value, a unit, an identifier, an address, a decoder note or a
   route. Nothing in this decision reaches a vehicle, and it carries no
   safety class for the same reason.

7. **The file is taken as delivered.** It renders one English term two ways
   in one place: `duty cycle` is `рабочий цикл` in 43 names and `коэффициент
   заполнения` in 6, and the Ukrainian splits the same way. The owner chose
   to keep his file as it stands, and the 43 agree with SDD's own Russian,
   which uses `рабочий цикл` in all 35 of the names it shares. Recorded here
   so that it reads as a decision and not as an oversight.

## Consequences

- Ukrainian becomes a full interface for vehicle data for the first time. The
  narrower rule in `F11` — Ukrainian keeps SDD's English — now holds only for
  module names and failure-type wording, and that document must say so.

- A screen can carry JLR's Russian for a module name and the owner's Russian
  for a parameter name. That is a seam, it is small, and unifying it is not
  decided here.

- If a future SDD release renames a parameter, its translation is orphaned
  and the English shows. That is the fail-closed behaviour working, but it is
  silent unless counted, so the ingest must report how many library names
  find no row — a number, in the state document, not a warning nobody reads.

- JLR's Russian stays on the owner's disk as a cross-check and is not
  shipped. It caught nothing in this file; it may catch something in the
  next revision.

## What this does not decide

- **Fault-code descriptions and the help layer stay English.** SDD ships both
  in twelve languages, keyed identically, and the Russian pack is on the
  owner's disk. Using it is a library question — the text is JLR's, so it
  would travel as claims in the issued library, not as a file in this
  repository — and it needs its own decision.

- **`COMMON_SDD_DATA_RULES`**, SDD's symptom-driven tree, 78 MB over 56
  documents, remains untouched and uningested.

- **No other language.** The table has three columns. A fourth is a decision
  about who maintains it, not a decision about code.
