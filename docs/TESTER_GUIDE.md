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
  Nothing is switched off; Windows asks once, for this file. Every later
  build is a new file, so it asks again. If there is no "More info" button:
  right-click the file, Properties, tick "Unblock" at the bottom, Apply,
  and run it again.
- **Windows 11 with Smart App Control on:** the application is blocked with
  no "Run anyway", and Microsoft provides no per-app exception. The
  uninstaller is blocked as well, so the application has to be removed by
  hand. Smart App Control is switched off in Windows Security under App &
  browser control, Smart App Control settings, and since the April 2026
  Windows update it can be switched back on in the same place, without
  reinstalling; before that it was a one-way decision. If the installer
  says the application is already installed and cannot remove it, or if
  you want it gone, write to us: it is one folder, two shortcuts and two
  registry entries, and we will send the exact lines. Your call: turn it
  off while you work with the application and put it back afterwards, or
  use another PC. We are not asking you to. The real answer is a signed
  installer, and that is planned.

Nothing else is asked of you. Do not disable Defender, do not add
exclusions.

## Installing on a Mac

Open the disk image and drag ProwlOne into Applications. The application
is **not signed or notarised** with Apple, so the first launch is refused
with "Apple could not verify … is free of malware". What to do:

- **macOS 14 and earlier:** right-click the application, choose "Open",
  then "Open" again in the dialog. Once.
- **macOS 15:** try to open it once, then open System Settings → Privacy &
  Security, scroll to the message about ProwlOne and click "Open
  Anyway". Once.
- **If the message says the application was not downloaded from the App
  Store**, the signature is not the point: Privacy & Security is set to allow
  App Store applications only, and macOS then offers no "Open Anyway" at all.
  Either set "Allow applications downloaded from" to "App Store and identified
  developers" and repeat the step above, or use the commands below, which work
  whatever that setting says.
- If macOS only says "cannot be opened" with an OK button and offers
  nothing else, the bundle's seal did not survive the download. Two
  commands in Terminal repair it on your Mac — the first removes the
  download quarantine mark, the second seals the application with an
  ad-hoc signature (no Apple account involved) — then open it again:

```bash
xattr -cr "/Applications/ProwlOne.app"
```

```bash
codesign --force --deep --sign - "/Applications/ProwlOne.app"
```

**If the application on a Mac says the data does not match the stamp of your
copy**, and you have just carried that copy on a stick or over a share, the
data is almost certainly fine. macOS writes a sidecar beside every file
(`._platform.json` next to `platform.json`), invisible in Finder and ending in
`.json` all the same, and the check counts them as files that should not be
there. In Terminal, in the library folder:

```bash
dot_clean .
```

Then choose the folder in the application again. If `dot_clean` does not do
it:

```bash
find . -name '._*' -delete
```

Builds after 0.9.6 ignore such files; this note is for 0.9.6 and earlier.

The MongoosePro JLR appears on a Mac as a USB modem port with no driver
to install; a clone with a different chip may need its maker's driver.
Nobody has run the Mac build yet: if you are the first, say so in your
notes, and tell us the macOS version and whether the adapter was found.

## First launch

1. The language switch is in the top right: English, Russian, Ukrainian.
   SDD's own text — module names, fault-type wording — exists in English and
   Russian; Ukrainian gets the interface in Ukrainian and the car's data in
   English.
2. The library archive is your personal copy: it is issued in your name,
   signed, and valid for one month from the date inside it. The application loads no other library — an
   unstamped, altered or expired folder is refused with a plain message —
   and asks for a renewal a week before the date. A new copy takes us a
   minute; just ask. We hand it over on one request: keep it to yourself,
   do not copy or pass it on. Every report you send names your copy.
3. **Unpack the zip into an ordinary folder first** — right-click the
   archive, "Extract All…". The folder chooser in the application shows
   folders only; an archive will not appear in it.

   Inside you will find eight `.json.gz` files and `issued_to.json`. **Those
   files stay packed** — the application reads them compressed, which is why
   the copy is about ten megabytes and not three hundred. Unpacking one by
   hand makes the copy stop matching its stamp; ask for a new one.

   Then, in **Data library**, click **Choose folder…**, pick that unpacked
   folder, and press **Load library**. Loading takes a few seconds; the panel
   then shows how many manifests and records were read.
4. Plug the adapter into USB. In the adapter panel discover and connect; the
   panel shows the COM port and the device's identity.
5. Still without a car: run **Capture** on the high-speed bus once. It
   should finish in a few seconds with zero frames and no error — that
   proves the adapter takes our commands. The first operation after
   plugging in takes a moment longer: the adapter starts up in a small
   loader program and is switched to its working firmware before the bus
   is opened. Nothing is written to the adapter; unplugging it returns it
   to the loader.

## Try the bench first

