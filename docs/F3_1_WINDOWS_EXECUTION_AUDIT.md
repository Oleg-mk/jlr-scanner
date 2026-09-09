# F3.1 Windows Execution Audit

Audit date: 2026-08-30. Host: Windows 11 Pro 64-bit, version `10.0.26200`, build `26200`.

## Result

The F3 native launch failure is a Smart App Control Code Integrity denial. The active policy identifies itself as `VerifiedAndReputableDesktop`; it rejects newly produced, unsigned PE files with Code Integrity status `0xc0e90002`, surfaced to Cargo/Tauri as Windows error `4551`.

The failures do not all name the same file type. They include the ProwlOne application, Rust tests, Cargo build scripts, and a proc-macro DLL. In the sampled events they do share the same active Code Integrity policy and status. No matching AppLocker blocking event was found.

No Smart App Control, Defender, Secure Boot, Code Integrity, driver-policy, or exclusion setting was changed.

## Smart App Control and policy evidence

- `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy\VerifiedAndReputablePolicyState = 1`.
- Code Integrity Operational policy name: `VerifiedAndReputableDesktop`.
- policy ID: `27555.1000.240208`.
- policy GUID: `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`.
- policy hash: `2668895A5B233A80432D00D67251D7B7F52686A3FB13780F4B242C5A1F937A01`.
- requested signing level: `2`; validated signing level: `1`.
- signing scenario: `1`; `UserWriteable=false`.
- denial status: `0xc0e90002`.
- event channel: `Microsoft-Windows-CodeIntegrity/Operational`.
- relevant event IDs: `3033` (did not meet Enterprise signing level) and `3077` (did not meet the signing level or violated CI policy).
- `CiTool.exe -lp -json` returned access denied (`0x80070005`) without elevation. The registry and event payloads were sufficient to identify the enforcing policy; no elevation or policy modification was attempted.

Microsoft documents that Smart App Control permits an app when Microsoft intelligence recognizes it as safe or it is signed by a certificate issued by a CA in the Microsoft Trusted Root Program. Unknown unsigned code is blocked by default: [Smart App Control overview](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview).

## Blocked files

All listed files are PE executable code, have Authenticode status `NotSigned`, have no signer, and are not Windows OS binaries.

| Class | File | SHA-256 flat/file hash | CI evidence |
| --- | --- | --- | --- |
| A — product application | `target/debug/prowlone-shell.exe` | `4575F2CF991D90978860636A5235F0B039916A55C002A559D613C0F533521772` | event 3077, record 4769 |
| D — Rust test | `target/debug/deps/jlr_scanner_shell-884f4e3f479ec17d.exe` | `F6A50268781F44B5AA9DD97A14E732CA62B1495360E758E1A3D2C994F8331F93` | event 3077, record 4774 |
| C — Cargo build script | `target/release/build/icu_normalizer_data-28623ffab73afea6/build-script-build.exe` | `CA400FE55835BDC2BFE781F3EE5D8A6CD69CF6377FF7386D14169383252DA398` | event 3077, record 4744 |
| C — Cargo build script | `target/release/build/serde_core-ac5dac6de05cef64/build-script-build.exe` | `0345ABE8A1928C18DDF069CC19C429865AF1657982A51D94F0471A4DDBEE4693` | matching 4551/CI sample |
| C — Cargo build script | `target/release/build/serde_core-4d66aa363a39b054/build-script-build.exe` | `9CEACCFBB91FAEEC9756069A154164BB2018EC50BF9F8CAED1C306DE85EF1BA8` | matching 4551/CI sample |
| C — Cargo build script | `target/release/build/parking_lot_core-b8592bb93fc193fe/build-script-build.exe` | `45E1BB6C01F67A06753660D59049BEFB2D55863663F15EA6A7FAD26B827EC15A` | matching 4551/CI sample |
| E — proc-macro DLL loaded by Rust | `target/debug/deps/windows_implement-3992c680c98582e3.dll` | recorded as unsigned PE during audit | event 3033/3077 sequence, record 4764 |
| E — proc-macro DLL loaded by Rust | `target/debug/deps/phf_macros-6568bdd800537b5e.dll` | recorded as unsigned PE during audit | repeated earlier Code Integrity denials |

The main application file was 12,883,456 bytes, last written `2026-08-30T12:54:03+03:00`; its Code Integrity flat hash exactly matched `Get-FileHash`. The test executable was 899,584 bytes. The sampled build scripts were 136,704 to 281,088 bytes.

No separate Tauri helper/bootstrapper was the observed launch blocker because bundling did not complete locally. The release build stopped earlier on Cargo build-script execution. AppLocker EXE/DLL logs contained no matching JLR denial; informational event 8044 entries related to rustup command-line checks are not the cause.

## Authenticode before F3.1

- ProwlOne debug executable: `NotSigned`.
- sampled Cargo/test executables and proc-macro DLLs: `NotSigned`.
- signer: none.
- installed verification tool: Windows SDK SignTool `10.0.26100.0` (x64/x86/arm64 variants present).
- Azure CLI: not installed locally.
- repository GitHub Actions signing secrets/environments: none found at audit time.
- trusted signing account/profile belonging to ProwlOne: not found.

## Development versus product distribution

Cargo build scripts, test executables, and proc-macro DLLs are temporary development outputs; they are not release payloads and will not be mass-signed. GitHub CI already provides a Windows environment where the Rust/frontend suites pass. F3.1 therefore treats local temporary-binary execution separately from the production requirement: build the actual release in CI, trusted-sign it, verify it, download it to this protected Windows 11 host, and use that exact product artifact for USB-only F3 acceptance.
