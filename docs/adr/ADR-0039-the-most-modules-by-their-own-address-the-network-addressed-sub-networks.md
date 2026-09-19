# ADR-0039: The MOST modules by their own address — the network-addressed sub-networks of the L319, L320 and L322

- Status: **Proposed, 2026-09-19** — the item the owner put next in line
  the same night («на черзі борг із недосяжної шини мост»), in the order he
  approved the evening before: the self tests first, the `NETWORK_ADDRESSED`
  route as its own ADR beside them, the rest when they have evidence.
  Accepted on his word.
- Scope: the knowledge base (one new claim from SDD's own data), one
  built-in research manifest, the survey's wording and the interface's
  note. **Read-only.** No new request, no session change, no routine, no
  transmit API: a module on such a sub-network is read exactly as a module
  on the medium-speed bus is read today.

## Context

Every SDD-era car has modules this product cannot reach: the ones on a
sub-network behind a gateway module. The X250 of 2010 has eleven such
modules, ten of them on the MOST ring; the L319 and L320 of 2010 have nine
each, the L322 of 2010 ten. `F10` recorded on 2026-09-08 that the door's
key is not in our hands: SDD's platform documents name the gateway and the
mechanism but not the routine that opens it.

Read again on 2026-09-19, the 45 platform documents say three different
things under `<gateway>`, and only one of them is a locked door:

- **`ROUTINE_CONTROL`**, 28 documents: the gateway is told to open the
  sub-network by a routine SDD's data does not name. The X250, the X150,
  the L359, the L405, the L494, the L538, the L550, the X152, the X351 —
  and the L319 and L320 from MY12. This stays as `F10` left it: stage 2,
  and a listen-only capture of a genuine SDD session is the evidence path.
- **`NGI_NETWORK_ADDRESSED`**, 7 documents of 2016 and later: not read yet,
  and nothing is claimed about it here.
- **`NETWORK_ADDRESSED`**, 8 documents, all with `ACM_SYSTEM_A` as the
  gateway of `SUB_MOST`, whose main net is `CAN_MS`: `PLATFORM_L319.xml`
  (T5, MY05), `L319_201000` (MY10), `PLATFORM_L320.xml` (T5, MY06),
  `L320_201000` (MY10), `PLATFORM_L322.xml`, `L322_200600`, `L322_200700`
  and `L322_201000` (MY10). No routine is named because none is declared:
  the access method itself says the module is reached by its address on
  the network.

Those eight documents are two kinds, and the difference is the whole
decision.

**The MY10 documents address the MOST modules like any other module.** In
`L319_201000`, `L320_201000` and `L322_201000` the sub-network is declared
with `<can><rate>125</rate><identifier>11</identifier></can>` and
`<addressing><can_iso15765><normal/>`, the same parameters as `CAN_MS`
itself, and its modules carry ordinary 11-bit diagnostic identifiers in
the main bus's own identifier space:

| MY10 | modules on `SUB_MOST` (request identifier) |
|---|---|
| L319, L320 | `AAM` 0x7A4, `APIM` 0x7D0, `DABM` 0x7D6, `DACMC` 0x7D5, `FEM` 0x784, `REM` 0x771, `SRM` 0x782, `TEL` 0x781, `TVM` 0x707 (two rows) |
| L322 | the same and `CDP` 0x770 |

Twenty-eight module rows. Nothing in the data distinguishes the request
to `AAM` at 0x7A4 from the request to a module on `CAN_MS`, except the
statement that the answer comes back through the `ACM`.

**The older documents address them with a 29-bit layout the data does
not compose.** `PLATFORM_L319.xml`, `PLATFORM_L320.xml` and the three
L322 documents of MY04.5–07 are the 29-bit `normal_fixed` programmes of
`ADR-0017`. Their `SUB_MOST` is 29-bit at 125 kbit/s under
`<addressing><can_iso15765><enhanced>` with, verbatim:

```
can_id_prefix   type="all"       0x6F
network_mask    type="main_net"  3
network_mask    type="sub_net"   3
network_address type="main_net"  0x600
network_address type="sub_net"   0x500
```

and the modules carry one-byte physical addresses (`AAM` 0x86, `DACMC`
0x8D, `FEM` 0x61, `REM` 0x8B, `SRM` 0x81, `TEL` 0x90, `TMC` 0x67, `TVM`
0x87; eight rows in each of the five documents, forty in all). How a
29-bit identifier is composed from a prefix, two masks, two network
addresses and a node address is stated nowhere in the data this product
holds, and is not a layout of ISO 15765-2 this project can cite. Under
the working discipline that is `UNKNOWN`, and it stays `UNKNOWN`.

## Decision

1. **The library records what SDD says about every sub-network's
   gateway.** The platform adapter reads each `<network>` that declares a
   `<gateway>` and records, on every module of that sub-network — per
   module, as the bus facts SDD states once per network are recorded
   today, so the survey reads it where it reads the module — one claim
   `sdd_gateway` in the escaped `key=value;…` text form of `ADR-0028`:
   `main_net`, `sub_net`, `gateway`, `access`, and — where the addressing
   is `enhanced` — `prefix`, `main_net_mask`, `sub_net_mask`,
   `main_net_address`, `sub_net_address`, verbatim. The same evidence and
   state as the module's other claims from the document,
   `REAL_SOURCE_INGESTED` like the rest of the platform slice. This is
   data, not reach: it lets the interface say *why* a sub-network waits —
   routine, network address, NGI, or an addressing layout we cannot
   compose — from the record instead of from a rule on the bus's name,
   and it keeps the five numbers of the older layout where a future
   capture can meet them.

2. **A network-addressed sub-network with normal 11-bit addressing is
   reached on its main bus's route, as a hypothesis.** A built-in research
   manifest, `mongoose_jlr_network_addressed_route_hypotheses.json`, binds
   the bus `SUB_MOST` to the physical route J1962 pins 3/11 at 125 kbit/s
   and the backend route `ms-can` — the binding `CAN_MS` already has —
   for exactly the three documents that declare it so: `L319` MY10–11,
   `L320` MY10–11 and `L322` MY10–12, by programme and model-year range,
   because the same programmes' MY12 documents switch the sub-network to
   `ROUTINE_CONTROL`, and because `SUB_MOST` on every other programme is
   a different thing. The mechanism is `ADR-0015`'s, unchanged: the record
   is `UnverifiedResearch`, the survey shows the module as a **hypothesis**
   with the route's `UNVERIFIED` validation, the first answered read on
   such a car confirms it per vehicle through the F13 intake (`ADR-0016`),
   and its absence refutes it and is recorded with the same care.

   Why a hypothesis and not a fact: SDD's data states the parameters and
   the mechanism's name; that the `ACM` forwards a normally addressed
   request without being asked is this project's reading of
   `NETWORK_ADDRESSED` against `ROUTINE_CONTROL`, and a reading is
   research until a car answers. Why not a guess: the sub-network declares
   the main bus's rate, width and addressing, its modules sit in the main
   bus's identifier space, and no routine is declared where the same
   documents declare one for every gateway that needs it.

3. **The older layout stays out, and says so.** No identifier is derived
   for a module on an `enhanced` sub-network. The survey keeps it
   unreachable with the reason stated from the record of decision 1: the
   addressing layout is not in the data. The way in is the one `F10`
   already names for the routine — a listen-only capture on pins 3/11
   while a genuine SDD session reads one MOST module of such a car — which
   would show the identifiers as sent; or a document that names the
   composition. Either turns decision 3 into a derivation the size of
   `ADR-0017`'s, in its own amendment.

4. **The read path is untouched.** A module bound by decision 2 is read
   through `execute_prepared_uds_read` on `ms-can` with 11-bit normal
   addressing, the path the medium-speed modules already use; the gateway
   is not addressed, not opened and not spoken to. The survey's reason for
   the hypothesis names the gateway module and this ADR; the network
   map's lane note for a gatewayed sub-network comes from the recorded
   access method — *reached through `ACM` by network address, unverified*,
   *SDD opens it with a routine command, stage 2*, *NGI, not read yet*,
   *addressed with a layout the data does not compose* — and falls back to
   today's wording on a library issued before decision 1.

5. **The bench mounts them.** The bench answers on the route the survey
   gives a module, so a synthetic sub-network declared the MY10 way in the
   synthetic platform fixture, bound by a test-side manifest of the same
   form, lets the whole-stack test read a module behind a bench gateway
   end to end. That earns `FIXTURE_TESTED` for the resolution and the
   route; it proves nothing about an `ACM`.

## Consequences

- On an L319 or L320 of MY10–11 or an L322 of MY10–12 with the current
  library, nine or ten more modules — the audio amplifier, the
  infotainment and telephone modules, the DAB and the TV, the front and
  rear entertainment — appear in the survey as reachable on a hypothesis,
  and the network check may try them; nothing changes for any other car.
  The manifest is built into the application, so no library is re-issued
  for the route; the gateway claims of decision 1 reach an issued library
  with its next export.
- The knowledge base gains one claim name and no new value kind; the
  resolver, the executor and the guard are unchanged. The exporter's
  record count grows by one per module on a gatewayed sub-network.
- The interface stops inferring "behind a gateway" from a bus's name
  where the data can say it.
- X250's `SUB_MOST` and `SUB_CAN1` are exactly where they were: the
  routine's evidence path is a capture, and the owner's decision on
  searching SDD's program binaries for it is not taken here.

## Not in this decision

Any routine to a gateway; the `enhanced` layout; `NGI`; the sub-networks
of the 2014-and-later cars; anything that sends a frame this product does
not send today. No operation here has met a car.

## Validation

`IMPLEMENTED` and `FIXTURE_TESTED` on the synthetic sub-network when built;
the three real bindings are `Unverified` by construction and become
`CAPTURE_VALIDATED` per vehicle only through a tester's report.

## Built, 2026-09-19, the same night

- *Decision 1.* The platform adapter records `sdd_gateway` on every
  module of a gatewayed sub-network, with the `enhanced` numbers where
  the bus declares them; the survey carries it to the interface as the
  module's `gateway`. The synthetic platform gained `SUB_SYNTH` behind
  `SYNTHMOD_SYSTEM_A` by network address and one module on it, `TVMOD`,
  and the platform golden reads the claim back verbatim.
- *Decision 2.* The built-in manifest binds `SUB_MOST` to pins 3/11 at
  125 kbit/s and `ms-can` for L319 MY10–11, L320 MY10–11 and L322
  MY10–12, six records, `unverified`; the built-in set is seven
  manifests now. The survey words such a hypothesis with the gateway's
  name and this decision, and words every other gatewayed module with
  what the data says about its gateway: the routine SDD would send, the
  layout the data does not compose, NGI not read yet.
- *Decision 4.* The network map's lane note and the module's reason come
  from the recorded gateway, and fall back to the bus's name on a library
  issued before this. The browser demo's MOST and NGI lanes carry their
  gateways.
- *Decision 5.* A test-side manifest of the same form binds `SUB_SYNTH`
  to `hs-can` for `SYNTHA`; the session tests survey `TVMOD` as a
  hypothesis with the gateway named, and the whole-stack bench test
  reads its fault codes end to end over the main bus's route, the
  route validation `UNVERIFIED` as the survey said. Nothing has met a
  car; the three real bindings wait for an L319, L320 or L322 of MY10.
