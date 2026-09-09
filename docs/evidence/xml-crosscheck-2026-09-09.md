# Cross-check of the SDD XML against the exported library, 2026-09-09

The library's breadth comes from one parser. If that parser misreads an
address or a byte range, nothing downstream notices: the survey, the read
plan and the report all agree with each other and all are wrong. So the same
source files were read a second time, by a separate program in another
language, written from the shape of the XML rather than from the Rust code,
and the two readings were compared.

The reader is `scripts/xml-crosscheck/crosscheck.py`. It shares no code with
`sdd-ingest`. Its detailed output quotes source values and is therefore
written to a local file, never into this repository; what is recorded here is
the method, the counts and the findings.

## What was compared

| | |
|---|---|
| Platform documents | 55 (`CURRENT_JLR_XCL_XML_DATA_XML`, `CURRENT_JLR_MCP_XML_XML`) |
| DID-formatting documents | 6 (`COMMON_SDD_DATA_SNAPSHOT_LANG_EN`) |
| Module rows read from the XML | 770 `(programme, module)` |
| Module rows in the library | 742, plus 60 this project derives itself |
| Read parameters read from the XML | 4,127 |
| Read parameters in the library | 3,799 |

**Addresses.** For every module a platform document declares: the diagnostic
request and response CAN identifiers. Compared as the set of pairs per
programme and module, because mapping a file name to SDD's year breakpoint is
itself parsing and a difference there would drown the thing being checked. A
misread digit still shows, because it puts a value in one set and not the
other.

**Field widths.** For every read parameter a DID-formatting document
declares: the byte range and the mask, against the `identifier_definition`
encoding the library carries. Compared as sets, because the four formatting
documents can define the same parameter more than once.

## Result

```
addresses     agree 711 | disagree 8 | only in the xml 51 | only in the library 23
field widths  agree 3799 | disagree 0 | only in the xml 328 | only in the library 0
```

**Field widths: nothing wrong.** Every byte range and mask the library
carries is one the XML declares. 3,799 of them. This closes the question of
whether we misread a width.

**Addresses: one real defect, and it is ours.** The rest of the differences
are explained below and are not errors.

## The defect: a module's qualified addresses collapse to one

A platform document may declare the same module acronym several times, each
under a `<qualifier>` naming an engine type, a build year or a market, with a
different diagnostic address. Nine module rows in the corpus do this. The
ingest writes one record per document and module — the record id is
`sdd-169-platform-<document>.module.<FAMILY>.addressing`, with no qualifier
in it and none carried into `applicability` — so only one address survives,
and in both cases examined it is the last the document declares.

What this costs, concretely:

- **X250 MY10** declares the instrument cluster at `0x7B2/0x7BA` for the V6
  petrol and the 4.2 V8, and at `0x720/0x728` for the 5.0 V8, the
  supercharged V8 and the V6 diesel; the parking-brake module likewise at
  `0x7B4/0x7BC` and `0x756/0x75E`. The library holds only the second address
  of each. On an X250 of 2010 with the V6 or the 4.2, both reads would go to
  the wrong address and the module would appear silent. On the 5.0
  supercharged — the owner's own car — the surviving address is the right
  one, by luck rather than by correctness.
- **L405 and L494, MY14 and MY16** declare the ABS module at `0x760/0x768`
  for cars built in 2014 and at `0x7E6/0x7EE` for 2015 and later. The library
  holds only `0x7E6/0x7EE`. L405 appears to hold both only because a
  different document, MY13, supplies the other.
- **X404** loses three rows the same way, but X404 is not ingested at all
  (see below), so nothing depends on it today.

The fix is not architectural: `applicability` already carries `powertrain`,
`market` and `other`, which is where the qualifier belongs, and the record id
needs the qualifier so that two rows no longer collide. It changes the
exported library, so it changes what issued copies contain.

## The differences that are not errors

- **51 module rows only in the XML.** They belong to `X101`, `X103`, `X206`,
  `X356`, `X358` and `X404` — platform documents whose own `platform_name`
  declares a programme the ingest does not carry. `X404` is folded into
  `X400` deliberately (`F9`, 2026-09-07); the other five are a coverage gap,
  not a misreading. Worth a decision of its own, separately from this check.
- **23 module rows only in the library.** `X100`, `X202`, `X350` and `X400`
  rows whose source is one of those variant documents, recorded under the
  base programme; plus modules the project derives rather than reads.
- **60 derived rows set aside.** The 29-bit `normal_fixed` identifiers of
  `ADR-0017`. The XML is not their source, so comparing them would only
  manufacture noise.
- **328 parameters only in the XML.** Read parameters the library does not
  carry. Not examined in this pass; none of them contradicts the library.

## Two mistakes in the checker itself, corrected during the run

Recorded because they are the kind of thing that makes a cross-check lie:

1. The programme was first taken from the file name. `PLATFORM_X204.xml`
   declares `X206`; the document names itself and the file name does not
   always agree. Now the declaration wins.
2. Addressing records whose entity is a whole vehicle programme rather than a
   module were compared as if they were modules, and the derived 29-bit rows
   were compared against an XML that never held them. Both are now set aside
   explicitly.

## Reproducing

```bash
python scripts/xml-crosscheck/crosscheck.py \
  --platforms <SDD>/CURRENT_JLR_XCL_XML_DATA_XML \
  --platforms <SDD>/CURRENT_JLR_MCP_XML_XML \
  --snapshot <SDD>/COMMON_SDD_DATA_SNAPSHOT_LANG_EN \
  --library <exported library> \
  --report <a path outside this repository>
```

It exits non-zero when anything disagrees.
