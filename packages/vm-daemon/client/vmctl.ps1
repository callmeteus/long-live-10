# vmctl - host-side controller for the LL10 VM daemon.
#
# Talks to the daemon over VirtualBox NAT (host 127.0.0.1 -> guest 55990/55991).
# No WinRM involved. Use this to drive the UI and read results.
#
# Examples (run from PowerShell on the host):
#   .\vmctl.ps1 -Command PING
#   .\vmctl.ps1 -Command WIN
#   .\vmctl.ps1 -Command "TEXT notepad"
#   .\vmctl.ps1 -Command ENTER
#   .\vmctl.ps1 -Command PROCS
#   .\vmctl.ps1 -Command "LONGRUN 200"     # starts a long batch in the daemon
#   .\vmctl.ps1 -GetLog                     # reads repro-long.log over HTTP
#   .\vmctl.ps1 -GetStatus
param(
    [string]$Command,
    [switch]$GetLog,
    [switch]$GetStatus,
    [switch]$GetDaemonLog,
    [string]$HostName = "127.0.0.1",
    [int]$ControlPort = 55990,
    [int]$HttpPort = 55991,
    [int]$TimeoutMs = 8000
)

function Invoke-Control {
    param([string]$Line)
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $iar = $client.ConnectAsync($HostName, $ControlPort)
        if (-not $iar.AsyncWaitHandle.WaitOne($TimeoutMs)) { throw "connect timeout" }
        $stream = $client.GetStream()
        $reader = New-Object System.IO.StreamReader($stream)
        $writer = New-Object System.IO.StreamWriter($stream)
        $writer.AutoFlush = $true
        $writer.WriteLine($Line)
        if (-not $stream.DataAvailable) { Start-Sleep -Milliseconds 200 }
        return $reader.ReadLine()
    } finally { $client.Close() }
}

function Invoke-HttpGet {
    param([string]$Path)
    try {
        $resp = Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "http://$HostName`:$HttpPort/$Path"
        return $resp.Content
    } catch { return "HTTP ERROR: $($_.Exception.Message)" }
}

if ($GetLog) { Write-Output (Invoke-HttpGet "log"); return }
if ($GetStatus) { Write-Output (Invoke-HttpGet "status"); return }
if ($GetDaemonLog) { Write-Output (Invoke-HttpGet "daemonlog"); return }
if (-not $Command) { Write-Output "vmctl: provide -Command, -GetLog, -GetStatus or -GetDaemonLog"; return }

Write-Output (Invoke-Control $Command)
