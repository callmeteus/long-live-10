param(
    [ValidateSet("disk", "install")]
    [string]$BootMode = "disk",
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" })
)

$ErrorActionPreference = "Stop"

$qemu = if ($env:LL10_QEMU) { $env:LL10_QEMU } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-system-x86_64.exe" }
$disk = Join-Path $VmDir "vm\ll10-22h2.qcow2"
$iso = Join-Path $VmDir "iso\Win10_22H2_English_x64.iso"
$unattend = Join-Path $VmDir "unattend\autounattend.iso"

if (-not (Test-Path $qemu)) {
    throw "QEMU was not found at $qemu. Install it with 'scoop install qemu' or set LL10_QEMU."
}
if (-not (Test-Path $disk)) {
    throw "VM disk was not found at $disk. Run tools/vm/qemu/Setup-Vm.ps1 first."
}

$bootOrder = if ($BootMode -eq "install") { "d" } else { "c" }

$args = @(
    "-name", "ll10-22h2",
    "-machine", "pc,accel=whpx",
    "-cpu", "host",
    "-smp", "2,sockets=1,cores=2,threads=1",
    "-m", "4096",
    "-rtc", "base=localtime,clock=host",
    "-drive", "file=$disk,format=qcow2,if=ide,index=0"
)

if (Test-Path $iso) {
    $args += @("-drive", "file=$iso,media=cdrom,readonly=on,if=ide,index=2")
}
if (Test-Path $unattend) {
    $args += @("-drive", "file=$unattend,media=cdrom,readonly=on,if=ide,index=3")
}

$args += @(
    "-netdev", "user,id=net0,hostfwd=tcp:127.0.0.1:55985-:5985,hostfwd=tcp:127.0.0.1:53389-:3389",
    "-device", "e1000,netdev=net0",
    "-vga", "std",
    "-display", "none",
    "-vnc", "127.0.0.1:1",
    "-qmp", "tcp:127.0.0.1:4444,server=on,wait=off",
    "-boot", "order=$bootOrder,menu=on",
    "-pidfile", (Join-Path $VmDir "vm\ll10-22h2.pid"),
    "-serial", "file:$(Join-Path $VmDir 'logs\qemu.log')"
)

$process = Start-Process -FilePath $qemu -ArgumentList $args -WindowStyle Hidden -PassThru
Write-Host "Started QEMU (PID $($process.Id))."
Write-Host "QMP: 127.0.0.1:4444  VNC: 127.0.0.1:5901  WinRM: 127.0.0.1:55985  RDP: 127.0.0.1:53389"

if ($BootMode -eq "install") {
    Write-Host "Sending boot keys for 20 seconds..."
    for ($i = 0; $i -lt 20; $i++) {
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
            $command = '{"execute":"human-monitor-command","arguments":{"command-line":"sendkey ret"}}' + "`r`n"
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($command)
            $stream.Write($bytes, 0, $bytes.Length)
            Start-Sleep -Milliseconds 100
            $client.Close()
        } catch {
            # QMP may not be ready yet.
        }
        Start-Sleep -Seconds 1
    }
}
