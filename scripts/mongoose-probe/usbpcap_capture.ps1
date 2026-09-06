# Elevated USBPcap capture of one root hub for a fixed number of seconds.
# Run through Start-Process -Verb RunAs so that one UAC prompt covers
# start, wait and stop.
#   usbpcap_capture.ps1 -Control \\.\USBPcap1 -Out C:\path\capture.pcap -Seconds 25 [-Devices 3]
param(
  [Parameter(Mandatory = $true)][string]$Control,
  [Parameter(Mandatory = $true)][string]$Out,
  [int]$Seconds = 25,
  [string]$Devices = ""
)
$exe = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$args = @('-d', $Control, '-o', $Out, '-A', '-s', '65535', '-b', '4194304')
if ($Devices -ne '') { $args = @('-d', $Control, '-o', $Out, '--devices', $Devices, '-s', '65535', '-b', '4194304') }
$log = "$Out.log"
"start $(Get-Date -Format o) control=$Control devices='$Devices' seconds=$Seconds" | Out-File $log -Encoding utf8
$p = Start-Process -FilePath $exe -ArgumentList $args -PassThru -WindowStyle Hidden -RedirectStandardOutput "$Out.stdout.txt" -RedirectStandardError "$Out.stderr.txt"
Start-Sleep -Seconds $Seconds
if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
"stop  $(Get-Date -Format o) exited=$($p.HasExited) size=$((Get-Item $Out -ErrorAction SilentlyContinue).Length)" | Out-File $log -Append -Encoding utf8
