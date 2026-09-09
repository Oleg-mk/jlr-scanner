<#
.SYNOPSIS
  Issue a signed, dated copy of the exported library to one named tester, and zip it.

.DESCRIPTION
  Runs the stamp_library tool in Docker (the Rust toolchain never runs on
  this host) with the owner's issuing key mounted from ~\.jlr-scanner.
  Every manifest gets "[issued CODE]" in its notes; issued_to.json records
  the name, the dates, the code, each bundle's SHA-256 and the owner's
  signature (ADR-0019). The tool reads the copy back as the application
  will and refuses to finish if it would not load. The result is zipped
  next to the output folder. Hand the zip to that tester only.

.EXAMPLE
  .\scripts\stamp-library.ps1 -IssuedTo "Ivan Ivanenko"
  .\scripts\stamp-library.ps1 -IssuedTo "Oleg" -Days 365
#>
param(
  [Parameter(Mandatory = $true)][string]$IssuedTo,
  [int]$Days = 30,
  [string]$Library = "$env:USERPROFILE\Downloads\prowlone-library",
  [string]$OutRoot = "$env:USERPROFILE\Downloads\prowlone-issued",
  [string]$KeyDir = "$env:USERPROFILE\.jlr-scanner"
)

$repo = Split-Path -Parent $PSScriptRoot
$slug = ($IssuedTo -replace '[^\p{L}\p{N}]+', '_').Trim('_')
if ($slug -eq '') { throw "the name must contain letters or digits" }
if (-not (Test-Path (Join-Path $KeyDir 'library-issuer.key'))) {
  throw "no issuing key in $KeyDir; make one with: stamp_library --new-key /issuer/library-issuer.key"
}
$out = Join-Path $OutRoot $slug
New-Item -ItemType Directory -Force $out | Out-Null

docker run --rm `
  -v "${repo}:/w" -w /w `
  -v jlr-cargo-registry:/usr/local/cargo/registry `
  -v "${Library}:/library" -v "${out}:/out" -v "${KeyDir}:/issuer" `
  rust:1.98-slim sh -c "cargo run --release -q -p diagnostic-session --example stamp_library -- /library /out `"$IssuedTo`" $Days"
if ($LASTEXITCODE -ne 0) { throw "stamping failed (exit $LASTEXITCODE)" }

$zip = Join-Path $OutRoot "prowlone-library-$slug.zip"
Compress-Archive -Path "$out\*" -DestinationPath $zip -Force
Get-Content (Join-Path $out 'issued_to.json') -TotalCount 7
"zip: $zip ({0:N1} MB)" -f ((Get-Item $zip).Length / 1MB)
