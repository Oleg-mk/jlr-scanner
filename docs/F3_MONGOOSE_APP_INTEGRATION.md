# F3 Mongoose Application Integration

## Scope and status

F3 is the first production application vertical slice:

`React UI -> Tauri application service -> transport-serial -> mongoose-jlr -> MongoosePro JLR`

Implementation status: `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`.

Physical USB-only acceptance status: `PASS / NATIVE_USB_VALIDATED`. On 2026-08-31, the real native Tauri application completed unplugged discovery, USB enumeration, production connect and `0x8109`, UI disconnect/reconnect, physical unplug fail-closed handling, and physical replug rediscovery with a real MongoosePro JLR and no vehicle. See `docs/evidence/F3_NATIVE_USB_ACCEPTANCE_2026-08-31.md`.

F1 hardware-confirmed `Windows 11 -> usbser -> production Rust -> Mongoose -> 0x8109`. F3 now also hardware-confirms the added `native Tauri window -> React UI -> existing production backend` path.

F2B vehicle validation remains `DEFERRED_BY_OWNER_DECISION`. No vehicle is required or permitted for F3 acceptance.

## Application behavior

The application exposes five user states: `NO_ADAPTER`, `ADAPTER_DETECTED`, `CONNECTING`, `CONNECTED`, and `ERROR`.

- discovery filters Windows serial devices dynamically by USB VID/PID `18E1:0104`;
- no COM port is hardcoded;
- one adapter is selected automatically;
- multiple adapters require explicit user selection;
- the user must click **Connect** before the serial transport opens;
- connect uses the production `mongoose-jlr` device and performs only the allowlisted `cGetBoardInfo` round trip;
- a valid board response is represented by response command `0x8109`; undecoded bytes remain raw technical evidence;
- polling handles initial absence, plug, unplug, changed COM port, and replug without a second UI implementation;
- user-facing messages are separated from optional technical error details.

## Tauri boundary

The shell exposes only application intents:

- `get_adapter_state`;
- `discover_adapters`;
- `connect_adapter`;
- `disconnect_adapter`.

Mongoose framing, command IDs, verifier calculation, byte-stream decoding, and board-response validation remain in the production crates. The Tauri application service composes discovery, connection, DTO state, and errors; it does not implement the protocol.

An architecture regression check rejects Mongoose packet/frame symbols and CAN channel, receive, pin-routing, or listen-only protocol calls in the Tauri source.

## Capability truth shown after connect

| Logical capability | Physical route | Implementation evidence | Vehicle validation |
| --- | --- | --- | --- |
| HS-CAN | JLR DLC pins 6/14, 500 kbit/s | `IMPLEMENTED`, `FIXTURE_TESTED`, `HARDWARE_CONFIRMED` route | `NOT_YET_VALIDATED` |
| MS-CAN | JLR DLC pins 3/11, 125 kbit/s | `IMPLEMENTED`, `FIXTURE_TESTED`, `HARDWARE_CONFIRMED` route | `NOT_YET_VALIDATED` |
| CCP HS-CAN | X250 vehicle-side pins 12/13 | `UNSUPPORTED_BY_ADAPTER`; no MongoosePro JLR route | `NOT_APPLICABLE` |

Capability rows are information only. F3 does not open any CAN channel. Pins 12/13 are not configured or touched.

## Safety boundary

F3 contains no application CAN TX API and performs no automatic vehicle command. It must not connect to an X250 OBD connector, configure HS-CAN or MS-CAN, capture frames, send CAN/ISO-TP/UDS/KWP, discover ECUs, or invoke firmware/programming paths.

The only real-device write allowed by F3 is the already F1-accepted, explicitly user-triggered board-info request after serial connection. Disconnect closes the transport.

## Automated validation

Local source-validation evidence on Windows 11:

- final `cargo fmt --all -- --check`: pass;
- final `cargo clippy --workspace --all-targets -- -D warnings`: pass;
- a full run of the current Rust source passed all 46 workspace unit tests before the native-launch attempt;
- the final post-launch-attempt `cargo test --workspace` rerun started successfully, then Windows Application Control blocked the newly linked `jlr_scanner_shell` test executable (`os error 4551`); this rerun is `BLOCKED`, not green;
- final `pnpm lint` and architecture checks: pass;
- final `pnpm test` with real UI rendering and user actions: 6/6 pass;
- final `pnpm build`: pass.

GitHub CI provided clean, policy-independent complete workspace runs for the branch. This documentation-only status update does not rerun them.

Covered application cases include zero, one, and multiple adapters; explicit selection; connect and board-info success; open/board-info failure mapping; disconnect; hot unplug/replug; changed COM port; truthful capability states; and UI transitions. Codec behavior continues to use the existing production golden fixtures rather than a mocked codec.

## Physical USB-only acceptance record

Current result: `PASS / NATIVE_USB_VALIDATED`.

On Windows 11 Pro 25H2 build `26200.9278`, Smart App Control was temporarily changed from `ON` to `OFF` through the supported Windows Security UI. Defender, Tamper Protection, VBS, and Code Integrity enforcement remained active; no bypass or registry write was used. The native `target/debug/jlr-scanner-shell.exe` then launched through the repository's `pnpm tauri dev` command. After acceptance, Smart App Control was restored to `ON` and verified.

Observed USB-only results:

1. Physically unplugged: no present `18E1:0104`; native UI showed **Adapter not detected**.
2. Physically plugged, no vehicle: Windows and the native UI detected `18E1:0104` on dynamically resolved `COM3` using `USB CDC / Serial` and `usbser / mongoose-jlr`.
3. Native **Connect**: **Connected**, **Board communication Verified**, response command `0x8109`.
4. Exact raw board-info: 168 bytes, SHA-256 `EF7C4F017856A4954B43239FFC5C0815F08345E8D6410E7EBDF67375457732C6`.
5. Native **Disconnect -> Connect**: both transitions passed; the second connection returned to **Verified**.
6. Physical unplug while connected: native UI automatically changed to **Adapter disconnected** without a crash or stale connected state.
7. Physical replug: native UI automatically rediscovered the adapter and exposed **Connect** on current `COM3`.

No vehicle was used, no CAN channel was opened, and no CAN TX API was added. The exact response and Windows control audit are retained in `docs/evidence/F3_NATIVE_USB_ACCEPTANCE_2026-08-31.md`.

## Responsive render QA

Result: `PASS` for the rendered frontend baseline, not for physical USB acceptance.

- viewports: desktop `1440x1000`, tablet `900x1100`, phone `390x844`;
- NO_ADAPTER and application-boundary CONNECTED states were rendered;
- sections stack at tablet/phone widths and capability rows become labelled vertical records on phone;
- no horizontal overflow, console error, or page error was observed;
- primary/disconnect button height was 44 px;
- capability information remained hidden before connect;
- accepted ImageGen desktop/mobile concepts were visually compared with the final screenshots.

The in-app Browser runtime was attempted first but could not start because the Codex Windows ACL helper failed with `apply deny-read ACLs`. The fallback used the installed Edge executable through the already installed `playwright-core`; no dependency was added to the repository. Connected-state data was mocked only at the application/Tauri boundary for render QA, consistent with the test policy. This does not claim real adapter detection.

## Native USB validation gate

F3 is `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`. The native application launched and real Mongoose detection/connect/`0x8109`/disconnect/unplug/replug succeeded through the production application path with no vehicle. This does not complete F3.1 trusted distribution: production users must not be required to disable SAC, so F3.1 remains `DEFERRED_EXTERNAL_DEPENDENCY — requires trusted signing identity`. Merge and tag decisions remain separate owner actions.
