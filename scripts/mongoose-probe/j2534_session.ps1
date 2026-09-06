# Drive the vendor J2534 library exactly as a diagnostic application would,
# so that USBPcap can record what it really sends to the adapter:
#   PassThruOpen -> PassThruConnect(CAN, 500 kbit/s) -> pass-all filter ->
#   ReadMsgs for two seconds -> Disconnect -> [same for ISO15765] -> Close.
# Must run in 32-bit PowerShell (SysWOW64) because monpj432.dll is 32-bit.
# Nothing here writes to the vehicle: no WriteMsgs is issued.
param(
  [string]$Dll = 'C:\Program Files (x86)\Drew Technologies, Inc\J2534\MongoosePro JLR\monpj432.dll',
  [int]$ReadSeconds = 2
)

if ([IntPtr]::Size -ne 4) { throw "run this in 32-bit PowerShell: C:\Windows\SysWOW64\WindowsPowerShell\v1.0\powershell.exe" }

$code = @"
using System;
using System.Runtime.InteropServices;
using System.Text;

[StructLayout(LayoutKind.Sequential)]
public struct PASSTHRU_MSG {
    public uint ProtocolID;
    public uint RxStatus;
    public uint TxFlags;
    public uint Timestamp;
    public uint DataSize;
    public uint ExtraDataIndex;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 4128)]
    public byte[] Data;
}

public static class J2534 {
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruOpen(IntPtr pName, out uint pDeviceID);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruClose(uint DeviceID);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruConnect(uint DeviceID, uint ProtocolID, uint Flags, uint BaudRate, out uint pChannelID);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruDisconnect(uint ChannelID);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruReadMsgs(uint ChannelID, [In, Out] PASSTHRU_MSG[] pMsg, ref uint pNumMsgs, uint Timeout);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruStartMsgFilter(uint ChannelID, uint FilterType, ref PASSTHRU_MSG pMaskMsg, ref PASSTHRU_MSG pPatternMsg, IntPtr pFlowControlMsg, out uint pFilterID);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruReadVersion(uint DeviceID, StringBuilder pFirmwareVersion, StringBuilder pDllVersion, StringBuilder pApiVersion);
    [DllImport("$($Dll.Replace('\','\\'))", CallingConvention = CallingConvention.StdCall)]
    public static extern int PassThruGetLastError(StringBuilder pErrorDescription);
}
"@
Add-Type -TypeDefinition $code

function LastError { $sb = New-Object System.Text.StringBuilder 256; [void][J2534]::PassThruGetLastError($sb); $sb.ToString() }
function Step($name, $rc) { if ($rc -eq 0) { "OK   $name" } else { "FAIL $name -> rc=$rc ($(LastError))" } }

$CAN = 5; $ISO15765 = 6; $PASS_FILTER = 1

$deviceId = 0
Step "PassThruOpen" ([J2534]::PassThruOpen([IntPtr]::Zero, [ref]$deviceId)); "     deviceId=$deviceId"
$fw = New-Object System.Text.StringBuilder 80; $dll = New-Object System.Text.StringBuilder 80; $api = New-Object System.Text.StringBuilder 80
Step "PassThruReadVersion" ([J2534]::PassThruReadVersion($deviceId, $fw, $dll, $api)); "     firmware=$($fw) dll=$($dll) api=$($api)"

foreach ($proto in @(@{name='CAN'; id=$CAN}, @{name='ISO15765'; id=$ISO15765})) {
  $channel = 0
  Step "PassThruConnect $($proto.name) 500000" ([J2534]::PassThruConnect($deviceId, $proto.id, 0, 500000, [ref]$channel)); "     channel=$channel"
  if ($channel -ne 0) {
    $mask = New-Object PASSTHRU_MSG; $mask.ProtocolID = $proto.id; $mask.DataSize = 4; $mask.Data = New-Object byte[] 4128
    $pattern = New-Object PASSTHRU_MSG; $pattern.ProtocolID = $proto.id; $pattern.DataSize = 4; $pattern.Data = New-Object byte[] 4128
    $filterId = 0
    Step "PassThruStartMsgFilter PASS (mask 0)" ([J2534]::PassThruStartMsgFilter($channel, $PASS_FILTER, [ref]$mask, [ref]$pattern, [IntPtr]::Zero, [ref]$filterId)); "     filterId=$filterId"
    $msgs = New-Object PASSTHRU_MSG[] 8
    for ($i = 0; $i -lt 8; $i++) { $msgs[$i] = New-Object PASSTHRU_MSG; $msgs[$i].Data = New-Object byte[] 4128 }
    $deadline = (Get-Date).AddSeconds($ReadSeconds); $got = 0
    while ((Get-Date) -lt $deadline) {
      $n = [uint32]8
      $rc = [J2534]::PassThruReadMsgs($channel, $msgs, [ref]$n, 200)
      if ($rc -eq 0 -and $n -gt 0) { $got += $n }
    }
    "     messages read in $ReadSeconds s: $got"
    Step "PassThruDisconnect" ([J2534]::PassThruDisconnect($channel))
  }
}
Step "PassThruClose" ([J2534]::PassThruClose($deviceId))
