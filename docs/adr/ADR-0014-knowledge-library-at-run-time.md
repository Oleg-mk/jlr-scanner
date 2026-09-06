# ADR-0014: The knowledge library reaches the application as manifests

- Status: Accepted. Recorded in the same change as its implementation rather
  than before it, which the working discipline forbids; the lapse is noted
  here rather than hidden.
- Decision: The application's runtime dependency is the **knowledge base**,
  loaded as F5 JSON manifests from a directory the user names, plus the two
  documented manifests built into the binary. SDD stays what `README.md` and
  `docs/research/sdd/PROVENANCE.md` say it is — a research reference the
  owner's own tooling reads — and is never installed, bundled, or read by the
  shipped application.

  Concretely:

  1. `crates/diagnostic-session` is the F10 composition layer: it loads a
     `KnowledgeLibrary` and surveys a vehicle into `app-contracts` snapshots.
     The Tauri shell holds it as state behind three commands —
     `get_data_library`, `load_data_library`, `survey_vehicle` — and does
     nothing else.
  2. A manifest file is either one F5 manifest or a **bundle**, a JSON array
     of manifests. Each element is ingested on its own, atomically, so one bad
     element costs only itself and is reported by name; nothing is dropped
     silently.
  3. `KnowledgeStore::ingest` no longer clones the store to be atomic. It
     validates the batch against the store plus itself in a first phase that
     mutates nothing, then inserts. Loading thousands of manifests is linear,
     not quadratic. `ingest_batch` exposes the same path for already-parsed
     batches, and `IngestionBatch` serialises to exactly the manifest shape,
     so anything an adapter produced can be written out and loaded back.
  4. `sdd-ingest`'s `export_manifests` example turns a locally extracted SDD
     tree into bundles, one per adapter kind, validating the whole set as one
     store first — exactly what the application will do on load. Its output
     lives outside the repository.
  5. The vehicle is described in SDD's own terms — programme, model year,
     breakpoint marker, engine — typed in, until VIN decoding exists. What the
     user does not state is "not stated", never "any".

- Reason: F10 made a module SDD describes resolvable, but the shell had no
  store at all; it knew one hard-wired X250 profile from F8. Something had to
  carry the knowledge into the application, and the recorded rules bound the
  choice: SDD content is never committed, never redistributed, and never a
  runtime dependency, while the F5 knowledge base is the product's own store
  of record, already serialisable through `JsonManifestAdapter`. Loading
  manifests at run time satisfies every rule at once. Ingesting SDD XML inside
  the application would have made SDD a runtime dependency; bundling derived
  manifests into the binary would have decided a distribution question that is
  the owner's, not this decision's.

  The atomic-without-clone ingest was forced by arithmetic. The corpus is
  thousands of manifests and over a hundred thousand records; cloning the
  store per manifest is quadratic and would have turned a few seconds into
  hours. Two-phase validation keeps the guarantee the tests already state — a
  rejected manifest leaves no partial write — at linear cost.

- Consequence: `diagnostic-session` may depend on `app-contracts`,
  `diagnostic-environment`, `knowledge`, `uds-execution` and `serde_json`,
  and on nothing in transport, protocol, backend, OS or UI. The architecture
  checker enforces the manifest rule. The frontend names none of it; the
  checker's existing frontend scan forbids the crate names, so the UI speaks
  of a "data library".

  **Open owner gate, `G4`.** Whether manifests derived from SDD may be given
  to community testers — and under what terms — is a legal and product
  question this decision does not answer. Until it is answered, the exported
  library exists only on the owner's machine, the repository carries only
  documented and synthetic manifests, and the tester programme cannot survey
  real vehicles. `docs/ROADMAP.md` records the gate.

  Live execution is unchanged: the survey computes plans and reads knowledge;
  no command opens a transport or transmits.
