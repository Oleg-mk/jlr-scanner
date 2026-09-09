# ADR-0021: The product is named ProwlOne, and carries no manufacturer's marks

Status: accepted, 2026-09-09.

## Context

The product was called JLR Scanner and shipped with an icon that read
"JLR" in white letters. Both put a manufacturer's marks inside the
product's own identity: `JLR` is Jaguar Land Rover Limited's, and an
application named and badged that way reads as an official product of
theirs no matter what a README says underneath.

That was tolerable while two people held the build. It is not tolerable at
the point the owner is now weighing — a public announcement on an
English-language, UK-centric owners' forum, in Jaguar Land Rover's home
market, among readers who work in the trade. The name is also the cheapest
thing to change today and the most expensive to change later, once testers,
reports and links carry it.

A second option was considered and rejected: putting a manufacturer's
figurative mark — the leaping cat — on the icon. That is the strongest
form of the same problem, not a solution to it, and redrawing a similar
leaping cat for automotive goods would be judged on similarity, not on
whether it was traced.

## Decision

1. **The product is `ProwlOne`.** A name of our own: a big cat on the
   prowl, and the one tool. It names no manufacturer and no model.
2. **The tagline on screen names no marks:** "Multi-platform vehicle
   diagnostics", translated with the interface. The manufacturers are named
   in prose — README, product pages, a forum post — as what the product is
   compatible with, beside the disclaimer the README already carries. That
   is how a marked product may be named by an independent tool, and it
   keeps the marks out of the badge, the title bar and the installer.
3. **The icon is a mark of our own:** a white ring on the project's dark
   green, open on the right, with a trace crossing it and one leaf-green
   beat — the vehicle held, and the sign of life read out of it. Flat,
   geometric, four parts, which is what survives at 32 and 16 pixels where
   a picture does not. The owner's first drawing, a spotted cat on an
   off-roader, was rejected by him on the desktop for reading cheaply at
   size; that judgement is recorded here because it was right, and because
   the same trap waits for any illustrative mark.
4. **The name reaches everything a user sees or installs:** product and
   window title, bundle identifier `com.prowlone.desktop`, the shell crate
   and therefore the binary inside the macOS bundle (`prowlone-shell`),
   installer and disk-image names, saved report file names, the issuer
   written into new library stamps, the signing description, and the CI
   artifact names.
5. **The name deliberately does not reach data.** The session-report
   schema id `jlr-scanner.session-report` stays: copies already in
   testers' hands write it and `report-intake` checks it, so renaming it
   would refuse their reports. Recorded source ids of evidence, such as
   `jlr-scanner-mongoose-jlr-route-bindings`, stay, and so do the origin
   strings beside them ("JLR Scanner source code…"): provenance records what
   a claim came from, under the name the source bore when it was recorded,
   and rewriting it would be rewriting evidence. Crates named after what they
   hold — `jlr-profiles`, `mongoose-jlr` — stay. The dated history in
   `CURRENT_STATE.md` and `docs/evidence/` stays, because it names files
   that really were called that.
6. **Library copies issued under the former name stay valid.** A stamp is
   verified against the issuer written inside its own signed file, never
   against the constant in the code, so the copies issued on 2026-09-06 and
   2026-09-07 keep working until their dates. Only new stamps say
   `ProwlOne`.

## Consequences

- The bundle identifier changes, so Windows and macOS treat this as a new
  application: the previous build is not upgraded in place and should be
  uninstalled by hand. Two people hold it, so this costs nothing now and
  would have cost a migration later.
- A trademark search for `ProwlOne` in the classes that cover diagnostic
  software (UKIPO, EUIPO, the Ukrainian register) is the owner's step
  before the public announcement. Nothing in this decision claims it was
  done.
- The repository is still `Oleg-mk/jlr-scanner`. Renaming it on GitHub
  keeps a redirect from the old URL and is the owner's call, since it
  changes a public address.
- Nothing about safety, evidence or the read-only boundary changes here.
  This decision is about identity only.
