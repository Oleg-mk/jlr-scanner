# ADR-0023: The issued library travels compressed

Status: accepted, 2026-09-10. Owner's decision after the numbers below.

## Context

The exported library is a directory of JSON manifests. A tester downloads it,
the application reads every file at start, and the stamp of `ADR-0019` says
the copy is the one that was issued.

It has grown. On 2026-09-10 the fault-code help layer was ingested — SDD's own
possible causes, actions required and monitoring conditions, per code, per
model, per model year, per fault type. Measured on the real corpus:

| | before | after |
| --- | ---: | ---: |
| Records | 131,501 | 242,691 |
| `dtc_help.json` | 127.0 MB | 275.7 MB |
| The whole directory | 170 MB | 311 MB |

311 MB is what a tester would download, keep, and hand back in a zip, and it
is what the application reads at every start — on the owner's 2016 Mac, which
already took about twenty seconds over the old size.

The manifests are JSON: long repeated key names, a verbose applicability block
of about five hundred bytes on every record, and English prose. Measured with
gzip at level 6 on this machine: **`dtc_help.json`, 263 MB, compresses to
9.7 MB in one second — 27 times.** The whole directory lands somewhere near
fifteen to twenty megabytes.

The size is not the help layer's fault. It is the packaging. Shipping 311 MB
of JSON to save one dependency is the wrong trade for a product whose data is
handed to volunteers over whatever connection they have.

## Decision

1. **A manifest may be stored compressed, as `<name>.json.gz`, gzip.** Gzip
   because it is everywhere, it streams, every operating system opens it, and
   one small dependency reads it. No archive format, no bundle-of-bundles: one
   file per manifest as today, only compressed.

2. **The loader accepts both.** A directory may hold `.json`, `.json.gz`, or a
   mixture, and the loader reads each accordingly. Libraries issued before
   this decision keep working, a hand-written manifest stays possible, and a
   tester who decompresses a file to look inside is not punished for it —
   provided the stamp is re-made, since the file set changed.

3. **The stamp covers what a manifest says, not how it is packed.** The
   `ADR-0019` stamp records, per file name, the SHA-256 of the manifest's
   JSON — decompressed. A library therefore stamps to the same hashes whether
   it is packed or not, and a re-compression with different settings does not
   invalidate an issue. The file name in the stamp is the name on disk, so the
   set of files is still exact: adding, removing or renaming one is still a
   mismatch, and the AppleDouble sidecars of a Mac are still ignored by the
   rule that skips names beginning with a dot.

4. **The export writes compressed.** `export_manifests` produces `.json.gz`;
   the stamping tool reads whatever it is given and writes the same form it
   read, so an issued copy stays as small as the export made it.

5. **Nothing else changes.** The manifest format itself, the record shape, the
   ingest, the trusted keys, the validity dates and the refusal messages stay
   as they are. This decision is about bytes on a disk and nothing else.

## Consequences

- The issued zip drops from about 311 MB to about 20 MB. A tester downloads
  it in seconds rather than minutes, and the owner mails a link that works.
- Start-up reads about a twentieth of the bytes and spends about a second
  decompressing them. On a slow disk that is a gain, not a cost; it will be
  measured on the owner's Mac once a compressed library is issued.
- One dependency, `flate2`, enters the workspace. It is the standard Rust
  gzip crate and pulls a pure-Rust deflate backend.
- A tester cannot read a manifest in a text editor without decompressing it
  first. They never did — a 127 MB JSON line was not readable either — and
  gzip is a double click on every platform the product supports.
- The old uncompressed libraries stay loadable, so nothing already issued
  stops working the day this ships.

## What this does not decide

- **Compressing anything else.** Session reports, captures and the diagnostic
  reports a tester sends back are small and stay plain JSON, because a person
  reads them and a maintainer greps them.
- **A different format for the manifests themselves.** A binary encoding would
  be smaller again and would cost the property that makes this base
  reviewable: that every record is legible text with its evidence beside it.
  If that is ever wanted it needs its own ADR and its own reason.
- **Trimming the help layer.** The records duplicated nothing that could be
  removed without losing what SDD says; the size was packaging, and packaging
  is what this fixes.
