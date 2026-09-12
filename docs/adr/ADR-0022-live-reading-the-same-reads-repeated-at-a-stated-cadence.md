# ADR-0022: Live reading — the same read-only reads, repeated at a stated cadence

Status: accepted, 2026-09-10. Owner's decision in discussion; no code yet.
The owner names this the start of the 1.1 line.

## Context

Every read the product makes today is one shot, on a button press: a
`ReadDataByIdentifier` (`0x22`) for one identifier of one module, a
`ReadDTCInformation` (`0x19`), the J1979 calibration identification, or a
listen on one bus for a stated number of seconds. That is the right shape
for a survey and for a report, and the wrong shape for watching a value: a
tester who wants to see coolant temperature climb, or the four wheel speeds
side by side while ESP complains, has to press the button again and again.

The owner asked, on 2026-09-10, for visualisation — gauges, a car seen from
above with its wheels and doors lit, cards for fault codes — and listed six
blocks: engine and gearbox, fault codes, fuel and emissions, electrics and
battery, chassis and comfort, and the CAN bus itself. Before answering, the
exported library was read for what it actually holds. Of its 15,281
identifier-parameter definitions (4,977 distinct names), the blocks are
served like this, counted by parameter and by the module SDD scopes it to:

| what | parameters | where |
| --- | --- | --- |
| fuel trim | 1,207 | PCM |
| battery, voltage | 1,204 | PCM, TCM, HVAC |
| temperatures | 834 | PCM, HVAC, FCDIM |
| throttle | 253 | PCM |
| gear, transmission | 215 | TCM, PCM |
| steering angle | 217 | SASM, BCM |
| boost, manifold | 162 | PCM |
| doors, bonnet, tailgate | 131 | PDM, DDM, RDM |
| vehicle speed | 98 | BCM, TCM |
| engine speed | 86 | BCM, RDCM |
| wheel speeds | 55 | RDCM, AWDCM, TCCM |
| coolant temperature | 55 | — |
| tyre pressure | 13 | BCM |
| particulate filter | 12 | PCM |
| fuel level | 8 | PCM |
| airbags, restraints, seat belts | **0** | — |

Four of the six blocks are served, and served **per module** — which is the
capability a cheap OBD-II dongle lacks: it reaches the PCM at the standard
address and nothing else, while the library names the steering-angle
sensor module, the rear differential module, and four door modules
separately. Two blocks are not served: restraints, for which the catalogue
holds nothing, and the traction battery of an electric or hybrid car, which
the SDD era barely has.

Three facts bound what can be promised. First, **no read has yet been made
from a real vehicle**: every value the product has ever shown came from a
fixture, a replay or the bench. Second, **the cadence is unknown**: each
parameter is one request and one response over ISO-TP, through the
MongoosePro framing, and nobody has measured how many of those the adapter
carries in a second. A gauge that promises ten updates a second and delivers
one is worse than a table. Third, **SDD holds no signal map**: it says which
identifier to ask a module for and how to decode the answer, not how the
same value is laid out in the frames modules broadcast to each other. Values
can be asked for; they cannot be overheard.

Since 0.9.7 every shell command runs off the window's thread
(`CURRENT_STATE.md`, 2026-09-10), which is the precondition for anything
that runs for minutes while a person watches.

## Decision

1. **One new operation, `LIVE_READ`, of class `READ_ONLY`.** A live read is
   a loop of requests the product is already permitted to make — today
   `ReadDataByIdentifier` for identifiers the loaded library lists for a
   module, later SAE J1979 mode 01 (decision 7) — sent one after another to
   a chosen set of module-and-identifier pairs, at a stated cadence, until
   stopped. No new service enters the `uds` crate's request constructors.
   The loop runs in the default session: it never sends
   `DiagnosticSessionControl` (`0x10`), never sends `TesterPresent`
   (`0x3E`), never asks for security access. An identifier a module answers
   only in another session is unavailable to the live read, with that
   reason, exactly as `SAFETY_BOUNDARIES.md` requires: unknown fails closed.

