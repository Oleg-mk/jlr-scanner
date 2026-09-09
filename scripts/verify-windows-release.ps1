$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($env:JLR_EXPECTED_PUBLISHER)) {
    throw 'JLR_EXPECTED_PUBLISHER is required for release verification.'
}

$releaseRoot = (Resolve-Path -LiteralPath 'target\release').Path
$mainExecutable = Join-Path $releaseRoot 'prowlone-shell.exe'
$bundleRoot = Join-Path $releaseRoot 'bundle'

if (-not (Test-Path -LiteralPath $mainExecutable -PathType Leaf)) {
    throw "Main application executable is missing: $mainExecutable"
}
if (-not (Test-Path -LiteralPath $bundleRoot -PathType Container)) {
    throw "Tauri bundle output is missing: $bundleRoot"
}

$shippedExecutableCode = @(
    Get-Item -LiteralPath $mainExecutable
    Get-ChildItem -LiteralPath $bundleRoot -Recurse -File |
        Where-Object { $_.Extension.ToLowerInvariant() -in @('.exe', '.dll', '.msi') }
)

if ($shippedExecutableCode.Count -lt 2) {
    throw 'Expected the signed application executable and at least one signed installer.'
}

$signTool = Get-ChildItem -LiteralPath 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter signtool.exe -Recurse -File |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $signTool) {
    throw 'Windows SDK x64 signtool.exe was not found.'
}

$manifest = foreach ($file in $shippedExecutableCode) {
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ($signature.Status -ne 'Valid') {
        throw "Authenticode status for $($file.FullName) is $($signature.Status), not Valid."
    }
    if (-not $signature.SignerCertificate) {
        throw "No signer certificate is present for $($file.FullName)."
    }
    if (-not $signature.TimeStamperCertificate) {
        throw "No RFC 3161 timestamp certificate is present for $($file.FullName)."
    }
    if (-not $signature.SignerCertificate.Subject.Equals($env:JLR_EXPECTED_PUBLISHER, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Unexpected publisher for $($file.FullName). Expected '$env:JLR_EXPECTED_PUBLISHER'; actual '$($signature.SignerCertificate.Subject)'."
    }

    & $signTool.FullName verify /pa /all /v /tw $file.FullName
    if ($LASTEXITCODE -ne 0) {
        throw "SignTool verification failed for $($file.FullName) with exit code $LASTEXITCODE."
    }

    [pscustomobject]@{
        File = $file.FullName.Substring($releaseRoot.Length).TrimStart('\')
        SHA256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
        Authenticode = $signature.Status.ToString()
        Publisher = $signature.SignerCertificate.Subject
        CertificateThumbprint = $signature.SignerCertificate.Thumbprint
        TimestampSubject = $signature.TimeStamperCertificate.Subject
    }
}

$destination = Join-Path (Resolve-Path -LiteralPath '.').Path 'dist\windows-signed'
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath $mainExecutable -Destination $destination

$installers = @($shippedExecutableCode | Where-Object { $_.FullName -ne $mainExecutable })
foreach ($installer in $installers) {
    Copy-Item -LiteralPath $installer.FullName -Destination $destination
}

$manifest | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $destination 'signature-manifest.json') -Encoding utf8
