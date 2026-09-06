$ErrorActionPreference = 'Stop'

$required = @(
    'AZURE_CLIENT_ID',
    'AZURE_CLIENT_SECRET',
    'AZURE_TENANT_ID',
    'JLR_ARTIFACT_SIGNING_ENDPOINT',
    'JLR_ARTIFACT_SIGNING_ACCOUNT',
    'JLR_ARTIFACT_SIGNING_PROFILE',
    'JLR_EXPECTED_PUBLISHER'
)

$missing = @($required | Where-Object { [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($_)) })
if ($missing.Count -ne 0) {
    throw @"
Signed release configuration is incomplete. No build or unsigned artifact will be published.
Missing GitHub environment secrets/variables: $($missing -join ', ')
See docs/F3_1_WINDOWS_SIGNING.md.
"@
}
