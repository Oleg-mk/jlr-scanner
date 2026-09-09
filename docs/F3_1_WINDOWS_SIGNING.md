# F3.1 Windows Trusted Signing Baseline

Status: `DEFERRED_EXTERNAL_DEPENDENCY — requires trusted signing identity`.

Selected production path: `OPTION_B_OV_CERTIFICATE`.
Previous Option A selection: `BLOCKED_PENDING_OWNER_REGION_ELIGIBILITY`.
Owner signing region: `UKRAINE`, entity: individual entrepreneur (ФОП), decided 2026-09-03. Option A is therefore closed: Azure Artifact Signing Public Trust lists organisations in the United States, Canada, the European Union, the United Kingdom, Australia, New Zealand, Japan, South Korea, Singapore, Switzerland, Norway and Israel, and individuals in the United States and Canada only (Microsoft, Artifact Signing quickstart, checked 2026-09-03). Option B is the path: an RSA code-signing certificate from a Microsoft Trusted Root CA, issued either as OV under the CA/B Forum sole-proprietor procedure against the Ukrainian state business register or as IV on the owner's own name; both satisfy Smart App Control. Which one a CA applies to a Ukrainian ФОП is the first question to ask that CA. Sources: https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart ; https://www.ssl.com/products/software-integrity/code-signing/ev-sole-proprietor/ ; https://www.namecheap.com/support/knowledgebase/article.aspx/9470/68/can-i-get-an-ovev-certificate-if-i-apply-as-an-individual-and-dont-have-a-registered-organization/

Interim decision 2026-09-03: the certificate — account, identity verification, purchase — is deferred until real vehicle results exist. Until then the unsigned CI build runs on the owner's own hardware only, preferably the Windows 10 laptop, which has no Smart App Control and admits the build through SmartScreen's per-file allow; switching Smart App Control off on the Windows 11 desktop is the fallback, only after confirming the switch is reversible on that build. No stranger ever installs an unsigned build. The signing-ready configuration is retained but inactive. No signing workflow, trusted-builder bootstrap, account creation, identity verification, billing, certificate purchase, credential creation, signed build, or signature verification is in progress. F3.1 is not a blocker for unrelated project development and resumes only after an explicit owner decision.

## Decision

The production path is an **RSA OV code-signing certificate from a CA in the
Microsoft Trusted Root Program** (Option B), signed in CI through that CA's
hardware or cloud-HSM integration. A signed standalone application executable
and signed NSIS installer must be verified before either can be uploaded as a
workflow artifact.

Azure Artifact Signing Public Trust (Option A) was the previously selected
strategy. It is now `BLOCKED_PENDING_OWNER_REGION_ELIGIBILITY` because Microsoft
restricts Public Trust certificates to an explicit region list that the owner
has not been confirmed to satisfy. Option A is not rejected on technical
grounds; if the owner is confirmed eligible, it remains the lower-maintenance
choice because Microsoft holds the signing key. Until that confirmation exists,
the region gate is treated as unresolved and Option B is the planning baseline.

This is a production distribution path, not a local Smart App Control workaround. Self-signed certificates are prohibited. Smart App Control and other Windows protections remain enabled.

Official basis:

