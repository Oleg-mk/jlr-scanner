[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $FilePath
)

$ErrorActionPreference = 'Stop'

$required = @(
    'AZURE_CLIENT_ID',
    'AZURE_CLIENT_SECRET',
    'AZURE_TENANT_ID',
    'JLR_ARTIFACT_SIGNING_ENDPOINT',
    'JLR_ARTIFACT_SIGNING_ACCOUNT',
    'JLR_ARTIFACT_SIGNING_PROFILE'
)

$missing = @($required | Where-Object { [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($_)) })
if ($missing.Count -ne 0) {
    throw "Trusted signing is required. Missing environment variables: $($missing -join ', ')."
}

$resolved = (Resolve-Path -LiteralPath $FilePath).Path
$extension = [IO.Path]::GetExtension($resolved).ToLowerInvariant()
if ($extension -notin @('.exe', '.msi')) {
    throw "Refusing to sign unsupported file type '$extension': $resolved"
}

$targetRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..\..\..\target\release')).Path
if (-not $resolved.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to sign a file outside the release target: $resolved"
}

& artifact-signing-cli `
    -e $env:JLR_ARTIFACT_SIGNING_ENDPOINT `
    -a $env:JLR_ARTIFACT_SIGNING_ACCOUNT `
    -c $env:JLR_ARTIFACT_SIGNING_PROFILE `
    -d 'ProwlOne' `
    $resolved

if ($LASTEXITCODE -ne 0) {
    throw "Artifact Signing failed for $resolved with exit code $LASTEXITCODE."
}
