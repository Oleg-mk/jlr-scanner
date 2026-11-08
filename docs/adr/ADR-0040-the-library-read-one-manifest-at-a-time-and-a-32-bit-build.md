# ADR-0040: The library read one manifest at a time, and a 32-bit Windows build after it

- Status: **Accepted, 2026-11-08**, on the owner's word the same morning
  («так, приймаю, починай»), proposed hours earlier on his word to take the
  task and begin with this record («так, беремо, починай з ADR»), after the
  first tester from the forum turned out to have only a 32-bit laptop.
  Decisions 1, 2 and 4 built and decision 5 measured the same day; see
  the end.
- Scope: how the application loads a library directory, a tool that cuts
  an exported library to named programmes, and a second Windows installer.
  The library's content, the stamp's format, the safety boundaries and the
  diagnostic path are not touched.

## Context

The first tester to answer the call of 2026-10-12 runs a small JLR workshop
and has used SDD for years. His workshop laptop is a Panasonic Toughbook
CF-19 that runs only 32-bit Windows 10 — the classic SDD machine — and he
has no 64-bit one. The Windows installer is x64 only, and no 32-bit build has
ever been made.

A 32-bit build alone would not be enough. Measured on 2026-11-08 with the
release build of the `survey` example, which loads a library directory
exactly as the application does, sampling the process's working set:

| library | records | peak working set | load time |
| --- | --- | --- | --- |
| full, as issued | 533,775 | about 5.1 GB | 62 s |
| cut to his seven programmes, English and Russian | 355,049 | about 3.5 GB | 27 s |
| cut to his seven programmes, English only | 260,458 | about 2.4 GB | 13 s |

The seven programmes are the ones he sees most: X250, X152, L319, L494,
L405, L538 and L359. The running application was seen holding 2.6 to 3.5 GB
with the full library loaded. A 32-bit program on 32-bit Windows has 2 GB of
address space in all, and the same peak is a burden for the 64-bit laptops
with 4 GB that many workshops keep.

The peak is mostly the loader's, not the library's. Today
`KnowledgeLibrary::load_directory_with`:

1. reads every bundle's decompressed text into memory before ingesting any
   of it — 830 MB in all, 503 MB of it the fault-code help — because the
   stamp's hashes are checked first and the texts are kept for the ingest
   that follows;
2. parses each bundle into a generic JSON tree (`serde_json::Value`), which
   costs several times its text;
3. serialises each manifest of the tree back to a string and parses that
   again, into the typed manifest the store takes;
4. ingests it.

So the texts, the largest tree and the growing store are resident together.
The store itself has room to shrink too: in the help bundle the
applicability of each record is 144 MB of the records' 398 MB of text — the
same few constraints repeated on 315,660 records — but whether that needs
touching can only be told after the loader stops dominating the peak.

## Decision

1. **The stamp is checked by streaming.** Before anything is ingested, each
   file is decompressed as a stream and hashed as it passes; no text is
   kept. What is checked and every refusal stay as `ADR-0019` has them: a
   copy that is not signed by a trusted key, does not match every bundle,
   or is past its date loads nothing.
2. **Then one manifest at a time.** Each bundle is read again as a stream,
   and its manifests are deserialised one by one straight into the typed
   manifest — no generic tree, no second serialisation — then validated by
   the very rules the JSON manifest adapter applies today (the adapter is
   split so that both paths call the same validation) and ingested before
   the next one is read. The bytes are hashed again as they are read; a file
   that no longer matches what was checked discards the whole load, and the
   library refuses as it does for a mismatch today.
3. **Measure, then decide about the store.** After 1 and 2 the peak and the
   resident size are measured on the full library and on the cut copy of
   decision 4, with the numbers written into this record. If the store
   itself is still too large for the 32-bit budget of decision 5, sharing
   the repeated applicability between records is the next lever — as an
   amendment to this record, not a part of it.
4. **A cut copy.** A tool cuts an exported library to named programmes and,
   if asked, to English only. A record stays when it applies to any
   programme or to one of the named ones; the evidence its records cite
   stays with it; a manifest left with nothing goes. The cut copy is stamped
   like any other (`ADR-0019`), and its scope is written into the name it is
   issued to, so the library panel shows it. The application changes
   nothing for it: a programme the copy does not cover is simply not
   offered.
