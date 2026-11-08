<#
.SYNOPSIS
  Refuse a push of a commit the preflight has not passed on.

.DESCRIPTION
  scripts/preflight.ps1 writes .git/prowlone-preflight-ok with the commit it
  passed on — and only when every step passed, the shell crate included.
  This hook, installed as .git/hooks/pre-push by scripts/install-hooks.ps1,
  compares that mark with HEAD and stops the push when it is missing, stale
  or made with -SkipShell. On 2026-11-08 a release was pushed while Docker
  was down and the Rust steps had FAILED, because the push was chained to
  the preflight by hand and the hand did not look at the result; CI caught
  nothing wrong that time, and this hook is so that it never has to.

  Bypass, for a push that is not code — a tag of a commit already checked,
  for instance — with: git push --no-verify
#>
$ErrorActionPreference = "Stop"
$repo = (git rev-parse --show-toplevel).Trim()
$mark = Join-Path $repo ".git/prowlone-preflight-ok"
$head = (git rev-parse HEAD).Trim()

if (-not (Test-Path $mark)) {
  Write-Host "pre-push: no preflight mark. Run: powershell -File scripts/preflight.ps1" -ForegroundColor Red
  exit 1
}
$marked = (Get-Content $mark -TotalCount 1).Trim()
if ($marked -ne $head) {
  Write-Host "pre-push: the preflight passed on $($marked.Substring(0, 7)), HEAD is $($head.Substring(0, 7)). Run it again." -ForegroundColor Red
  exit 1
}
Write-Host "pre-push: preflight passed on $($head.Substring(0, 7))." -ForegroundColor Green
exit 0
