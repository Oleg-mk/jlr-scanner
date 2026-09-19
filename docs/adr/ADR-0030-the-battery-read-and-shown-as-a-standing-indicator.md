# ADR-0030: The battery, read and shown as a standing indicator

- Status: Accepted, 2026-09-12 (the owner took the next row from the
  capability matrix and set the shape: "не просто виводимо інфу, це
  критичний показник особливо при роботі зі сканером… виводимо все що нам
  доступно", shown as a card in the session's own column)
- Decision: the vehicle battery becomes a first-class read of its own —
  operation `BATTERY_STATE`, class `READ_ONLY` — and a standing card in the
  session column rather than a row lost among two thousand parameters. What
  the battery monitor holds is read from the modules SDD says hold it, each
  value shown as the type it is where the data describes its bytes and as
  the bytes themselves where it does not, grouped by what a person actually
  asks: how full, what it is doing now, what it leaks when parked, how it
  has aged, and what the car remembers about it. The traction battery of a
  hybrid is its own group, from its own module.
- Reason: a diagnostic session runs on the car's own battery, and a session
  is exactly when it is weakest — ignition on, engine off, a tester drawing
  from the same rail. Every module read this product makes can fail for one
  reason that has nothing to do with the module: the battery sagged. A
  scanner that shows a module's silence without showing the rail behind it
  makes the person guess. This is also where SDD-derived knowledge earns
  its place: a cheap dongle reads one voltage, while the platform documents
  name, per car and per module, the whole battery-monitor dataset — state of
  charge, estimated temperature, quiescent current, cumulative charge and
  discharge, time in service, monitor resets, cold cranking voltage.
- Consequence: a rule in the knowledge layer says which identifiers are
  battery parameters and what each is for; the platform ingest records them
  per module, joining SDD's own byte description where it has one; the
  library gains a battery query; the shell gains a stepping service and the
  session bundle a `battery_reads` block the intake reads; the interface
  gains a card in the session column. Nothing is written: battery
  registration and monitor reset stay forbidden, and the reset **counter**
  is read, never set.

## What SDD says, measured

Read from the 45 platform documents on 2026-09-12, and from the DID
formatting document the catalogue is built from:

| | |
| --- | --- |
| identifiers a first word match found | 61 |
| of those, the battery: the curated table | 59 |
| module-identifier pairs, after the rule | 106 |
| programmes | 17 |
| modules that serve them, after the rule | `GWM`, `BCM`, `RSJB`, `BECM`, `FSJB`, `IPC`, `PSCM` |
| set type they arrive under | `NET` (the module's own set, under its "Additional PIDS") |
| a dedicated `type="BATT"` set exists | yes, and it holds **one** identifier — battery current — for `BCM`, `GWM` and `RSJB` in 30 documents |
| identifiers the DID formatting document also describes byte by byte | 18 |

The dataset itself, as SDD names it: state of charge (`0x4028`) and the
lowest it has been (`0x4035`, `0x41C3`); estimated temperature (`0x4029`);
current (`0x4090`, `0x402B`); quiescent current, low and high range and the
previous 15 minutes and 24 hours (`0x4025`, `0x41D0`, `0x401D`); cumulative
charge and discharge with the ignition on, off, and after shutdown
(`0x401E`, `0x4021`, `0x4026`, `0x401C`, `0x409E`); amp-hour charge loss
(`0x402E`) and the lowest calculated amp-hour value (`0x4047`); time in
service in days (`0x4027`); the number of battery replacement or monitor
resets (`0x4020`); estimated cold cranking voltage at the present state of
charge (`0x41EA`); battery type (`0x4058`, `0xEF19`); the generator's voltage
set point (`0x0304`); statistics blocks for charge balance, state of charge
and quiescent current (`0x41E4`, `0x41E5`, `0x41E6`, `0x414B`, `0x414C`,
`0xEEB0`, `0xEEB1`, `0xEEBB`, `0x40D7`, `0x4094`, `0x40A2`); monitor sensor
and alternator fault counters (`0x40FC`, `0x414D`); the quiescent relay box
and its event counters (`0x41EF`–`0x41FB`); the odometer at the last five
battery power shutdowns (`0x422C`); the second battery of a dual-battery car
(`0x41CD`, `0x41DA`); and, on the `BECM` of a hybrid, the traction battery —
temperature, state of charge, pack voltage, minimum and average module
voltage, variation in both, leakage resistance with the contacts open,
charge and discharge power limits, maintenance mode (`0x4800`, `0x4801`,
`0x480D`, `0x4814`–`0x4818`, `0x483E`, `0x483F`, `0x4840`, `0x4841`).

The two the word match found and the rule refuses are the turbocharger's
valve offset values, whose name contains "charge", and a parameter of the
`PCM` that shares a number with a battery one elsewhere. `PCM` is therefore
not in the list above at all: its only match was the turbocharger. That is
the double check of decision 1 doing exactly what it is for.

**What is not there.** SDD's 12-volt dataset names no *state of health* and
no *internal resistance*: the nearest thing it keeps is the estimated cold
cranking voltage at the present state of charge, beside the amp-hour charge
loss and the lowest calculated amp-hour value. Leakage resistance exists
only for the traction battery. This product will not compute a health
percentage out of parts SDD did not put together, and will not present one.

**SDD's own voltage bands** live in its `BatteryMonitor.ini`: below 11.6 V
low, 11.5–13.0 V medium, above 12.8 V high (the overlaps are SDD's own).
They are a fact about SDD, not a measurement of a battery.

## Decisions

1. **What counts as a battery parameter is a rule, and it lives in the
   knowledge layer.** `knowledge::battery` holds the curated table:
   identifier, the role it plays, and whether it belongs on the card's face.
   A parameter is admitted only when the identifier is in the table **and**
   the name SDD gives it on that module names the battery or the charging
   system — the double check that keeps `0xDE05 Turbocharger valve offset
   values` out, which a word match on "charge" would have let in. An
   identifier outside the table is not a battery parameter, however it is
   named. The precedent is `ADR-0024`: the rule that tells a mileage from a
   counter lives beside the catalogue, not in the shell that draws the table.

2. **The ingest records them per module, as the platform document scopes
   them.** The DID formatting document describes the bytes of 18 of these
   identifiers but names no module for any of them, so its rows resolve as
   *insufficient evidence* and reach nothing. The platform document names the
   module, the programme and the breakpoint marker but not the bytes. Joined,
   they make a readable parameter: the platform ingest writes one
   `ParameterDefinition` per module and battery identifier, scoped to that
   module and that car, carrying the formatting document's own encoding and
   unit where it has them and the platform's own name and nothing else where
   it does not. The join is described in the record's own locator and
   description, as `ADR-0028` did when it pulled the configuration's titles
   from the text database.

3. **A value without a description of its bytes is shown as bytes.** 18
   identifiers decode into a number with a unit — state of charge in per
   cent, temperature in degrees with SDD's own −40 offset, current in amperes
   at 0.0625 per bit, time in service in days, cold cranking voltage,
   generator set point, and the dual-battery bits. The rest are shown as the
   bytes they are, under SDD's own name for them, which usually carries the
   unit in its text. Nothing is scaled by guess, and no unit is inferred from
   a word in a name.

4. **One operation, the stepping shape the product already has.**
   `BATTERY_STATE`, schema `prowlone.battery-state`, class `READ_ONLY`: one
   `ReadDataByIdentifier` per module and battery identifier, planned through
   the same resolver, executed through the same prepared transaction, each
   read leaving the record a single module read leaves, so the intake turns a
   tester's battery read into evidence exactly as it turns a passport read
   into evidence. The shell owns the plan and the rules; the interface owns
   the timer.

5. **The card judges nothing, and says whose words it uses.** The state of
   charge is drawn as a bar with its number; the groups sit in the battery
   panel, in the working area, because thirty-odd rows are a panel's worth
   of reading and not a rail's (the owner, the same evening: "картку між
   кроками і автомобілем зробити лише таку, а всі параметри вивести в
   основне поле"); a module that answers nothing says so;
   a module that answers nothing says so; a car whose data names no battery
   parameter gets the card with that sentence and no empty gauges. Where a
   band is shown behind the voltage, it is SDD's own band from its battery
   monitor's configuration, named as SDD's — this product does not invent a
   threshold, does not colour a battery red, and does not say a battery is
   bad. What it does say is when a reading was taken, because a battery
   reading five minutes old during a diagnostic session is a different fact
   from a fresh one.

6. **Nothing is written, and the word *reset* appears only as a count.**
   Registering a new battery, resetting the monitor, clearing the statistics
   and setting the battery type are writes; they are out of stage 1 and out
   of this ADR. `0x4020` is read as *how many times this was done*, and the
   architecture check keeps a write constructor out of the battery path as it
   does everywhere else.

## What this does not decide

- A health percentage of any kind, and any arithmetic that would produce one.
- Live repetition of the battery read at a cadence: `ADR-0022`'s live read
  already repeats identifiers, and whether the card should drive it is a
  separate decision, taken when a car has been seen.
- The legislated `0x42` control-module voltage of `ADR-0022` §7, which the
  standard-OBD panel already reads, joining the card.
- Anything about the traction battery beyond showing what `BECM` answers:
  no cell balancing, no capacity estimate, no range arithmetic.

## Built

**2026-09-12 — `IMPLEMENTED / FIXTURE_TESTED`.** Nothing has met a car.

- *The rule.* `knowledge::battery`: 59 identifiers with their roles
  and whether each belongs on the card's face, admitted only when the
  identifier is in the table **and** the name names the battery or the
  charging system. SDD's own voltage bands are recorded beside it as
  constants, named as SDD's, and no code applies them to a verdict.
- *The ingest.* `PlatformAdapter::add_battery` writes one parameter
  definition per module and battery parameter, scoped to the module, the
  programme and the marker. The exporter builds `BatteryFormatting` from the
  DID formatting document in its first pass and hands it over in the second,
  so 18 identifiers carry SDD's own encoding and unit and the rest
  carry the platform's own name and no encoding. Real source:
  1,486 battery records, 106 module-identifier pairs over
  17 programmes; the library re-exported to 355,112
  records (from 353,626), nothing rejected; the owner's copy re-stamped
  (code D9C9-6B50, valid to 2026-10-12).
- *The resolver.* Nothing new: the platform records are catalogue parameters,
  so the readable-identifier gate of `ADR-0012` admits a battery read exactly
  as it admits a passport read. `f10_uds_execution`'s list of what a module
  exposes now names the battery identifiers, which is the gate saying so.
- *The read.* `BatteryService`, operation `BATTERY_STATE`, class `READ_ONLY`:
  one request per module and identifier, every parameter of that identifier
  decoded from the one answer, `asked` and `answered` both counting reads
  (the mileage survey's lesson), every read leaving the record a single read
  leaves. The session bundle carries `battery_reads`; the intake reads them
  under `battery_reads[i].reads[j]` and records no value of the battery.
- *The card and the panel.* The card sits in the session column between
  the steps and the vehicle and carries only what a glance needs: a level
  bar with the state of charge, three tiles for voltage, current and
  temperature, the time the reading was taken, and the button that starts a
  run. Everything else — every group the rule names, SDD's own name for each
  row in the user's language where the dictionary has it, a switch for the
  rows that answered nothing, the modules the run could not plan — is the
  battery panel in the working area, beside the passport and the
  configuration. A refusal says why rather than hiding the button.
- *The bench.* Believable values for the identifiers the data can decode —
  78 % charged, 21 °C, 1.5 A into the battery, 10.9 V of cold cranking, 1287
  days in service, two monitor resets, 23 mA parked — every row marked
  `SYNTHETIC`, and the end-to-end run fails if the report ever says *good*,
  *bad*, *poor*, *healthy*, *replace* or *failing*.
- *Tests:* the rule's four, two platform goldens, the readable-identifier
  list, the shell's end-to-end bench run, the intake's own, and 7 interface
  tests (95 in all).

**Departures from the text above.** Decision 1 said the rule would be
curated from the identifiers measured on 2026-09-12; it is, minus the
quiescent relay box's four event counters being called `Drain` rather than a
group of their own, which would have been a group of one on most cars. And
decision 5 originally put the groups on the card itself; the owner saw the
first drawing and moved them to a panel, which is what is built — the card
is the indicator, the panel is the reading. Where that panel finally lives,
and whether it becomes a dashboard of its own, is his to settle in a later
pass.

See `CURRENT_STATE.md` for the state and `F9_SDD_KNOWLEDGE_INGESTION.md` for
the ingest slice's real-source numbers.

## Amendment, 2026-09-19: a low battery is a precondition of the session

- Status: **Accepted, 2026-09-19** — drafted on the owner's question of the
  same morning ("чи повинні ми виводити попередження при напрузі
  акумулятора менше порогової і пропозицію підʼєднати зовнішній блок
  живлення, так як робе це СДД") and accepted on his word the same hour
  («ок, потім за потреби поправимо візуальні рішення»): the mechanism is
  decided here, the look is his to adjust once seen.

### What changes

Decision 5 stands: the card judges nothing, the battery is never called
good or bad, nothing is coloured red for its health. What this amendment
adds is a sentence about the **session**, not about the battery — the same
kind of sentence as *the car stands still, the ignition is on and the engine
is off* in the consent of `ADR-0036`. A reading of the rail is also a fact
about whether the next read or write is safe to make, and SDD says so
before it starts a session; this product says it in the same place and for
the same reason.

1. **The line.** When the voltage the battery card holds — the headline
   `VOLTAGE` reading of the last `BATTERY_STATE` run, the one the card
   already shows and dates — lies in SDD's own *low* band, one line stands
   under the card: the voltage, the band named as SDD's, how old the reading
   is, and the request to connect an external supply before reading modules.
   In the reader's own interface language; the number and the band are
   SDD's.

2. **The number is SDD's, and it is already here.** The band is the one
   this record measured on 2026-09-12 from SDD's `BatteryMonitor.ini` and
   `knowledge::battery` has carried as a constant since:
   `SDD_VOLTAGE_LOW_MAX_MV`, 11.6 V. This product still invents no
   threshold. (On 2026-09-18 the assistant proposed the threshold as "ours,
   marked ours", not having reread this record; the record already held the
   right source, and the proposal is corrected here.) Only the low band is
   used: the medium and high bands and their overlaps are SDD's own and are
   not drawn as a scale.

3. **The same line stands in every stage-2 confirmation** while the
   condition holds. `ADR-0036`, decision 3, already puts the preconditions
   of an operation in front of the person before anything is sent, in SDD's
   own words where the data has them; this one joins them, sourced from
   SDD's configuration rather than from the module's data, because a clear
   or a routine on a sagging rail can leave a module halfway.

4. **Nothing is blocked.** The person decides, as SDD lets them decide.
   Failing closed is for procedures this product does not know
   (`DEVELOPMENT_PRINCIPLES.md`), not for a car whose battery is low. The
   line goes when a fresh reading is above the band, and says nothing while
   there is no reading at all: no reading is not a low reading.

5. **On the bench** the line appears when the synthetic reading is below the
   band, marked synthetic like everything else the bench answers, so the
   mechanism is run in without a car. The bench's voltage is chosen by its
   scenario, so both states are shown on every commit.

6. **The record needs nothing new.** `battery_reads` already carries the
   reading and the time it was taken, so a report already says at what
   voltage a session's reads were made; the line is the interface's and is
   not written anywhere.

### What this does not decide

- Joining the legislated `0x42` control-module voltage, or the live read's
  battery voltage, to this line: those keep their own rule — the person's
  limits of `ADR-0022` and F15 — and the question of one voltage for the
  whole screen stays where the record above left it.
- Any automatic re-read of the battery to keep the line current: a
  reading's age is shown, and refreshing it is the person's click, as
  before.
- Anything SDD does beyond the warning — it does not stop a session either.

### To build, on acceptance

The interface: one line under the battery card, and the same line in the
clear's confirmation (`ModuleDetails`), both from the card's own reading
and the constant the knowledge crate exports; three languages. The bench:
a scenario whose voltage is in the low band. Tests: the line with a low
reading, no line with a healthy one, no line with no reading, and the
confirmation carrying it. `SAFETY_BOUNDARIES.md` gains nothing: no
operation is added. `CURRENT_STATE.md` records it as built.

### Built, 2026-09-19 — `IMPLEMENTED / FIXTURE_TESTED`

Built the hour it was accepted; nothing has met a car.

- *The number.* `BatteryReadSnapshot` carries `sdd_low_voltage_max_mv`,
  filled by the shell from `SDD_VOLTAGE_LOW_MAX_MV` through
  `diagnostic-session`, which re-exports it: the interface holds no
  number of its own, and a snapshot without one says nothing.
- *The line.* `batteryPreconditionText` beside the rest of the battery's
  formatting; `BatteryPrecondition` under the header; the same text handed
  to the module panel and shown in the clear's confirmation beside the
  other preconditions. Amber, one sentence, three languages; a bench
  reading says *synthetic* on the end. Nothing is disabled by it.
- *The bench.* Every scenario ending in nine answers 11.2 V for `0x402A`,
  inside SDD's low band; every other scenario keeps 12.6 V. Scenario 0
  stays the vehicle in good order.
- *Tests.* The line with a low reading, none with a healthy one, none with
  no reading, none with no band; the synthetic mark; the clear's
  confirmation carrying it with the confirming control still enabled; the
  bench's two voltages (4 interface tests, 1 bench test).

## Amendment, 2026-09-19, later: the voltage comes from whatever read it

- Status: **Accepted, 2026-09-19** — on the owner's word («роби»), after the
  morning's amendment met his own car.

### What the car said

The precondition line above stands on the battery read's headline
voltage, `0x402A`. Asked on 2026-09-19, the owner's library declares
thirteen battery parameters for the X250's `RSJB` and `0x402A` is not
among them: the car's battery monitor, as SDD documents it, does not
report the rail's voltage. So on the owner's own car the card never showed
a voltage and the line could never appear, on the bench or on a road. The
synthetic fixture had the same gap, which is why the bench never noticed;
it now declares the voltage, and the bench's two voltages, 12.60 V and
11.20 V on a scenario ending in nine, are read through the whole stack in
a test.

What the X250 does answer with a voltage: the legislated OBD read, PID
`0x42` *control module voltage* (`ADR-0022` §7), and the live read of a
module's supply voltage, which the owner read himself the day before —
`PCM 0xDD02`, 13.50 V.

### What changes

1. **The voltage is the freshest of three readings**, each with the time
   it was taken: the battery read's headline voltage; the legislated
   `0x42`; and a live-read value in volts whose name is the battery's or a
   control module's voltage, the same words the live limits' starter
   suggestion recognises (F15). A reading without a known time is older
   than any reading with one. The adapter itself measures nothing: the
   Mongoose stack has no such read, and this record does not invent one.

2. **The precondition line stands on that reading**, in SDD's low band as
   before, and says which read it came from and how old it is.

3. **The battery key on the plate shows that reading** once there is one —
   "13.50 V" beside the cell — the owner's own words the same morning: «і
   виводити напругу на значок акумулятора після зчитування». Which read it
   came from is on the key's tooltip and in the line; the key itself keeps
   to the number. It judges nothing, as decision 5 says.

4. **Nothing else moves.** The card's state of charge still comes from the
   battery read alone; the live tiles keep the person's limits; the report
   already carries every one of the three reads with its time, so the
   line writes nothing.

### What this does not decide

- A voltage measured by the adapter at the connector: not available in
  this stack today. If the firmware is found to offer one, it joins as a
  fourth source, first in freshness because it is always current.
- Any automatic re-read to keep the line current: the person's click, as
  before, for all three.

### Built, 2026-09-19, the same day — `IMPLEMENTED / FIXTURE_TESTED`

- *The three sources.* `voltage.ts` chooses the freshest of the battery
  read's headline, the legislated `0x42` and a live supply voltage, each
  with its time; the OBD and live controllers keep the time of their last
  answer; the line and the key stand on the choice. Unit-tested on the
  three, on the tie and on the reading without a time.
- *The bench's rail follows the scenario on every read, not on the battery
  read alone.* On a scenario ending in nine PID `0x42` answers 11.2 V and a
  live read of a module's supply answers 11.2 V, as `0x402A` does: one rail,
  one number, so the line appears on the bench for a car whose battery
  monitor declares no voltage of its own — the owner's X250 among them,
  which is what the amendment was written for. A sensor channel keeps its
  band; every other PID its value. Tested in the bench crate on the three
  reads, and through the whole stack in the shell: `0x402A` and `0x42` on
  scenario 9 and on the default, 11.2 against 12.6 and 14.0 V.
- *Decision 3, amended at the screen the same afternoon* («перенести в
  батарейку, жирним шрифтом і при наведені мишкою на батарейку міняти на
  прочитать»): the voltage is carried on the cell, in bold, not beside it,
  and gives way to the cell's word — *Read*, or *Stop* while a read runs —
  while the pointer is over the cell or the focus is on it, so the key
  still says what it does. A cell that cannot read keeps the voltage.
  Where the reading came from stays on the cell's tooltip.