5. **A 32-bit Windows build, once it fits.** When a cut English copy for the
   tester's programmes loads with a peak under 1.5 GB in a 32-bit build —
   measured on this host, where a 32-bit process meets the same 2 GB
   ceiling as on his laptop — CI builds a 32-bit installer beside the
   64-bit one and the Releases page carries it. Until then nothing 32-bit is
   published, and nothing is promised to him beyond the attempt.
6. **Nothing else changes.** The library's content, the stamp's format,
   the safety classes and the diagnostic path stay as they are.

## Consequences

- Every user gains the lower peak, and the full library loads on a 64-bit
  laptop with 4 GB with room to spare, which it does not reliably do now.
- The tester guide states a memory requirement once one is measured; it
  states none today.
- The stamp check reads each file twice, once to hash and once to ingest:
  decompressing some 30 MB twice costs seconds.
- A 32-bit build measured on this host is `IMPLEMENTED` and measured, not
  `HARDWARE_CONFIRMED` on a CF-19: the tester's own run is what confirms it,
  and his laptop's own memory, shared with Windows and the web view, may
  set a lower ceiling than the address space does.
- Not decided here: sharing applicability in the store (decision 3), any
  Windows older than 10, and a 32-bit macOS, which does not exist.

## Built, 2026-11-08, the same day

- *Decisions 1 and 2.* `KnowledgeLibrary::load_directory_with` reads each
  file twice and keeps neither reading: the first pass hashes the
  decompressed text as it streams past, keeps 16 KB for the issue code and
  checks the whole is JSON; the stamp is checked against those hashes by
  `issue::verify_digests`, the rules of `ADR-0019` unchanged. The second
  pass hands each manifest of a bundle to the ingest as its own text
  (`serde_json`'s raw values over a streaming reader) and hashes the bytes
  again; a file that differs from its first reading discards the load. The
  built-in manifests, the tests and the tools take the same path, and the
  survey over the full library is byte for byte what the old loader gave.
- *Decision 3, measured.* Peak working set of the release `survey`
  example, 64-bit: the full library 5.1 → 4.1 GB; the seven programmes in
  both languages 3.5 → 2.7 GB; the same in English 2.4 → 1.9 GB. The load
  alone of the full library peaks at 4.1 GB too: what remains is the store,
  not the loader. Sharing the repeated applicability between records is the
  next lever, as this record said, and is not built — the tester's case
  did not need it.
- *Decision 4.* `cut_library`, an example of `diagnostic-session`, cuts an
  exported library to named programmes and, with `--english-only`, drops
  the Russian packs' manifests; it reads each bundle through the loader's
  stream and writes as it goes. The Discovery 4 alone, in English:
  533,742 → 118,522 records in 4,307 manifests, 7.8 MB packed, 14 s. The
  copy was stamped for the tester with its scope in the name,
  «… — Discovery 4 (L319), English», for three months.
- *Decision 5, measured.* The `i686-pc-windows-msvc` target builds the
  workspace, the application and its NSIS installer unchanged. On that
  copy, 32-bit: `stamp_library --check` peaks at 625 MB; a probe of the
  phases shows the load peaking at 656 MB and settling at about 350 MB, a
  survey of the car adding a transient of about 280 MB, the bench vehicle
  the same, nothing above 660 MB. The application itself, run by the
  owner with that copy, reached about 700 MB resident after the load and
  about 1.05 GB with the survey and the panels open — under the 1.5 GB of
  this decision, on a machine where the address space does not bind. A
  reload of the library took the same run to a peak of 2.97 GB: the shell
  held the old library through the load of the next. It now drops it
  first. CI builds the x86 installer beside the x64 one, as a matrix of the
  Windows job; the Releases page carries both from the next build.
- *The tester guide* says what the measurements allow: 64-bit Windows or
  macOS for the full library, with 8 GB to be comfortable; 32-bit Windows
  10 only with a copy cut to one or a few programmes, 4 GB being enough
  for one.
- *Not done.* Decision 3's store work; any run on a real 32-bit machine —
  the tester's CF-19 is the first, and his report is what confirms it.
