# ADR-0024: The odometer, read from every module

Status: accepted, 2026-09-11. The owner's idea, taken after the catalogue was
read for whether the data supports it.

## Context

A car's mileage is not kept in one place. It is written by the instrument
cluster, and it is also written, independently, by modules that have no
business displaying it: the engine and gearbox controllers, the anti-lock
brakes, the airbag module, the parking-brake module, the tyre-pressure
module, the steering-angle sensor, the doors, the gateway, the telematics
unit. Each keeps its own count because each needs to know how far the car has
gone — for service intervals, for wear models, for its own fault histories.

Mileage fraud works because a tool rewrites the places the tool can reach.
The cluster always; the engine controller often; the other forty modules
almost never, because writing them is slow, model-specific, and pointless for
the seller. The disagreement is then the record of what happened.

**The catalogue was read before this was decided.** Of 15,281 identifier
parameters SDD describes:

| what | where |
| --- | --- |
| `0xDD01` **Total distance**, km | declared for **92 module families** over 21 vehicle programmes — ABS, PCM, TCM, RCM, IPC, PBM, TPM, SASM, AWDCM, GWM, TCU, the door modules, HVAC, the parking-aid module, and on down |
| `0x61BB` **Odometer store**, km | the instrument cluster, 23 records |
| `0xD018` / `0xD019` first and latest CAN-signal-timeout event log | each stamps the total distance at the moment of the event |
| `0x1EC2`, `0x1ED0`, `0x1EC9`, `0x1ECF` | gearbox stall, failed drive-gear selection, delayed park engagement, driver's door auto-park — each records **the odometer at the time of the incident** |
| `0xD91C` Revised Jaguar flight recorder data | carries an odometer field |
| SAE J1979 PID `0xA6` | the legislated odometer; already decoded by `obd-j1979`, rarely supported on SDD-era cars, free to try |

The event histories are the sharper instrument. A gearbox-stall record at
250,000 km on a car whose cluster reads 120,000 is not a disagreement between
counters; it is a reading from a future the car has not reached.

This is also the first capability the product would have that serves someone
who does not own the car yet. A cheap dongle reaches the engine controller at
a standard address and nothing else — it cannot ask forty modules anything.
Reaching every module on every bus is the whole reason this project exists.

## Decision

1. **One new operation, `MILEAGE_SURVEY`, of class `READ_ONLY`.** It reads
   identifiers the product is already permitted to read, from modules the
   survey already knows how to reach, with the same `prepare` step every
   single read passes. No new service, no new addressing, nothing written.
   It is the network check of F10 with one identifier instead of fault codes.

2. **Every module the survey can reach is asked.** The set is not chosen by
   hand: the point is breadth. For each surveyed module the loaded library
   lists `0xDD01` (or the cluster's `0x61BB`) for, a read is prepared and
   executed. Modules the data does not describe are not asked and are not
   counted against anything.

3. **The table reports, with the arithmetic done.** One row per module:
   its acronym, the identifier asked, what came back with the catalogue's
   own unit, and **the difference from the highest reading found on the
   car**. The difference is arithmetic over two numbers this product read
   itself — it is a fact about readings, not a claim about a person, and
   hiding it would only make a human subtract six-digit numbers in their
   head. The highest reading is the reference rather than the cluster's,
   because the cluster is precisely what gets rewritten; the cluster's row
   is marked as the number on the dashboard.

4. **Event stamps are shown as what they are.** A recorded incident carries
   a mileage and is not a current odometer, so those rows sit apart, each
   with its event named and its own difference: *mileage at the event minus
   the current highest reading*. A positive number there is an event the car
   has not yet driven to.

5. **No verdict is printed, ever.** The word for what a difference might mean
   does not appear in the product: not "tampered", not "rolled back", not
   "suspicious". A number in a column is the whole of what is claimed.
   One line under the table states the honest causes of disagreement — a
   replaced module is the common one, and a used cluster, a replacement
   gearbox or an airbag module fitted after a crash all disagree lawfully.
   Whoever reads the table draws the conclusion; the product does not.

6. **Silence is silence.** A module that does not answer is recorded as not
   answering, and a module that declines is recorded with its refusal. A
   zero in a mileage column would be a lie about a car, so nothing that did
   not answer gets a number.

7. **What must not be mistaken for an odometer.** `0xF421` *Distance
   travelled since the malfunction indicator lamp was activated* is a
   legislated counter that resets with the codes; it says nothing about a
   car's life and is excluded by name, not by pattern. Any parameter whose
   name carries *since*, *trip*, or *lamp* is excluded the same way. A
   feature that confuses those with an odometer would deserve the distrust
   it earned.

8. **The readings are evidence of exactly what a read is evidence of.** Each
   row goes to the session report with its module, route, identifier, raw
   bytes and decoded value, carrying the marks any single read carries. On
   the bench everything is `SYNTHETIC` and the intake refuses the bundle, as
   it refuses any bench bundle. A mileage survey from a car proves the
   modules answered as recorded; it does not make anything
   `VEHICLE_CONFIRMED` on its own.

9. **Where it lives.** The shell composes what exists: the survey for the
   module list and routes, `ModuleReadService::prepare` per module, the
   adapter service per request, the catalogue for the decode. The
   `JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI` line holds unchanged, and
   no new crate is needed.

## Consequences

- A careful forger who writes every common identifier in every module still
  passes. Ninety-two modules and the event histories make a complete job
  expensive and an incomplete one visible; that is the honest limit and both
  tester guides will say it in the same change that ships the feature.
- The product acquires a use before ownership: a person going to look at a
  car has a reason to install it. That widens who the tester programme can
  reach, which is the constraint the roadmap names first.
- Every report tells us which modules actually answered on which programme —
  something SDD does not record and only cars can teach.
- The run puts one request per module on the bus, once. It is the network
  check's traffic, not a live read's.

## Built, 2026-09-11; read by the intake since 2026-09-12

The survey keeps, per read, the record a single module read leaves, and the
intake reads it as `mileage_surveys[i].reads[j]` through the builder a
module read goes through — so a survey of forty modules is forty pieces of
evidence that those modules answered at those addresses on those routes,
and nothing about mileage, which was never evidence of anything. Two
faults of the first build were found by the review of 2026-09-12 and are
in `CURRENT_STATE`: the reference mixed units, and the report's rows were
not marked on the bench.

## What this does not decide

- **Writing a mileage anywhere.** Not now, not in stage 2, not with a
  confirmation dialog. Correcting an odometer is the fraud this feature
  exists to expose, and the product will not gain the ability to commit it.
- **A judgement, a score, or a flag.** No traffic light, no "risk", no
  percentage. If a later version wants to say something about a set of
  readings, that needs its own ADR and its own evidence.
- **Units.** The catalogue's unit is shown as the catalogue gives it, and a
  record without scaling shows raw counts labelled raw. Converting between
  kilometres and miles, and deciding which a given market's module means, is
  a separate question that needs a source better than an assumption.
- **A vehicle history.** One reading is one moment. Nothing here builds a
  timeline, and the report carries readings, not a story.
