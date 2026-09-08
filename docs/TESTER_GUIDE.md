# Tester guide

This is the hand-out for a named tester. `TESTER_GUIDE.uk.md` is the same
text in Ukrainian, the one the community receives. Keep the two in step.

## What this is

An independent diagnostic application for Jaguar and Land Rover of the SDD
era, built to reach what SDD reaches — every module on every bus — without
SDD's Windows 7, virtual machines and install misery. It talks to the car
through a MongoosePro JLR adapter, the same class of adapter SDD uses.

At this stage the application only **reads**. It sends the diagnostic
requests SDD sends to read fault codes, identification and live values, and
it records what the car answers. It writes nothing to the car: no fault-code
clearing, no adaptations, no coding, no programming. Those come later, as
separate, explicitly confirmed steps. The capture function does not even
send: it listens to the bus for a few seconds and records the traffic.

One consequence you will see on the map: the modules on the MOST ring and
other sub-networks — audio, telematics, navigation — show as "not
reachable". SDD reaches them through a gateway module that it opens with
a routine command. A command is not a read, so this stage does not send
it. Everything needed for those modules is already known — the gateway,
the command, the addresses — and they come in stage 2, once the first
non-read operation has its own safety rules. The map says so under the
bus.

## What you need

- A Windows 10 or Windows 11 PC, or a Mac (Apple silicon or Intel).
- A MongoosePro JLR adapter, original or clone. If a clone does not appear
  in the application, say so: the USB identifiers from Device Manager
  (Hardware Ids, the `VID_` and `PID_` values) are what we need to add it.
- The installer, received from the maintainer.
- The data library, a folder received from the maintainer as an archive.
  It is derived from SDD's own diagnostic data and is not redistributed;
  keep it to yourself. Your copy is stamped with your name and a short
  code — the library panel shows it, and every report you send carries it.
- A car: Jaguar or Land Rover from 2005 to about 2016.

## Installing

Run the setup file. It is **not signed** with a code-signing certificate,
so Windows shows a prompt. What to expect:

- **Windows 10, and Windows 11 without Smart App Control:** SmartScreen
  shows "Windows protected your PC". Click "More info", then "Run anyway".
  Nothing is switched off; Windows asks once, for this file.
- **Windows 11 with Smart App Control on:** the file is blocked with no
  "Run anyway". Smart App Control can be turned off in Windows Security
  under App & browser control, but Windows does not let you turn it back on
  without reinstalling. If you would rather keep it, use another PC; do not
  turn it off for our sake.

Nothing else is asked of you. Do not disable Defender, do not add
exclusions.

## Installing on a Mac

Open the disk image and drag JLR Scanner into Applications. The application
is **not signed or notarised** with Apple, so the first launch is refused
with "Apple could not verify … is free of malware". What to do:

- **macOS 14 and earlier:** right-click the application, choose "Open",
  then "Open" again in the dialog. Once.
- **macOS 15:** try to open it once, then open System Settings → Privacy &
  Security, scroll to the message about JLR Scanner and click "Open
  Anyway". Once.
- If macOS only says "cannot be opened" with an OK button and offers
  nothing else, the bundle's seal did not survive the download. Two
  commands in Terminal repair it on your Mac — the first removes the
  download quarantine mark, the second seals the application with an
  ad-hoc signature (no Apple account involved) — then open it again:

```bash
xattr -cr "/Applications/JLR Scanner.app"
```

```bash
codesign --force --deep --sign - "/Applications/JLR Scanner.app"
```

The MongoosePro JLR appears on a Mac as a USB modem port with no driver
to install; a clone with a different chip may need its maker's driver.
Nobody has run the Mac build yet: if you are the first, say so in your
notes, and tell us the macOS version and whether the adapter was found.

## First launch

1. The language switch is in the top right: English, Russian, Ukrainian.
   SDD's own text — module names, fault-type wording — exists in English and
   Russian; Ukrainian gets the interface in Ukrainian and the car's data in
   English.
2. Unpack the library archive into a folder. The archive is your personal
   copy: it is issued in your name, signed, and valid for one month from
   the date inside it. The application loads no other library — an
   unstamped, altered or expired folder is refused with a plain message —
   and asks for a renewal a week before the date. A new copy takes us a
   minute; just ask. We hand it over on one request: keep it to yourself,
   do not copy or pass it on. Every report you send names your copy.
3. In **Data library** click **Choose folder…**, pick that folder, then
   **Load library**. Loading takes about ten seconds; the panel then shows
   how many manifests and records were read.
4. Plug the adapter into USB. In the adapter panel discover and connect; the
   panel shows the COM port and the device's identity.
5. Still without a car: run **Capture** on the high-speed bus once. It
   should finish in a few seconds with zero frames and no error — that
   proves the adapter takes our commands. The first operation after
   plugging in takes a moment longer: the adapter starts up in a small
   loader program and is switched to its working firmware before the bus
   is opened. Nothing is written to the adapter; unplugging it returns it
   to the loader.

## In the car

Ignition on, engine off (position II). Plug the adapter into the diagnostic
socket. Then, in order:

1. **Capture** the high-speed bus for the default few seconds and save the
   file. This is listen-only and cannot disturb anything; it is also the
   most valuable thing you can send us from a car we have not seen.
2. **VIN.** Enter the VIN; the application decodes it with SDD's tables and
   offers the matching programme and model year. Correct them if you know
   better.
3. **Survey.** The network map shows the modules SDD expects on this car,
   by bus, with each one's reachability and why. Click **Check all
   modules**: the application asks every reachable module to identify
   itself and marks who answered.
4. **Read** one module — the engine module is a good first choice — for its
   fault codes and identification.
5. **Save the session report** from the header. It is one file that holds
   everything above.
6. For the next car, or to start over, click **New session** in the header:
   the report, survey and reads are dropped after you confirm; the adapter
   and the library stay.

If a step fails, save the report anyway and go on to the next step. A car
that stays silent is evidence too.

## What to send

The build you run is written at the foot of the window, for example `0.9.1 · a1b2c3d`; quote it in every message, and it is also inside the report file.


The session report file, the capture file, and a few lines: the car
(model, year, engine), the adapter (original or clone, and the clone's USB
identifiers if it was not found), your Windows version, and what did or did
not work. Send it through the channel you received the build from.

The report contains the VIN, the adapter's serial number and what the
modules answered. It contains no personal data beyond that; say so if you
would like the VIN masked in what we publish.

## If something goes wrong

- **The adapter is not found.** Check Device Manager for a COM port when
  the adapter is plugged in. If there is none, the driver is missing; if
  there is one and the application still does not list it, send us the
  Hardware Ids.
- **The library fails to load.** The panel lists which files failed and
  why; send the message.
- **A capture or read fails with "device status" and some words.** The
  words are the adapter's own answer; send them exactly. If the message
  says the adapter's firmware did not answer, unplug the adapter, plug it
  back in and try once more.
- **Modules do not answer.** Expected on some cars at this stage: the
  addressing of many modules is derived from SDD's data and is marked
  `UNVERIFIED` until a car confirms it. Your report is what confirms it.
- **The application does not start.** Say which Windows version and what
  appeared, if anything.