Before the car — or without one — click **Connect the bench (virtual
vehicle)** in the adapter panel. No adapter is needed. A virtual vehicle,
built from the library for the programme and model year you choose,
answers the same commands the real path sends: survey it, check all
modules, read a module, listen to a bus, look at the report. The whole
window turns orange-sand with a BENCH band top and bottom while the bench is
connected, every value is marked synthetic, and nothing from a bench
session can be saved: the report is shown on screen only. Bench data
proves the software, never a car; do not send us bench screenshots as
Beside the button is a **Scenario** number, which decides what the bench
reports. `0` is a vehicle in good order: every module answers and none has a
fault code. Any other number gives the modules codes drawn from the library's
own wording for them, so every code shown has a description. The same number
always paints the same picture, so «scenario 7» in an e-mail is enough for us
to see exactly what you saw. The report carries it too, as `bench_scenario`,
so there is nothing extra to send.

Click **Disconnect** to leave the bench; if you recorded anything
on it, the application asks for a new session before a real adapter is
connected.

## Mileage from every module

In the "Module network" section, beside the button that checks every module,
there is **Read the mileage**. A car does not keep its mileage in one place:
the cluster, the engine, the gearbox, the brakes, the airbag module, the
parking brake, the tyre-pressure module, the doors, the gateway — each counts
for its own reasons. The button asks every module the data names a distance
for, once each, and puts the answers in a table.

The **Difference** column is how far that reading is from the highest on the
car. The highest, not the cluster's: the cluster is the first thing rewritten.
It is the subtraction of two numbers the application has just read, and
nothing more.

A second table, **Recorded at an event**, holds the odometer as it stood when
a module wrote something down — a gearbox stall, a failed gear selection, a
door opened. A reading above the highest running total is an event the car has
not driven to.

**The application draws no conclusion.** Modules disagree for honest reasons:
a replaced cluster, a gearbox or an airbag module fitted after a repair each
carry their own count. What a difference means is for a person to decide. And
a mileage rewritten carefully in every module will not show here; a careless
one will.

Nothing is written — not here, not anywhere.

## What the data says about a code

Under every fault code there is a fold: **What the data says about this code**.
It is not our text. It is what SDD writes about that code — possible causes,
actions required, sometimes the conditions under which the module sets it and
the manufacturer's own limits ("voltage below 7.5 Volts for at least 200ms").
In English, because that is the only language the data holds it in.

The data writes different text for different model years, so the screen is
chosen for the car: by module, programme, model year and fault type. **If the
model year is not given and the data offers more than one, nothing is shown**
and the application says why — the neighbouring year's causes would send you
to the wrong part. Set the model year in the Vehicle section.

Two thirds of codes have text. The rest carry only a placeholder in the data,
and then there is no fold.

## Live reading

The "Module network" section has a **Live reading** panel. It does what the
Read button does, but round after round: tick the parameters you want — up
to sixteen, from any modules the data describes — press Start, and the
application asks for each in turn until you stop it. The table shows the
current value with the catalogue's unit, the smallest and largest of the
run, how many readings there have been, and beside it **how long a round
actually takes**. That is a measurement, not a promise: if a round takes two
seconds, it says two seconds.

The limits are deliberate: no faster than ten requests a second, no longer
than ten minutes a run, one request at a time, and Stop gets through between
two requests. An entry that is silent or refused three rounds running is
dropped from the set, with its reason. All of it is read-only.

**The vehicle must be standing**, engine running or not. This is a
diagnostic instrument for a stationary car, not for driving.

The whole run joins the session report: every value with the milliseconds
since the run began, the raw bytes, and the library's own decode.

## Standard OBD-II

The "Listening and standard OBD-II" section has a **Standard OBD-II** panel:
what every OBD-II car answers at the same addresses, JLR or not, by SAE
J1979 rather than by the library — current data (engine speed, vehicle
speed, temperatures, fuel trims, voltage…), the three kinds of fault code,
the freeze frame, monitors, VIN, calibration, ECU name. Read-only: there is
no "clear codes" button here and there will not be.

**Read everything supported** asks the module which parameters it knows and
reads them six to a request; the table fills as the answers come. The
responder is one of the eight addresses ISO 15765-4 reserves. The standard
does not say which module sits at which: 0x7E0 is the engine controller by a
convention kept almost everywhere, 0x7E1 is usually the transmission, and the
other six are free and silent on most cars. Something answers at them on
hybrids, on diesels with AdBlue, sometimes on engines with two controllers.
Reading an empty address is a module saying nothing, not a fault in the
application. A parameter
whose layout we do not carry is shown as raw bytes and labelled so — we do
not invent values. All of it works on the bench too, marked synthetic.

**Freeze frame** reads the whole frame: first the code that froze it, then
every parameter the frame carries. **Vehicle information** reads all of
mode 09 that is a read: VIN, calibrations and their verification numbers,
the in-use monitor counters, the ECU's name and serial number. Parameter
names are in the interface language where we carry a translation, else
the standard's English.

## In the car

Ignition on, engine off (position II). Plug the adapter into the diagnostic
socket. Then, in order:

1. **Capture both pairs.** The high-speed bus first (pins 6/14), then the
   medium-speed one (3/11), a few seconds each; save both files and send
   them. This is listen-only and cannot disturb anything, and it is the
   most valuable thing you can send us from a car we have not seen. We
   need the 3/11 pair in its own right: traffic on it confirms that this
   car brings its medium-speed bus to the connector. Silence is an answer
   too, so save that as well.
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

The build you run is written at the foot of the window, for example `0.9.3 · a1b2c3d`; quote it in every message, and it is also inside the report file.


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
