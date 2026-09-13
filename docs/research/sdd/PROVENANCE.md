# SDD research artifact provenance (public summary)

The knowledge this project derives from SDD traces to one artifact:

- File: `SDD_169.00.001_FULL.exe`, SDD version `169.00.001`, a late SDD
  release, obtained by the owner on 2026-09-02 from Jaguar Land Rover's own
  delivery host (`diagnosticdelivery.jlrext.com`, on the same `jlrext.com`
  domain as the official TOPIx portal). The installer download does not
  require a diagnostic subscription; connecting to vehicles through JLR's
  services does.
- Size: 3,497,211,865 bytes. SHA-256:
  `B56B0D8426F532F3C217D3D481AA97A218C0B902C77B986FEDF561DF3EF48CB8`.
  JLR distributes the installer unsigned, so authenticity rests on the
  delivery channel and this hash.
- Container: InstallShield media, read with the installer's own
  `/extract_all` switch and `unshield`; 159,510 files across 232
  components. The installer's `FLASH`-named components carry the ECU
  firmware — `.vbf` payloads and a few `.iso` images. The boundary this
  project enforces is at **ingestion** and at the **repository**:
  `sdd-ingest` never reads a firmware file, none enters the knowledge
  base, and the repository is verified to hold zero `.vbf`, `.iso` or
  other firmware, with no crate depending on one (`ADR-0005`). The
  owner's own local research extraction does contain them — 371 `.vbf`
  and 4 `.iso` across six `FLASH` components in `SDD169_EXTRACTED` —
  because a full `unshield` writes every component to disk. An earlier
  revision of this note claimed the extraction itself contained no
  `.vbf` and named a count of 479; that was wrong about the disk, and it
  is corrected here rather than quietly, because a provenance chain that
  hides its own mistakes is worth less.

## Handling rules

- The installer and any SDD content are never committed to this
  repository, never redistributed, and never a runtime dependency. Only
  derived records (`sdd-ingest`, `RestrictedMetadataOnly`), provenance and
  synthetic fixtures enter the repository; the derived library itself is
  issued to named testers under a signed, dated stamp (`ADR-0019`).
- Derived records are `documented` evidence with provenance pointing here.
  Nothing extracted from SDD is vehicle evidence, and none of it becomes
  `VEHICLE_CONFIRMED` without a real vehicle's answer.
- Programming-session addressing present in the platform documents is not
  ingested at all: a route recorded in the knowledge base is a route
  something can later be built on.
- This project's code performs no decryption and handles no keys.

## What the corpus supplied, in short

- `COMMON_SDD_DATA_SNAPSHOT_LANG_EN`: 5,382 read-only DID definitions with
  bit layouts and 1,854 typed converters.
- `COMMON_SDD_DATA_DTC_HELP_LANG_EN`: 6,171 fault-code help documents.
- `COMMON_SDD_DATA_DTC_HELP_LANG_RU`: the same 6,171 documents in Russian,
  opened from the installer by the owner's own scripts on 2026-09-13 (every
  file checked against the cabinet's MD5, the installer's SHA-256 confirmed);
  read for the text of each help screen and, since the same day's
  amendment, for the two description indexes (`ADR-0034`).
- `COMMON_SDD_DATA_ODST_LANG_EN`: 93 per-module test definitions.
- `COMMON_SDD_DATA_ODST_LANG_RU`: the same 93 documents in Russian, opened
  from the installer by the owner's own scripts on 2026-09-13; read for the
  text of each help screen only (`ADR-0034`).
- `CURRENT_PAG_UTILS_RUNTIME/CANLinkMonitorData.xml`: addressing width per
  programme and year, and the global module-to-identifier map.
- `PLATFORM_<PROGRAM>_<YEAR>.xml`: 110 platform documents with bus rates,
  identifier widths, gateways, sub-networks and diagnostic identifiers for
  1,418 module entries over 89 modules and 22 programmes, plus 18,294
  further DID definitions with access classes (`READABLE`, `WRITEABLE`,
  `CONTROLLABLE`, `SECURITY_REFS`) — the machine-checkable stage-1 boundary.
- `VINDecode.xml`: 33 model decoders over 2,473 values.

Every text-bearing family above is published once per language — twelve of
them, Russian among them and Ukrainian not — with the same record keys in
each. Only the English packs have been ingested.

The full, per-file provenance record — every path, count and date of the
survey — is kept privately by the owner and can be shown to a reviewer.
Two predictions this project's assistant made about where the addressing
lived were wrong before it was found in the platform documents; the
private record keeps them, because a provenance chain that hides its own
mistakes is worth less.
