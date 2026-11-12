# ProwlOne

**Multi-platform vehicle diagnostics** for Jaguar and Land Rover of the SDD
era, roughly 2005–2016 — on an ordinary Windows PC or a Mac, with a
MongoosePro JLR cable, without SDD's Windows 7, virtual machine and install
misery.

It is not a generic OBD-II scanner, and not an SDD clone or front end: it is
an independent application that reads every module it can reach on every
bus, with the manufacturer's own identifiers and fault-code wording.

## What it does

- Reads fault codes with their help text, part numbers and software levels,
  live values, the battery and the car's configuration, from every module
  it can reach.
- Reads the mileage from every module that keeps one and lays the figures
  side by side, with the difference from the highest. It draws no
  conclusions — modules can differ for honest reasons — but it is a quick
  cross-check before buying a used car.
- Listens to a bus without sending anything, and saves the traffic.
- Saves the whole session as one report, readable on screen, printable or
  saved as PDF.
- In a **service mode** that you switch on yourself — off on every start,
  and asking again before each operation — it clears fault codes, on one
  module or on every module that reported some, and runs a module's
  on-demand self test. You use these operations at your own risk.
- Speaks English, Ukrainian and Russian.

Flashing modules and programming keys are not planned — that is a
deliberate choice.

## What you need

- A MongoosePro JLR cable, original or a faithful clone. No extra drivers
  or J2534 software: Windows and macOS see the original cable with their
  own built-in drivers; some clones need their maker's driver. On a PC that
  also runs SDD the cable is usually held by SDD's own driver: builds from
  1.2.5 read it there too, earlier builds need it handed to Windows's
  serial driver, and the
  [tester guide](docs/TESTER_GUIDE.md#if-something-goes-wrong) shows how.
  ELM327-type dongles cannot reach these modules and are not supported.
- Windows 10 or 11, or a Mac with Apple silicon or Intel. Windows 7 and 8
  are not supported: the application's window is Microsoft's WebView2, which
  exists for Windows 10 and 11 only. The full data library takes about 4 GB
  while it loads, so 8 GB is comfortable; a 32-bit Windows 10 PC with 4 GB
  works with a library cut to one or a few models.
- A Jaguar or Land Rover from about 2005 to 2016.
- The data library. It is not in this repository and not on the Releases
  page: it is derived from SDD's own data and is issued to each tester
  personally, valid for a month and renewed on request. Without it the
  application connects to the cable and listens to a bus, but cannot survey
  the car or word its fault codes.

## Download

Installers are on the [Releases](https://github.com/Oleg-mk/jlr-scanner/releases)
page — one for 64-bit Windows, one for 32-bit Windows, one for any Mac. They are not signed yet, so Windows
and macOS warn before the first start; the [tester guide](docs/TESTER_GUIDE.md)
says what you will see and what to do. The application installs in seconds
to a minute; the data library adds a couple of minutes to download, unpack
and load.

## Status

The application works end to end on a bench simulator. What it needs now is
results from real cars — as many models, years and engines as possible —
and nothing is confirmed on a car yet. Every report helps: the
[tester guide](docs/TESTER_GUIDE.md#what-to-send) says what to send.

## Contributing

Issues and pull requests are welcome, in English or Ukrainian. Please do not
send SDD files, nor the capture of a vehicle that is not yours without its
owner's consent. How the application
is built, and why, is in [docs/](docs/), starting with
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

Copyright © 2026 Oleg-mk. Licensed under the GNU Affero General Public
License, version 3 ([LICENSE](LICENSE)): use, study, change and share
freely; a changed version that others use, including over a network, must
be shared under the same terms with its source. The name "ProwlOne" and the
project's marks are not covered by the license. Jaguar, Land Rover, Range
Rover and SDD are trademarks of Jaguar Land Rover Limited; this project is
not affiliated with or endorsed by it.

Written by Oleg-mk with Claude (Anthropic) as the coding assistant.
