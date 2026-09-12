# ADR-0031: The readable report — printed, saved, handed over

- Status: Accepted, 2026-09-12 (the owner took the next row from the
  capability matrix: "Друк, PDF, надсилання звіту")
- Decision: the session gains a **second artifact with a different reader**.
  The session bundle stays exactly as it is — one JSON document, the
  evidence, what `report-intake` reads. Beside it the application now
  renders the same session as a document a person reads: the car, the
  library it was read with, the modules and their reasons, every read of the
  session with its values, and the caveats that belong to each. It prints
  from the application, saves as a web page any browser turns into a PDF,
  and is what a tester attaches to a message or hands to the person whose
  car it is.
- Reason: the bundle is written for a machine and reads like one. A tester
  sending results, a workshop showing an owner what was found, and the owner
  of this project reading a report six months later all want prose, tables
  and a page break — not a JSON tree. The matrix row said *partly — the
  bundle file*, and that is the whole gap.
- Consequence: no new operation, no new safety class, nothing new recorded.
  The readable report is built in the interface from the bundle the shell
  already returns, in the language the interface is in, and leaves through
  the one command that writes files — so the bench still writes nothing to
  disk. The printing is the platform's own: the application asks the
  webview to print, and the print dialog offers whatever printers and
  PDF writers the machine has.

## Decisions

1. **One source, two shapes.** The document is rendered from the session
   bundle and from nothing else. Anything not in the bundle is not in the
   report; anything in the bundle that the report leaves out is a decision
   written here, not an accident. What it leaves out today: the raw response
   bytes of every read (the bundle keeps them; a printed page of hex helps
   nobody), and the library's per-manifest failure list beyond its count.

2. **Every section carries its own worth.** A reading printed on paper
   outlives the screen that explained it, so each section repeats what its
   values are worth: the route's validation state, `SYNTHETIC` where the
   bench answered, and one line saying that nothing in this document is
   vehicle-confirmed by being in it. A bench session prints with a band
   across the head of the document and the word on every table.

3. **No verdicts, the same as everywhere else.** The document says what the
   modules answered. It does not say a battery is weak, a code is
   important, a configuration is wrong, or a car is healthy. Fault codes
   carry SDD's own description and this project's own help text where there
   is one, both attributed; a code with no description says so.

4. **The VIN and the adapter's serial can be masked.** The guide already
   asks a tester to say if they want the VIN masked in what is published;
   a switch on the report does it before the document is printed or saved,
   showing the last four characters only. The bundle is untouched: masking
   is a property of this document, not of the evidence.

5. **Printing is the platform's.** The application calls the webview's own
   print. On Windows that is WebView2's print dialog, where "Microsoft Print
   to PDF" is a printer like any other; on macOS the same dialog offers
   "Save as PDF". Where a webview refuses to print — and one may — the
   saved web page is the answer: any browser prints it, and the button that
   saves it sits beside the button that prints.

6. **Sending stays a human act.** The application does not send mail, does
   not upload, and creates no account: `README.md` and the tester programme
   are clear that a tester sends the file through the channel they received
   the build from. What the application does is make that easy — it saves
   the file, says where it put it, and offers to open that folder so the
   file can be attached.

## What this does not decide

- A template a workshop can brand, or a layout beyond one clean document.
- Printing anything but the whole session: a single module's page, a single
  read, a comparison of two sessions.
- Turning the document into evidence. It is a rendering; the bundle is the
  record, and `report-intake` reads that and nothing else.

## Built

**2026-09-12 — `IMPLEMENTED / FIXTURE_TESTED`.**

- `printableReport.ts` builds the document from the bundle and from nothing
  else: the car, the adapter, the library with its issue stamp, the survey
  with each module's reasons, and a section per kind of read — module reads,
  passports, mileage, battery, configuration, the live set — each with what
  its values are worth in its own heading and a caveat under its table.
  Raw bytes and the library's failure list stay in the bundle.
- `PrintableReport.tsx` renders it and carries its own stylesheet inside the
  element, so the saved page prints the same anywhere with nothing behind it.
- The report panel gained the controls: prepare, print, save as a web page,
  and a switch that masks the VIN and the adapter's serial in the document
  while the bundle keeps them. Printing calls the webview's own dialog;
  saving goes through the one command that writes files, so the bench still
  writes nothing and the button says why.
- `reveal_in_folder` shows a file the application has just written in the
  system's file manager, so it can be attached to a message. It refuses a
  path that is not a file, opens nothing, and sends nothing.
- Checked by 6 interface tests: the sections and their worth, a bench
  session marked across the head and on every table, the mask leaving the
  bundle alone, a session that recorded nothing, the document in the user's
  language, and one that fails if the document ever draws a conclusion.

See `CURRENT_STATE.md`.
