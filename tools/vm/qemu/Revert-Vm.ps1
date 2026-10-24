param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),
    [string]$Snapshot = "baseline",
    [switch]$StartAfter
)

$ErrorActionPreference = "Stop"

$qemuImg = if ($env:LL10_QEMU_IMG) { $env:LL10_QEMU_IMG } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-img.exe" }
$disk = Join-Path $VmDir "vm\ll10-22h2.qcow2"

if (-not (Test-Path $qemuImg)) {
    throw "qemu-img was not found at $qemuImg."
}
if (-not (Test-Path $disk)) {
    throw "VM disk was not found at $disk."
}

# Ask QEMU to stop if it is running.
try {
    $client = New-Object System.Net.Sockets.TcpClient
    $client.Connect("127.0.0.1", 4444)
    $stream = $client.GetStream()
    $buffer = New-Object byte[] 4096
    $stream.Read($buffer, 0, $buffer.Length) | Out-Null
    $command = '{"execute":"qmp_capabilities"}' + "`r`n"
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($command)
    $stream.Write($bytes, 0, $bytes.Length)
    Start-Sleep -Milliseconds 100
    $command = '{"execute":"system_powerdown"}' + "`r`n"
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($command)
    $stream.Write($bytes, 0, $bytes.Length)
    Start-Sleep -Seconds 2
    $client.Close()
} catch {
    # The VM is probably already stopped.
}

Start-Sleep -Seconds 5

& $qemuImg snapshot -a $Snapshot $disk
Write-Host "Restored snapshot '$Snapshot'."

if ($StartAfter) {
    & (Join-Path $PSScriptRoot "Start-Vm.ps1") -VmDir $VmDir
}
