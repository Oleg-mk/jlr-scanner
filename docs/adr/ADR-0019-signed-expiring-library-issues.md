# ADR-0019: Signed, expiring library issues

Status: accepted, 2026-09-06; amended 2026-10-12 (the issuing service). Gate `G4` continued.

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

## Amendment, 2026-10-12: the issuing service — a personal card over one shared library

Every copy is handed over by hand today: the owner stamps it on his machine,
packs some 31 MB and sends it. With testers to be looked for beyond the club
(`ADR-0036`, amended the same day), he asked whether the library could be
fetched the way the installers are, with only a code handed out.

Three ways were weighed.

- *An encrypted library beside the installers on the public Releases page,
  opened by a code.* Refused: the file would sit on a public, permanent
  link under the owner's name, next to the code; the key would reach
  whoever was given a code and could be passed on, and anyone who reads the
  open source could take it out of the application; and a rights holder's
  complaint could close the whole repository, not the one file.
- *A service that stamps a whole copy per person, as the hand stamp does* —
  the code written into every one of some twelve thousand manifests and
  31 MB repacked for every download.
- *A service that signs a personal card over one shared set of bundles.*

The owner chose the third: «логіка і розум — 2 … якщо я роздаю бібліотеку
тестерам, то наступний логічний крок після отримання зворотнього звязку —
загальнодоступна бібліотека». A name inside every manifest is what makes a
copy traceable while it passes through a few hands; on the road to a
library for everyone it costs more than it gives.

1. **Two ways to issue.** By hand, as decided above and unchanged: the
   stamping tool writes the marker into every manifest and signs on the
   owner's machine with the owner's key. And through an issuing service,
   which this amendment adds.
2. **One shared set, a personal card.** The service keeps one set of
   bundles per edition of the library, exactly as the exporter writes them,
   with no marker, in private storage on a hosting account of its own that
   holds nothing else and is protected by two-factor sign-in. What is
   personal is the card: `issued_to.json` of schema version 2, as above —
   the name, the dates, the code, the issuer, the hashes of the shared
   bundles, signed. The application checks a card exactly as it checks a
   hand stamp; its loader does not change for it.
3. **The service signs with a key of its own.** A second Ed25519 key, made
   for the service and held only as its secret; its public half joins
   `TRUSTED_ISSUER_KEYS` beside the owner's, with its own key id. The
   owner's key never leaves his machine. Should the service's key ever be
   exposed, the next build drops it from the list; the builds before it
   keep trusting it, which is one more reason its cards carry dates.
4. **The code.** The owner makes one per tester, with the name and the
   validity, and the service knows the tester by it and by nothing else. A
   code is personal and works as a key: whoever holds it receives that
   person's card. It is long enough not to be guessed; a revoked or expired
   code receives nothing; the service keeps who received a card and when.
   The card's issue code is the code, so the library panel and every report
   name it as they name a hand-issued copy today.
5. **What is given up.** A copy from the service names its holder in its
   card only. Shared bundles passed on without their card name nobody; the
   application refuses them without a card, but their text can be read by
   other means. Accepted by the owner for the road ahead.
6. **What this does not decide.** The application's side — asking the
   service with a code and fetching the bundles — would be the
   application's first network request of its own; it gets its own record
   before its code. A library for everyone is a later and separate
   decision, after the testers' feedback, with its own record and a
   lawyer's view first. The way there is an edition of what has been
   confirmed on cars and written in this project's own words, which the
   provenance every record carries already allows to be selected.

Consequences of the amendment:

- The owner hands out a short code instead of an archive of 31 MB, and a new
  edition of the library is one upload, not one archive per tester.
- The copies already issued by hand stay valid until their dates.
- To build, after the record for the application's side: the service (the
  code check, the card, the access to the shared set), the tool that makes
  codes, the service key in the trusted list, and tests that a card over
  shared bundles loads while a card from an untrusted key, over a changed
  bundle, or past its date does not.
- Nothing here touches the safety boundary or the diagnostic path.
