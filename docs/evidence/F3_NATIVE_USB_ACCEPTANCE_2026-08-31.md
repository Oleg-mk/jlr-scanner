# F3 native USB acceptance — 2026-08-31

## Scope

This is adapter-only evidence for the production application path:

`native Tauri window -> React UI -> Tauri commands -> transport-serial -> mongoose-jlr -> MongoosePro JLR`

No vehicle was connected. No CAN channel was opened. The application exposed no CAN transmit API.

## Windows development unblock

- Observation started: `2026-08-31T15:19:49+03:00`.
- OS: Windows 11 Pro, version 25H2, build `26200.9278`.
- Relevant installed cumulative update: `KB5120998`.
- Smart App Control before acceptance: `ON` (`VerifiedAndReputablePolicyState = 1`).
- Previous Code Integrity evidence: events `3033` and `3077`, policy `VerifiedAndReputableDesktop` / `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, rejecting unsigned Cargo-produced executables. The historical native-launch surface reported Windows error `4551`.
- Current Microsoft guidance: the [Smart App Control FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions) states that recent Windows updates allow SAC to be disabled and re-enabled through Windows Security without a clean installation.
- Temporary change: only Smart App Control was changed from `ON` to `OFF`, using `Windows Security -> App & browser control -> Smart App Control settings`.
- No registry write, Code Integrity bypass, AppLocker bypass, exclusion, test-signing mode, driver bypass, or security-service change was used.
- Defender antivirus, real-time protection, and Tamper Protection remained enabled. VBS and kernel/user-mode Code Integrity enforcement remained active.
- Secure Boot readback was `UNKNOWN` because the read-only PowerShell query was denied the required privilege; Secure Boot was not changed.
- Memory Integrity direct readback was unavailable; it was not changed.
- Final Smart App Control state: `ON`, confirmed both by Windows Security selection and `VerifiedAndReputablePolicyState = 1`.

## Native launch

The repository-declared command `pnpm tauri dev` launched the real native process:

`C:\Users\<you>\Desktop\jlr-scanner\target\debug\jlr-scanner-shell.exe`

The top-level window class was `Tauri Window`, title `JLR Scanner`. Vite served the WebView development content at the configured `127.0.0.1:1420`, but acceptance interactions and observations were made through the native Tauri window. No external browser preview was substituted.

The first launch attempt found a stale JLR Scanner Vite process on port 1420. That exact repository-owned process was stopped, and the next native launch succeeded. Windows error 4551 did not recur. No source defect was found.

## USB-only sequence

1. With the adapter physically unplugged, Windows reported no present `VID_18E1&PID_0104` device and the native UI showed `Adapter not detected`.
2. After USB-only plug-in, Windows enumerated `USB Serial Device (COM3)`, class `VehiclePassThru`, bus-reported description `Drew Technologies Inc.`, Microsoft `usbser` driver `10.0.26100.9278`.
3. The native UI showed `MongoosePro JLR detected`, USB VID/PID `18E1:0104`, dynamically resolved `COM3`, transport `USB CDC / Serial`, and backend `usbser / mongoose-jlr`.
4. Native UI **Connect** performed the allowlisted production board-info operation. UI state became `Connected`; `Board communication` became `Verified`.
5. Native UI **Disconnect** returned to detected/not-connected state. A second **Connect** returned to `Connected / Verified`.
6. Physical unplug while connected removed the PnP device and the native UI automatically changed to `Adapter disconnected`; it did not retain a false connected state or crash.
7. Physical replug enumerated the same VID/PID and current `COM3`; the native UI automatically returned to `MongoosePro JLR detected` with **Connect** available.

The device serial was observed for identity matching but is intentionally omitted from this retained report.

## Real board-info response

- Response command: `0x8109`.
- Raw board-info length: `168` bytes.
- Raw board-info SHA-256: `EF7C4F017856A4954B43239FFC5C0815F08345E8D6410E7EBDF67375457732C6`.
- Exact raw board-info bytes:

```text
00 00 00 00 00 00 00 00 05 02 01 01 B4 B0 00 00 D8 55 00 00 00 08 01 01 00 10 01 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

## Acceptance result

- Native Tauri launch: `PASS`.
- Unplugged detection: `PASS`.
- Plugged detection and production transport identity: `PASS`.
- Board-info `0x8109`: `PASS` with retained exact response.
- Application disconnect/reconnect: `PASS`.
- Physical unplug fail-closed transition: `PASS`.
- Physical replug rediscovery: `PASS`.
- Vehicle used: `NO`.
- CAN channel opened: `NO`.
- Application CAN TX API: `NO`.
- F3 defects found: `NONE`.
- F3 source changes: `NONE`.
- Tests rerun: `NONE`; this was a physical native acceptance run with no code change, and the existing CI/fixture evidence was not repeated.

F3 status: `IMPLEMENTED / CI_FIXTURE_TESTED / NATIVE_USB_VALIDATED`.

F3.1 status remains `DEFERRED_EXTERNAL_DEPENDENCY — requires trusted signing identity`. Temporary local SAC control is not a production distribution solution.
