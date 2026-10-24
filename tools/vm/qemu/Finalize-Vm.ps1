param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),
    [string]$Snapshot = "baseline",
    [int]$TimeoutSeconds = 1800
)

$ErrorActionPreference = "Stop"

$qemuImg = if ($env:LL10_QEMU_IMG) { $env:LL10_QEMU_IMG } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-img.exe" }
$disk = Join-Path $VmDir "vm\ll10-22h2.qcow2"

if (-not (Test-Path $qemuImg)) { throw "qemu-img was not found at $qemuImg." }
if (-not (Test-Path $disk)) { throw "VM disk was not found at $disk." }

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$ready = $false
while ((Get-Date) -lt $deadline) {
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
        $command = '{"execute":"screendump","arguments":{"filename":"' + ($VmDir -replace '\\','/') + '/logs/finalize.ppm"}}' + "`r`n"
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($command)
        $stream.Write($bytes, 0, $bytes.Length)
        Start-Sleep -Milliseconds 500
        $client.Close()

        $convert = Join-Path $env:USERPROFILE "scoop\apps\python\current\python.exe"
        if (-not (Test-Path $convert)) { $convert = "python" }
        $check = @"
from PIL import Image
im = Image.open(r'$($VmDir -replace '\\','\\')\logs\finalize.ppm').convert('L')
dark = sum(1 for p in im.getdata() if p < 32)
print('dark' if dark > im.width * im.height * 0.9 else 'ready')
"@
        $check | Set-Content -Path (Join-Path $env:TEMP "finalize-check.py") -Encoding UTF8
        $result = & $convert (Join-Path $env:TEMP "finalize-check.py") 2>$null
        if ($result -eq "ready") {
            $ready = $true
            break
        }
    } catch {
        # QMP or screenshot is not ready yet.
    }
    Start-Sleep -Seconds 5
}

if (-not $ready) {
    throw "Timed out waiting for the VM desktop."
}

& $qemuImg snapshot -d $Snapshot $disk 2>$null
& $qemuImg snapshot -c $Snapshot $disk
Write-Host "Created snapshot '$Snapshot'."
