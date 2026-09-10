# The cadence of a read, 2026-09-10

What `ADR-0022` decision 3 asks for before any gauge is drawn: how fast a
read can be, measured, in its two halves. The **hardware half** is the link
to the adapter, with no vehicle attached and nothing transmitted on any
bus. The **software half** is the product's own read path with the bench
answering. Neither is a vehicle figure; the module's own answer time is the
one term only a car can show.

## The hardware half: the link to the adapter

`scripts/mongoose-probe/mongoose_link_cadence.py`, on the owner's Windows
desktop, MongoosePro JLR on `COM3` (`USB\VID_18E1&PID_0104`), 115 200 baud,
pyserial 3.5. The adapter was plugged in cold, so it answered from the
bootloader first; the script sent `cJumpToFirmware` and waited for the
firmware's board-info, as the application does (`ADR-0018`). Then:

1. **board-info round trips** — `0x0109` sent, the `0x8109` answer awaited,
   timed individually, one at a time for five seconds. Device-local: the
   USB/serial cost of one request and one answer.
2. **open, pins, close** — `cOpenChannel` on resource 5 listen-only at
   500 kbit/s, `cSetPins` 6/14, `cClose`, timed as one cycle, twenty times.
   Listen-only opens send no frame on the bus; this is the route handling a
   single read pays today, which opens and closes the route around itself.

Verbatim:

```
board-info status 0: bootloader
firmware answered 47 ms after the jump
board-info round trip: n=306  min 10.2  median 15.6  mean 16.4  p95 22.9  max 25.7 ms  -> 61.1 per second
open + pins + close (listen-only, resource 5): n=20  min 42.4  median 50.3  mean 50.0  p95 55.2  max 58.5 ms  -> 20.0 per second
```

## The software half: the read path with the bench answering

`cadence_on_the_bench` in `apps/scanner/src-tauri/src/bench_e2e.rs`, ignored
by default and run on purpose, in release, in the `rust:1.98-bookworm`
container the shell crate is tested in. It takes the very path a live read
would: the transaction prepared once from the library, executed over the
adapter framing through the bench transport, ISO-TP and UDS, the answer
decoded with the catalogue and marked — round after round over a set of up
to sixteen module-and-identifier pairs, for five seconds, then one
identifier alone for three.

With the owner's issued copy (`BE5E-E564`, 131,501 records) and his own car
described — X250, MY10, V8SC — the survey found 39 modules, 28 reachable,
and the bench mounted the 28. The set-builder walked the survey's modules in
order, so it also tried six the survey had already called unreachable, and
each was refused at preparation with the resolver's words, as it should be:

```
library: Loaded, 131501 records — Loaded 6510 manifests: 6515 sources, 131501 records.
vehicle: X250 MY10 — 39 modules surveyed, 28 reachable; bench: X250 MY10: 28 module(s) answer on the bench; every value is synthetic
  not in the set: AAM 0xD117 — … the route to AAM cannot be resolved from the loaded data …
  (likewise ACM 0x8334, APIM 0xDD01, DABM 0x8321, DACMC 0xD117, FCDIM 0x4108)
  ABS 0x203C  CCM 0xD703  DCSM 0xDD01  DDM 0x4162  DSM 0xDD01
  GSM 0xD000  HCM 0xDD01  HVAC 0x995A  ICM 0x4108  ICS 0x581D
full set: 10 entries — 779340 requests in 5.0 s, 779340 succeeded; 155866.8 requests/s, 0.01 ms per request, 77934 rounds of 0 ms
one identifier: 1 entry — 344986 requests in 3.0 s, 344986 succeeded; 114995.3 requests/s, 0.01 ms per request
```

With the SYNTHA fixtures (65 records, two identifiers on SYNTHMOD):

```
full set: 2 entries — 1253944 requests in 5.0 s, 1253944 succeeded; 250788.4 requests/s, 0.00 ms per request
one identifier: 1 entry — 759950 requests in 3.0 s, 759950 succeeded; 253316.3 requests/s, 0.00 ms per request
```

To repeat, with the issued copy mounted at `/library`:

```
docker run --rm -v "<repo>:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "<issued copy>:/library" rust:1.98-bookworm sh -c "apt-get update -qq; apt-get install -y -qq libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libudev-dev pkg-config; cd /w/apps/scanner/src-tauri; export CARGO_TARGET_DIR=/w/target/linux-shell; PROWLONE_LIBRARY=/library PROWLONE_PROGRAM=X250 PROWLONE_MODEL_YEAR=2010 PROWLONE_POWERTRAIN=V8SC PROWLONE_BREAKPOINT=MY10 cargo test --release cadence_on_the_bench -- --ignored --nocapture"
```

## What the two halves say together

The software costs **0.01 ms a read** with the real library; the link costs
**~16 ms a round trip**. The software is not a term in the cadence at all —
it sits four orders of magnitude below the hardware — so nothing about the
read path needs optimising for a live view, and nothing about it can help.

A read on a car is at least: one outbound command and its acknowledgement
over the link (~16 ms), the module's own time to answer (unknown until a
vehicle is met; tens of milliseconds is usual for a diagnostic request), and
the inbound frames back over the same link. With the route kept open for
the run, that is of the order of **30–70 ms a read, roughly 15–30 reads a
second**. With the route opened and closed around every read, as a single
read does today, add **~50 ms**: roughly **8–12 a second**.

Two things follow for the service `ADR-0022` describes. It opens the route
once for the run and closes it on stop, not per request — about a threefold
difference. And the ADR's floor of 100 ms between requests — ten a second —
binds before the hardware does; it is a courtesy to the bus, chosen, and can
be revisited once a car has been seen, not before.

## What it does not show

The bootloader-to-firmware figure here (47 ms) is not the 241 ms of
`F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`: that was the first board-info
poll at a 50 ms interval, this is the firmware's first answer measured
directly. Neither this nor anything above is a vehicle figure.
