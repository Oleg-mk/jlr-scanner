<#
.SYNOPSIS
  Install the repository's git hooks into .git/hooks (once per clone).

.DESCRIPTION
  Writes a pre-push hook that runs scripts/pre-push.ps1, which refuses a
  push unless scripts/preflight.ps1 passed on the very commit being pushed.
  Hooks are not versioned by git, so this is run once after cloning:

    powershell -File scripts/install-hooks.ps1
#>
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$hooks = Join-Path $repo ".git/hooks"
New-Item -ItemType Directory -Force $hooks | Out-Null
$hook = @'
#!/bin/sh
# Installed by scripts/install-hooks.ps1: the push waits for the preflight.
exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(git rev-parse --show-toplevel)/scripts/pre-push.ps1"
'@
[System.IO.File]::WriteAllText((Join-Path $hooks "pre-push"), $hook.Replace("`r`n", "`n"))
Write-Host "pre-push hook installed in $hooks"