- Smart App Control accepts recognized code or code signed by a certificate issued by a CA in the Microsoft Trusted Root Program: [Smart App Control overview](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview).
- Smart App Control requires RSA-based certificates; ECC signatures are not currently supported: [Sign your app for Smart App Control compliance](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control).
- Public Trust Artifact Signing explicitly supports Win32 code signing and Windows 11 Smart App Control; Public Trust Test and Private Trust are not publicly trusted substitutes: [Artifact Signing trust models](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-trust-models).
- Tauri 2 supports a custom `bundle.windows.signCommand` and documents Artifact Signing integration: [Tauri 2 Windows code signing](https://v2.tauri.app/distribute/sign/windows/).
- SignTool recommends SHA-256, `/pa` for the default Authenticode policy, and `/tw` to warn on a missing timestamp: [SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool).
- RFC 3161/SHA-256 timestamping preserves long-term signature validity: [Authenticode timestamping](https://learn.microsoft.com/en-us/windows/win32/seccrypto/time-stamping-authenticode-signatures).
- Artifact Signing Public Trust is restricted to an explicit region list for organizations and to the US/Canada for individual developers: [Artifact Signing quickstart](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart).

## Region eligibility gate

Verified 2026-09-01 against the Artifact Signing quickstart prerequisites:

> Public Trust certificates are available to organizations in the United States,
> Canada, the European Union, the United Kingdom, Australia, New Zealand, Japan,
> South Korea, Singapore, Switzerland, Norway, and Israel. Individual developers
> must be located in the United States or Canada. These geographic restrictions
> do not apply to Private Trust certificates.

Consequences for this project:

- the organization list does not include every European country; membership in
  the European Union is the operative test, not geographic Europe;
- an individual developer outside the United States or Canada is ineligible for
  Public Trust regardless of the organization list;
- Private Trust and Public Trust Test profiles are exempt from the restriction
  but are not publicly trusted, so they cannot satisfy Smart App Control and are
  not substitutes;
- Artifact Signing *resource* regions are a separate axis. Creating an account in
  a listed Azure region does not grant Public Trust eligibility to an ineligible
  legal entity.

`OWNER_SIGNING_REGION = UNKNOWN`. Per principle 4 this stays `UNKNOWN` until the
owner records the legal entity type (individual or organization) and its
registered country. That single fact selects the path:

| Owner situation | Eligible for Option A | Path |
| --- | --- | --- |
| Organization registered in a listed region | Yes | Option A preferred |
| Individual in the US or Canada | Yes | Option A preferred |
| Individual outside the US/Canada | No | Option B |
| Organization outside the listed regions | No | Option B |

Nothing in the repository establishes the owner's registered country, so no
eligibility claim is recorded here.

## Production options compared

| Option | Suitable | Smart App Control | Eligibility/account/cost | Local and CI signing | Key/secrets | Tauri 2 / complexity |
| --- | --- | --- | --- | --- | --- | --- |
| A. Azure Artifact Signing Public Trust | **CONDITIONAL — blocked pending region eligibility** | Yes, RSA Public Trust profile | Azure subscription, billing, identity validation, account/profile; Microsoft currently quotes about USD 9.99/month. Restricted to the organization region list and to US/Canada individuals; see "Region eligibility gate". | Local CLI and official GitHub integration supported | Microsoft holds the signing key. This baseline stores only Azure app credentials in GitHub Secrets and non-secret profile metadata in GitHub Variables. | Direct Tauri 2 `signCommand`; low operational key-management burden. |
| B. Traditional OV code-signing certificate from a Microsoft-trusted CA | **YES, selected** | Yes when RSA and correctly chained | Worldwide CA availability, so no region gate; Microsoft estimates USD 150–300/year; identity validation. Since June 2023 OV private keys require an HSM/token or cloud HSM. | Local SignTool support; CI depends on token/cloud-HSM provider | Never commit PFX/private key. Hardware/cloud custody and provider authentication are owner responsibilities. | Supported through Tauri custom/default Windows signing; higher lifecycle and CI complexity. |
| C. Microsoft Store | **NO for the present Tauri installer baseline** | Store-signed MSIX is trusted | Free developer account, Partner Center enrollment, submission and review; MSIX signing is supplied by Store | Store signs MSIX, not current Tauri EXE/MSI output; no CI signing of those current outputs | No publisher key for Store MSIX | Tauri currently generates EXE/MSI installers, and its Store guidance requires that linked Win32 installers already be signed. Moving to an external MSIX packaging/distribution design is a separate decision. |

Current Microsoft guidance recommends Artifact Signing for non-Store distribution, says self-signed code is unsuitable for public deployment, and says EV no longer receives automatic SmartScreen bypass merely because it is EV: [Windows code-signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options). Artifact Signing eligibility and resource prerequisites are recorded in the [current quickstart](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart).

EV is not selected. Paying the EV premium solely for reputation is no longer justified by current Microsoft behavior.

## Option B constraints

If Option B is executed, these constraints are binding:

- the certificate must be **RSA**. Smart App Control does not currently support
  ECC signatures, so an ECC profile would produce a signed artifact that Smart
  App Control still blocks;
- the private key must live on an HSM/token or cloud HSM. Choose a CA whose
  cloud-HSM offering exposes a non-interactive signing API, otherwise release
  signing cannot run in CI and degrades to a manual token operation;
- Smart App Control accepts a correctly chained Trusted Root Program signature
  directly. There is no reputation-accrual waiting period for Smart App Control;
- SmartScreen is a separate mechanism. A new publisher identity can still raise
  SmartScreen warnings until reputation accrues. That is a warning, not a block,
  and it is not an F3.1 acceptance criterion;
- the committed pipeline is Option A shaped. `artifact-signing-cli` and the seven
  `windows-signing` environment values below are Artifact Signing specific and
  must be replaced by the chosen CA's signing client and credential model before
  a release build can run. `verify-windows-release.ps1` is provider-neutral and
  is expected to survive that change unmodified.

## Committed signing architecture

The manual `Windows signed release candidate` workflow performs:

1. checkout;
2. fail-closed check for all required trusted-signing configuration;
3. pinned Rust `1.98.0`, Node `24.15.0`, and pnpm `11.19.0` setup;
4. frontend lint/tests/build plus Rust fmt/clippy/tests/shell check;
5. pinned `artifact-signing-cli 0.11.0` installation;
6. Tauri release build using `tauri.release.conf.json`;
7. in-process signing through `sign-windows-artifact.ps1`;
8. Authenticode, expected-publisher, timestamp, SHA-256, and SignTool verification;
9. upload of verified signed files and `signature-manifest.json` only.

The workflow is `workflow_dispatch` only. It does not create a GitHub Release, merge PR #3, or create a tag. Missing configuration stops before validation/build and no artifact upload step can run after a failure.

GitHub only exposes a `workflow_dispatch` workflow after that workflow exists on the default branch. Because PR #3 must not merge before signed execution, the committed branch workflow is signing-ready configuration but is not yet a runnable pre-merge trusted builder. Safe activation requires a separate, reviewed signing-infrastructure workflow to reach the default branch before PR #3, or another owner-approved trusted-builder mechanism. A PR-label trigger is deliberately not used because it would let PR-controlled code request access to future signing credentials.

## Tauri signing configuration

Normal development keeps `tauri.conf.json` with bundling disabled. Release CI merges `tauri.release.conf.json`, which enables only NSIS and defines:

```json
"signCommand": "pwsh -NoProfile -File scripts/sign-windows-artifact.ps1 -FilePath %1"
```

The wrapper permits only `.exe` and `.msi` files below `target/release`, reads the endpoint/account/profile and Azure credentials from the environment, and fails on missing data or any nonzero signing exit code. It contains no tenant, account, profile, credential, certificate, or private key.

Required `windows-signing` GitHub environment values:

| Kind | Name | Value |
| --- | --- | --- |
| Secret | `AZURE_CLIENT_ID` | Microsoft Entra application/client ID authorized to sign |
| Secret | `AZURE_CLIENT_SECRET` | client secret for that application |
| Secret | `AZURE_TENANT_ID` | tenant ID |
| Variable | `JLR_ARTIFACT_SIGNING_ENDPOINT` | regional `https://...codesigning.azure.net` endpoint |
| Variable | `JLR_ARTIFACT_SIGNING_ACCOUNT` | ProwlOne Artifact Signing account name |
| Variable | `JLR_ARTIFACT_SIGNING_PROFILE` | RSA Public Trust certificate profile name |
| Variable | `JLR_EXPECTED_PUBLISHER` | exact Authenticode certificate Subject returned by the approved profile |

The principal needs the minimum `Artifact Signing Certificate Profile Signer` role on the intended certificate profile. Rotate/revoke the client secret in Azure and GitHub, never in git. Microsoft-managed workload identity/OIDC is preferable when the Tauri signing CLI path supports the chosen credential flow end-to-end; this baseline follows Tauri's documented environment credential path and never handles the signing private key.

## What is signed and shipped

Current Windows release payload decisions:

- `target/release/prowlone-shell.exe`: signed and uploaded as the standalone product executable;
- `target/release/bundle/nsis/*-setup.exe`: contains the already signed application executable, is itself signed, verified, and uploaded;
- sidecars: none configured;
- application-shipped DLLs: none currently configured;
- MSI: not generated by this NSIS-only baseline; if enabled later, it is automatically included in the `.msi` signing/verification allowlist;
- Cargo build scripts, proc-macro DLLs, unit-test executables, and other `target` temporaries: not shipped and deliberately not signed.

Verification explicitly enumerates the main executable plus every `.exe`, `.dll`, or `.msi` under the Tauri bundle output. Each must have `Valid` Authenticode status, a signer, a timestamp certificate, an exact configured publisher subject, and successful `signtool verify /pa /all /v /tw`. The workflow records SHA-256 after signing in `signature-manifest.json`. Any failure prevents artifact upload.

## Local development and F3 acceptance

Smart App Control can block newly linked local Cargo build scripts/tests. On 2026-08-31, the supported Windows Security SAC toggle was used temporarily for F3 native USB acceptance and SAC was restored to `ON` afterward. That local development result does not provide a trusted product artifact and does not change the production-distribution requirement.

That toggle is repeatable rather than a one-time exception. Disabling Smart App
Control was historically irreversible without a clean install or PC reset;
Microsoft shipped a reversible toggle in the March 2026 update
(`KB5079391`, superseded by `KB5086672`) and the April 2026 security update.
The development host is Windows 11 Pro 25H2 build `26200.9278`, which is past
both, and its `VerifiedAndReputablePolicyState = 1` after the 2026-08-31
restore confirms re-enablement worked. Hosts on older builds must still treat
disabling SAC as a one-way action. See the
[Smart App Control FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions).

Toggling per build is still the worst available workflow. Preferred order:

1. **Run platform-independent crates off Windows.** Only `transport-serial` and
   the `prowlone-shell` Tauri crate are Windows-bound; `mongoose-jlr` carries
   no OS-specific dependency and is portable despite owning the device
   protocol. `core-types`, `app-contracts`, `knowledge`,
   `diagnostic-environment`, `diagnostic-execution`, `isotp`, `uds`,
   `obd-j1979`, `transport-api`, `transport-replay`, `diagnostic-simulator`,
   `diagnostics-core`, `jlr-profiles`, and `mongoose-jlr` build and test under
   WSL2 or Linux with Smart App Control uninvolved. That covers every F4–F8
   golden test, including the F8 transport-fake Mongoose exchange:

   ```
   cargo test --workspace --exclude prowlone-shell --exclude transport-serial
   ```

2. **Treat CI as the source of record**, as the existing phase documents already
   do. GitHub `windows-latest` runners do not enforce Smart App Control.
3. **Use a separate VM or host** for `transport-serial` and native Tauri shell
   work, which genuinely require Windows.
4. **Toggle SAC only for a native acceptance run**, then restore it, exactly as
   F3 did on 2026-08-31.

Smart App Control has no exclusion or allowlist mechanism comparable to Defender
exclusions, and a self-signed certificate does not satisfy it, so neither is an
option for `target/`.

The deferred validation workflow, when explicitly reactivated, is:

1. let baseline GitHub CI prove source tests;
2. after owner approval, land the reviewed manual trusted-builder workflow on the default branch without merging PR #3, then dispatch it against the F3 branch;
3. download its verified artifact onto the protected Windows 11 machine;
4. confirm its Authenticode signature locally;
5. launch that exact signed application and complete F3 USB-only Mongoose acceptance with the vehicle disconnected.

The unsigned local development build completed the real UI detection/connect/`0x8109`/disconnect/hotplug/replug sequence with no vehicle; see `docs/evidence/F3_NATIVE_USB_ACCEPTANCE_2026-08-31.md`. Signed-product execution remains unclaimed because no trusted signing identity or signed release artifact exists. Production users must not disable SAC to run ProwlOne. PR merge and tag decisions remain separate owner actions.

## Requirements if trusted distribution is reactivated

No ProwlOne Artifact Signing account/profile or credentials exist. No immediate owner action is required while F3.1 remains deferred. If trusted distribution is explicitly reactivated later, the owner must choose one of these legitimate paths:

0. **Resolve the region gate first.** Record the signing legal entity type and its
   registered country, then compare against the "Region eligibility gate" table.
   No signing work should start before this, because it decides whether step 1 is
   available at all and prevents paying for an unusable Azure identity validation.
1. **If the region gate confirms eligibility:** use an existing or owner-created Azure subscription; register `Microsoft.CodeSigning`; create an Artifact Signing account; complete Public Trust identity validation; create an RSA Public Trust certificate profile; create a least-privilege Entra app credential with `Artifact Signing Certificate Profile Signer`; then populate the seven GitHub environment values above.
2. **Otherwise, the selected path:** obtain an RSA OV code-signing certificate from a CA participating in the Microsoft Trusted Root Program and choose its supported hardware/cloud-HSM CI integration. The repository pipeline must then be adapted to that provider before running a release build; see "Option B constraints".
3. Approve a separate reviewed bootstrap PR/commit that places the manual trusted-builder workflow on the default branch while PR #3 remains unmerged. Do not replace this with a PR-triggered signing job.

Identity verification, Azure subscription/billing, resource/profile creation, certificate purchase, and credential creation must be performed by the owner. None was attempted automatically. Public Trust Test, Private Trust, and self-signed profiles are not acceptable substitutes.

Until explicit reactivation: `F3_1 = DEFERRED_EXTERNAL_DEPENDENCY`, `GITHUB_ACTIONS_SIGNING = INACTIVE`, `SIGNED_PRODUCT_ARTIFACT = NOT_AVAILABLE`, `AUTHENTICODE_AFTER = NOT_RUN`, `WINDOWS_EXECUTION = NOT_RUN`, `OWNER_SIGNING_REGION = UNKNOWN`, and `SELECTED_PATH = OPTION_B_OV_CERTIFICATE`. When the owner later supplies an eligible trusted signing profile and approves the default-branch trusted-builder bootstrap, the workflow can be run and its signed artifact plus manifest retained as F3.1 evidence.