2. **The live set is chosen from the library, never typed.** A set is a list
   of `ModuleReadRequest`-shaped entries — module family and identifier —
   each of which passes the same `prepare` step that validates a single
   read today, so the interface still cannot supply bytes, service numbers
   or addresses. A set holds at most sixteen entries; one request is in
   flight at a time; entries are visited in order, round after round.

3. **The cadence is stated, measured and shown — never assumed.** Two
   constants live in one place in the shell: the shortest gap between two
   consecutive requests, initially 100 ms, and the longest a live read may
   run without being started again, initially ten minutes. Before any gauge
   is drawn, the achieved rate is measured on the bench and then on the
   first hardware session, and recorded in `CURRENT_STATE.md`. The interface
   shows the round time it actually achieves beside the values, so a
   parameter that refreshes every two seconds is shown to refresh every two
   seconds.

4. **Stopping is always possible and always quick.** The loop takes the
   adapter for one request and releases it, never for its own lifetime —
   unlike a capture, which holds it for the whole listen — so a stop, a
   disconnect, or any other command gets through between two requests. The
   loop stops itself on an adapter error, on disconnect, and at the time
   cap. An entry that fails three rounds running — silence or a negative
   response — is dropped from the set with its reason; when the set is
   empty the loop stops. A live read never survives a new session.

5. **Every sample is evidence of exactly what one read is evidence of.** The
   session report gains a time series: for each sample, milliseconds since
   the run began, module, identifier, raw bytes, and the value decoded with
   the catalogue's converter and unit, carrying the same validation marks a
   single read carries. On the bench every sample is `SYNTHETIC`, and
   `report-intake` refuses the bundle as it refuses any bench bundle. A
   series from a car proves that the module answered at that address on
   that route, as a single read proves it, and says nothing more: no
   `VEHICLE_CONFIRMED` follows from a live read on its own, and the meaning
   of a value is the catalogue's decode, not ours.

6. **What is drawn is what the data holds.** Values with the catalogue's
   units and named states; the smallest and largest seen in the run; a
   trend over the run; and, for the modules SDD names, their place on the
   network map the product already draws — wheels, doors, steering as nodes
   that light up. A parameter without a converter is shown raw and labelled
   raw. Nothing is drawn for which the library holds nothing: no restraint
   status, no traction battery. A gauge is added only for a value whose
   achieved cadence (decision 3) makes a gauge honest.

7. **Standard J1979 live data is a separate slice.** `obd-j1979` gains mode
   01 (current data: engine speed, vehicle speed, coolant temperature,
   throttle, intake pressure, fuel trims, fuel level, MIL status and the
   fault-code count) and mode 02 (freeze frame), both read-only and
   standard-defined, so they are proven on the bench without a car and work
   on any car with an OBD-II port, not only a JLR. They complement the
   manufacturer identifiers of decision 1; they do not replace them, because
   the standard reaches one module and the library reaches every one.

