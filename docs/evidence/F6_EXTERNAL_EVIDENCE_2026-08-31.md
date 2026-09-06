# F6 external evidence acquisition

Date: 2026-08-31

Gate result: **SATISFIED**.

This evidence set supports one read-only X250 diagnostic-environment golden. It
does not authorize a vehicle connection, CAN transmission, Mongoose channel
open, diagnostic execution, or any inference from response and request IDs.

## Target

- vehicle report: 2010 Jaguar XF Supercharged;
- VIN: `SAJWA0HE4AMR59890`;
- capability: OBD Service/Mode `09`, InfoType/PID `04`, Calibration ID;
- observed responder: `0x7E8`;
- observed calibration ID: `CX23-14C204-ZAD`;
- `0x7E9` / `9X23-7J105-CB` is retained only as report context and is not part
  of the resolved target.

## Source inventory and fingerprints

| ID | Source and locator | Evidence role | Reproducible artifact |
|---|---|---|---|
| A | JaguarForums thread URLs supplied for F6; publicly indexed OBD Fusion report dated 2016-03-15/19 | Direct observation of VIN, `0x7E8`, and `CX23-14C204-ZAD` | `fixtures/knowledge/captured/x250_obd_fusion_mode09.normalized.txt`, SHA-256 `310ff355a053adc3430269c4903084c619b95a38e4d765ff87a767eac579641b` |
| B | Jaguar TSB `JTB00244NAS1`, pages 1-2, 24 February 2012 | OEM applicability and ECM diagnostic context | `jtb00244nas1.pdf`, SHA-256 `ff4d405e212b5e1f973c41c08aabd929043b7ec084da5d212af19b9ca4347202` |
| C | 2010 Jaguar XF (X250) V8-5.0L SC service-manual bundle, pages 7012-7014 and 7024 | Exact vehicle physical network route | bundle SHA-256 `e09b54db2571a984190a504ab7c7f4f38eefb842d496f0286dc6d0676ff3106e` |
| D | California BAR Data Acquisition Device Specification V2.5, PDF page 14 / printed page 13, section 3.2.46 | SAE J1979 Mode `09` InfoType `04` meaning | `bar-dad-2012.pdf`, SHA-256 `9138a95fbfef44483c7efd1149405d93e98736f89b31a2eb9b9670b0bd4d2a9c` |
| E | Elm Electronics ELM327DSJ, pages 41 and 43 | ISO 15765-4 11-bit functional/physical/response address roles and Calibration ID example | `elm327ds.pdf`, SHA-256 `43cab005c9e1678325a3ba64066db93e09e2a78a23eb5fa360ee1eaf9a585428` |
| F | `crates/mongoose-jlr/src/passive.rs`, production HS-CAN descriptor | Backend route | SHA-256 `79180a11e16f2398cec8d08702ba369e0935517e92da8835774e327b3d16998c` |

The original forum attachment could not be lawfully retrieved: the site denied
direct requests and its public index was the available reproducible surface.
Source A is therefore a minimal normalized snapshot of indexed facts, not a
copy of the thread or attachment. Its provenance and limitation are explicit in
the fixture. It is used only for the facts that the report actually observes.

## Field-level evidence matrix

| Resolved field | Value | Source | Class | Result |
|---|---:|---|---|---|
| vehicle program | Jaguar XF X250 | B + C | OEM documentation | supported |
| model year / engine | 2010 / 5.0L supercharged | A + B + C | direct observation + OEM documentation | supported |
| ECU family | ECM/PCM | B + C | OEM documentation | supported |
| logical network | HS-CAN | C | OEM documentation | supported |
| connector | J1962 / C2DB04B diagnostic connector | C | OEM documentation | supported |
| physical pins | 6 / 14 | C | OEM documentation | supported |
| bitrate | 500000 bit/s | C | OEM documentation | supported |
| backend route | `mongoose-jlr` / `hs-can` | F | source code | supported |
| protocol | ISO 15765-4 CAN | E | standard documentation | supported |
| CAN ID format | 11-bit | E | standard documentation | supported |
| functional request ID | `0x7DF` | E | standard documentation | supported |
| physical request ID | `0x7E0` | E | standard documentation | supported for the independently observed `0x7E8` responder |
| response ID | `0x7E8` | A | direct observation | supported |
| read-only capability | Mode `09`, InfoType `04`, Calibration ID | D | standard documentation | supported |
| calibration ID | `CX23-14C204-ZAD` | A | direct observation | supported |

## Join and semantics

VIN suffix `R59890` falls inside Jaguar TSB range `R47154-S38706`. The report's
2010 supercharged XF context joins the TSB's X250 5.0L ECM applicability and the
exact 2010 X250 5.0L SC service-manual network diagram. The manual independently
joins the ECM, HS-CAN, diagnostic connector, 500 kbit/s, and pins 6/14. The
production descriptor independently supplies the current backend route.

The address roles remain deliberately separate:

- `0x7DF` is a standards-documented functional request ID;
- `0x7E0` is a standards-documented physical request ID for the observed
  `0x7E8` responder;
- `0x7E8` is directly observed in Source A;
- Source A does not observe or prove `0x7DF`, `0x7E0`, bitrate, physical pins,
  protocol, or backend support;
- neither request ID nor response ID is computed by adding or subtracting
  eight.

## Exclusions

- No CAN route on pins 12/13 is created. The existing CCP negative golden
  remains INDETERMINATE and unchanged.
- No live operation, CAN TX API, diagnostics-core mapping, ISO-TP execution, or
  UDS request is added.
- No vehicle, Mongoose adapter, or CAN channel was used.
