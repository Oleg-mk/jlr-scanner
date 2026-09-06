# ADR-0019: Signed, expiring library issues

Status: accepted, 2026-09-06. Gate `G4` continued.

## Context

The library is the product's key, and by `G4` it never travels inside the
installer: each tester receives a personal copy. Since 2026-09-05 that copy
carries a stamp — `[issued CODE]` in every manifest and `issued_to.json`
beside the bundles with the name, the date, the code and each bundle's
SHA-256 — which the application reported but never enforced: a tester with
a doubtful copy could still test, and the report said which copy it was.

On 2026-09-06 the owner weighed what protects the product while the base
is still largely SDD-derived, and decided: a tester's copy lives one month,
is handed over with a request not to copy or pass it on, and the month is
checked by the application, not by conscience. A stamp anyone can write by
hand cannot carry that; it needs a signature only the owner can produce.

## Decision

1. **The stamp is signed.** `issued_to.json` becomes schema version 2:
   `issued_to`, `issued_on`, `valid_until`, `issue_code`, `issuer`, the
   bundle hashes, and an Ed25519 `signature` over a canonical text of those
   fields, with the `key_id` of the signing key. The private key lives on
   the owner's machine, outside the repository, and is never committed; the
   public keys the application trusts are a list in `diagnostic-session`,
   so a key can be rotated by adding the new one.
2. **The stamp expires.** `valid_until` is an inclusive UTC date; the
   stamping tool sets it thirty days after the issue date unless told
   otherwise. The application compares it with the machine's date. Setting
   the clock back is not defended against: the aim is to make copying
   traceable and stale, not impossible.
3. **The application fails closed.** The shell loads a directory only
   through `load_issued_directory`, which refuses — built-in data stays,
   nothing from the folder is ingested — unless the stamp is present,
   signed by a trusted key, matches every bundle and is not expired. The
   refusal names its reason and what to do: ask for a new copy. Tools,
   examples and tests keep the unenforced loader, which still reports the
   stamp as before.
4. **The stamping tool issues, checks and makes keys.** `stamp_library`
   signs with the key named by `JLR_ISSUER_KEY`, `--check` loads a folder
   exactly as the application would, `--new-key` makes a key pair and
   prints the public half for the trusted list.

## Consequences

- The owner's own working copy must be stamped too; a year's validity
  for himself is one command.
- An expired or hand-edited copy no longer loads. The panel says so in the
  tester's language; the session report of a loaded library carries the
  issue with its validity.
- `F13`'s stamp test reverses its premise: the loader now refuses, and the
  test proves each refusal and the one acceptance.
- Reports remain readable whatever the stamp: they are files, not the
  library.
- Nothing here touches the safety boundary or the diagnostic path.