8. **Where it lives.** The loop is an execution concern of the shell: a
   `LiveReadService` composes today's `ModuleReadService::prepare` and the
   adapter service per request; the set comes from knowledge — the loaded
   library's catalogue; the interface renders. `JLR KNOWLEDGE != PROTOCOL
   != TRANSPORT != UI` holds as it stands, `diagnostic-execution` still does
   not depend on `uds`, and no new crate is needed to begin. The bench
   answers `0x22` in a loop already and gains mode 01 with decision 7, so
   the shell's end-to-end test drives a live run on every commit.

9. **Order.** (a) This ADR. (b) The cadence measured on the bench. (c)
   `LiveReadService`, the recording, and a plain table of values — no
   gauges. (d) The visual layer of decision 6. (e) Mode 01 and 02. Steps
   (a), (b) and (e) need no car. Steps (c) and (d) are better begun with one
   real session report in hand, because they build over a read path that
   has never met a vehicle; the first tester report outranks them.

## What this does not decide

- **Overhearing values** from broadcast frames. SDD gives no signal map, and
  a map guessed from traffic would be an assumption recorded as a fact. If
  one is ever wanted it needs its own ADR and its own source of truth.
- **The traction battery** of an electric or hybrid car: out of the era the
  product serves; nothing in the library, nothing on screen.
- **Restraints and seat belts**: nothing in the library, nothing on screen,
  until the library says otherwise.
- **Bus error counters** (TEC, REC): those belong to the adapter, and
  whether the MongoosePro exposes them is unverified. Bus load computed
  from the product's own capture is a small matter and is not held back.
- **Use in a moving car.** The live view is a diagnostic instrument for a
  standing vehicle, engine running or not. Both tester guides will say so
  in the same change that ships it.

## Consequences

- A live read puts continuous request traffic on a diagnostic bus of a real
  car — at the initial floor, at most ten requests a second, which is what a
  scan tool does. The cap, the stop and the per-request lock of decision 4
  exist for that reason and are not optional.
- The session report grows with every run, bounded by the cadence and the
  time cap; the intake reads the series as it reads today's observations.
- The measured cadence decides the design of the visual layer. Nothing about
  gauges is settled until decision 3 has numbers.
- Whether any of this works on a car is learned the way everything else is:
  from a tester's report. The bench proves the software; a vehicle proves
  the vehicle.

## Measured, 2026-09-10

Decision 3 asked for numbers before a gauge; they are in
`docs/evidence/mongoose-link-cadence-2026-09-10.md`. The link to the
adapter answers a request in a median 15.6 ms; opening and closing a route
costs about 50 ms; the product's own read path costs 0.01 ms a read on the
real library. The cadence is therefore the link and the module, never the
software. Two things follow for the service of decision 8: it keeps the
route open for the run and closes it on stop, and the 100 ms floor of
decision 3 is the binding limit by choice — a courtesy to the bus, ten
requests a second, below what the link would carry. The module's own answer
time waits for a car.

## Built, 2026-09-10: the loop steps, and the route is not held

Decision 9(c) is in the product. Two things about the shape of it differ
from what the measurement above suggested, for reasons the code makes
unavoidable; they are recorded here rather than left as a surprise.

**The route is opened and closed per request, not held for the run.** The
device refuses a second open while one is open (`RouteAlreadyOpen`) and
refuses to close the transport with a route open (`ActiveRoute`). Decision
4 requires the loop to release the adapter between requests so a stop, a
disconnect or any other command gets through; a route left open across that
release would therefore fail every other command in the application, and a
disconnect with it. Decision 4 is a safety decision and wins. The cost is
the ~50 ms of open and close measured above, paid once per request; the
100 ms floor remains the binding limit as claimed, and because decision 3
shows the achieved round time beside the values, the cost is visible rather
than hidden.

**The loop's timer is the interface's; its rules are the shell's.** The
`LiveReadService` owns the set, the order, the floor, the time cap, the
failure counting and the samples; the interface asks for one step at a
time. A step that arrives sooner than the floor allows is refused, so no
interface — or fault in one — can make the product ask a bus faster than
decision 3 permits. This keeps the per-request lock of decision 4 exact,
keeps the whole loop testable without a thread, and lets the bench
end-to-end test drive a run deterministically on every commit. What
decision 8 places in the shell — the composition of `prepare` and the
adapter service per request — is in the shell.

**Decision 5 was not true until 2026-09-12.** The run recorded its samples and
the intake ignored the run: a tester's live session contributed no evidence.
Now each entry of the set keeps the record of its last completed request in
the shape a single module read leaves — the same builder, `read_record`,
serves all three — and the intake reads it through the same path, under
the name `live_read_runs[i].set[j]`. One record per entry, not per sample:
the module answering at that address on that route is proven once, and
repeating the request proves nothing new. Found by the review of the ADRs
against the code, recorded here because the decision said otherwise for
two days.

Nothing else changes: `LIVE_READ` is `READ_ONLY`, the set is chosen from
the library's readable identifiers and never typed, at most sixteen
entries, one request in flight, no `0x10` and no `0x3E`, an entry that
fails three rounds running is dropped with its reason, and every sample
carries the validation the single read carries — `SYNTHETIC` on the bench.
