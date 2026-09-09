<#
.SYNOPSIS
  Run everything CI runs, in one command, before pushing.

.DESCRIPTION
  On 2026-09-09 a push failed CI on `cargo fmt --check` — a step that was in
  the CI job and in none of the local checks, because the local checks were
  whatever anyone remembered to type. Twice, on two commits, for one missing
  line. This script is that list, so remembering is not part of it.

  It runs, in order and stopping for nothing:

    1. the architecture boundaries;
    2. the frontend lint, tests and build (through path filters, so a rename
       cannot silently match no project again);
    3. the portable Rust crates in Docker — fmt, clippy, tests;
    4. the Tauri shell crate in Docker with GTK and WebKit — clippy, tests,
       including the end-to-end bench test.

  Rust never runs natively on the owner's host, so 3 and 4 need Docker
  Desktop running. -SkipShell leaves out step 4, which is the slow one;
  use it only for a change that cannot touch the shell.

.EXAMPLE
  powershell -File scripts/preflight.ps1
  powershell -File scripts/preflight.ps1 -SkipShell
#>
param(
  [switch]$SkipShell,
  [string]$RegistryVolume = "jlr-cargo-registry"
)

$repo = Split-Path -Parent $PSScriptRoot
$results = New-Object System.Collections.ArrayList

# pnpm is not always on PATH in a non-interactive shell, and the root scripts
# call pnpm again for the workspace project, so the child needs it too.
$npmDir = Join-Path $env:APPDATA "npm"
if (Test-Path (Join-Path $npmDir "pnpm.cmd")) { $env:PATH = "$npmDir;$env:PATH" }
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
  Write-Host "pnpm not found on PATH; install it or add its directory" -ForegroundColor Red
  exit 2
}

function Invoke-Step {
  param([string]$Name, [scriptblock]$Body)
  Write-Host ""
  Write-Host "=================== $Name ===================" -ForegroundColor Cyan
  & $Body
  $ok = ($LASTEXITCODE -eq 0)
  [void]$results.Add([pscustomobject]@{ Step = $Name; Result = $(if ($ok) { "ok" } else { "FAILED" }) })
  if (-not $ok) { Write-Host "$Name FAILED" -ForegroundColor Red }
}

Invoke-Step "architecture boundaries" { node (Join-Path $repo "scripts/check-architecture.mjs") }
Invoke-Step "frontend lint"  { pnpm --filter ./apps/scanner/frontend lint }
Invoke-Step "frontend tests" { pnpm --filter ./apps/scanner/frontend test }
Invoke-Step "frontend build" { pnpm --filter ./apps/scanner/frontend build }

Invoke-Step "rust: fmt, clippy, tests" {
  docker run --rm -v "${repo}:/w" -w /w -v "${RegistryVolume}:/usr/local/cargo/registry" `
    rust:1.98-slim sh /w/scripts/preflight-workspace.sh
}

if (-not $SkipShell) {
  Invoke-Step "shell crate: clippy, tests" {
    docker run --rm -v "${repo}:/w" -w /w -v "${RegistryVolume}:/usr/local/cargo/registry" `
      rust:1.98-bookworm sh /w/scripts/preflight-shell.sh
  }
} else {
  [void]$results.Add([pscustomobject]@{ Step = "shell crate: clippy, tests"; Result = "skipped" })
}

Write-Host ""
Write-Host "=================== preflight ===================" -ForegroundColor Cyan
$results | Format-Table -AutoSize
if ($results | Where-Object { $_.Result -eq "FAILED" }) {
  Write-Host "Not ready to push." -ForegroundColor Red
  exit 1
}
if ($SkipShell) { Write-Host "Ready to push, but the shell crate was skipped." -ForegroundColor Yellow }
else { Write-Host "Ready to push." -ForegroundColor Green }
