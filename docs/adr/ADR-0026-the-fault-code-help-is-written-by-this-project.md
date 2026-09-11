# ADR-0026: The fault-code help is written by this project

Status: accepted, 2026-09-11. The owner's decision, after a disagreement this
document records rather than hides.

## Context

F14 gave the product a help layer: for a fault code on a particular car, the
screen SDD would show — possible causes, actions required, the conditions
under which the module sets the code. It is read from the issued library,
where it travels as `sdd_help` and `sdd_help_screen.<name>` claims, and it is
in English whatever language the interface is set to.

SDD publishes that layer in twelve languages and none of them is Ukrainian.
Measured on 2026-09-11: the Russian pack holds the same 3,452 files as the
English one, with identical tag counts and identical `dtcDescriptionId`
values, and only the human text changed.

The layer is 26,893 named texts filling 193,520 screen slots — short lines,
41 % of them 60 characters or fewer, reused 7.2 times each on average.

**The owner wrote all of it, in three languages.** English restated from what
the fact is, then Ukrainian and Russian. Two rounds, both measured against the
source the same way:

| | first | second |
|---|---:|---:|
| English lines of 80+ characters identical to SDD's | 4,751 | **8** |
| Russian lines of 80+ characters identical to SDD's | 8,200 | **0** |
| English identical overall | 68.8 % | 51.1 % |
| Russian identical overall | 71.1 % | 40.5 % |
| Ukrainian cells carrying a form Ukrainian cannot make | 250 | **40** |

What stayed identical sits under 80 characters, where a short instruction has
few natural forms — `Reconnect the injector and start the engine.`

Two faults were then corrected by this project's assistant and logged line by
line: 40 Ukrainian cells where Russian words had survived into the sentence,
rewritten whole rather than word by word; and 1,980 rows across the Ukrainian
and Russian columns where a common word had been left capitalised in the
middle of a sentence — `Перевірте Наявність DTC` — lowered only where the same
word is written in lower case at least three times elsewhere in the same
column, which no name ever is.

**The disagreement, recorded because it was real.** The assistant argued that
rewording does not detach the work from its source, because what binds it is
the selection — which 26,893 facts, attached to which codes — and that the
layer should therefore travel in the issued library as SDD's English already
does, rather than be committed to a public repository. The owner's answer, and
it is the decision: this is technical literature describing a machine. The
facts and the necessary procedures are dictated by the car, not chosen by an
author; for an injector-circuit code the causes *are* the injector, its
wiring, its connector and its driver, and anyone competent writing from
scratch arrives at the same list. A phrase being the natural way to say a
thing does not make it someone's property. The assistant's concern is stated
here once and is not re-argued.

## Decision

1. **The help text is this project's own, in three languages, and it is what
   the product shows.** Our English replaces SDD's on screen as well; a layer
   where the English reader sees one text and the Ukrainian reader another
   would be two products, not one.

2. **It lives in the repository**, beside `parameter_names.tsv` and
   `dtc_standard_text.tsv`, by the same mechanism: a tab-separated file
   compiled in with `include_str!`. 25,393 rows — one per distinct line, SDD
   itself carrying the same text under more than one name — 12.12 MB, which
   takes the installer from 4.78 MB to roughly 6.7 MB. Where the owner gave
   one line two renderings, 27 times, the one that fills more screens is kept.

3. **The key is a fingerprint, not the source line.** A help line arrives from
   the library as SDD's English, and that is how the row is found — but the
   table stores a 64-bit FNV-1a digest of the line rather than the line, so
   no sentence of SDD's is reproduced in this repository. The table stays
   readable because our own English sits in the next column. Whitespace is
   collapsed on both sides before hashing, because the ingest trims a
   mnemonic's text without flattening it.

   The alternative was to key by SDD's mnemonic name, which is an identifier
   and not text. It was not taken because the library does not carry the
   names — the ingest resolves them to text before the claim is written — so
   it would mean changing the ingest, re-exporting and re-issuing the library
   to every tester. The corpus is frozen at release 169, so a fingerprint
   cannot go stale.

4. **The library still decides what a car is shown.** Which screen a code gets
   on this model and year, and which lines that screen holds, remains the
   library's answer. Only the words are ours.

5. **Fail closed, one line at a time.** A line whose fingerprint is not in the
   table is shown as the library has it, in English. No machine translation,
   no partial substitution, and no falling back to the other language.

6. **It is a dictionary, not knowledge**, exactly as `ADR-0025` has it: no
   evidence, no applicability, no validation state, never in the knowledge
   store. It cannot move what the product believes about a car.

7. **The report is in English and carries ours.** A report is evidence and
   does not change language; the English it carries is now the product's own
   wording rather than SDD's.

## What it covers, measured

Counted against the issued library rather than against the corpus, because the
library is what the product reads:

| | |
|---|---:|
| Help screens the library holds | 36,983 |
| Distinct lines those screens show | 37,225 |
| Lines with wording of ours | 22,903 |
| **Share of all the places a line fills** | **88.0 %** |
| Screens read wholly in the user's language | 24,168 (65 %) |

The gap has a cause worth writing down, because it was this project's
assistant's mistake in preparing the work. The file handed to the owner was
built from SDD's mnemonic list — the 26,893 named texts. The library's screens
are those texts split on the newlines inside them, so a line the library shows
need not be a named text at all: `Control module pin:` appears on 311 screens
and exists nowhere in the corpus as a unit of its own. Those lines could not
have been offered for translation, because as units they did not exist.

Stitching consecutive lines back together before looking them up was measured
and rejected: it raises coverage from 88.0 % to 88.4 % and costs a matcher
that can mis-segment a screen. The plain per-line lookup stands.

The remaining 14,322 lines are listed in the owner's own order of usefulness —
by how many screens carry each — and translating the first few hundred closes
most of the gap. Until then they are shown in English, one line at a time,
which is what fail-closed means here.

## Consequences

- Ukrainian gets a fault-code help layer that has never existed in any tool.

- The installed application grows by about 12.6 MB. The lever, if that ever
  matters, is to store the table compressed and unpack it on first use;
  `flate2` is already a dependency (`ADR-0023`). Not done now, because a
  plain file is diffable and the size is affordable.

- A line the owner has not written is shown in English beside lines that are
  translated. That is the fail-closed behaviour and it is visible, which is
  the point; the count belongs in the state document, as `ADR-0025` requires
  of its own table.

- SDD's English help text stays in the issued library, where it already is and
  where it is needed to find a row. Nothing about the library changes.

## What this does not decide

- **The parameter names are untouched** and keep the arrangement `ADR-0025`
  gave them.

- **`COMMON_SDD_DATA_RULES`**, SDD's symptom-driven tree, 78 MB over 56
  documents, remains uningested.

- **No fourth language.** The table has three columns; a fourth is a question
  about who maintains it.
