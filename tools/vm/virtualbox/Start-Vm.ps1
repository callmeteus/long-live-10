# Start-Vm.ps1 (VirtualBox) - Starts the VM and waits until it is actually usable.
#
# Design notes (why this looks the way it does):
#
#   * Headless by default. Starting a GUI window steals focus from the operator
#     and is never needed for automated work. Pass -Gui only when a human wants
#     to watch the console.
#   * The guest is usable only when a real WinRM login succeeds. A TCP connect
#     is meaningless here: VirtualBox NAT accepts the connection while the guest
#     is still booting, which produced false "ready" results in the past.
#   * Cold boot on this image takes several minutes (first logon, Defender,
#     OneDrive first-run). The default timeout is therefore generous and can be
#     raised with -TimeoutSeconds or LL10_VM_TIMEOUT.
#   * Booting disk-first removes the floppy/DVD probe delay and makes boot
#     ordering deterministic across machines.
#   * The script is idempotent: run it on a VM that is already up and it returns
#     as soon as the login check passes.

[CmdletBinding()]
param(
    [string]$VmName = "ll10-22h2",

    # Show the VirtualBox console window. Off by default.
    [switch]$Gui,

    # How long to wait for a successful WinRM login after power-on.
    [int]$TimeoutSeconds = $(if ($env:LL10_VM_TIMEOUT) { [int]$env:LL10_VM_TIMEOUT } else { 900 }),

    [int]$WinRmPort = 55985,

    [string]$UserName = "ll10",

    [string]$Password = "Ll10Test!2026",

    # Return immediately after power-on instead of waiting for WinRM.
    # (A [bool] parameter cannot be passed from `powershell -File`, so this is
    # a switch: -NoWait.)
    [switch]$NoWait
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found at $vb." }

# ---------------------------------------------------------------------------
# Native helper: VBoxManage writes to stderr even on success, which would
# terminate the script under $ErrorActionPreference = "Stop".
# ---------------------------------------------------------------------------
function Invoke-VBox {
    param([string[]]$Arguments)

    $stdout = [System.IO.Path]::GetTempFileName()
    $stderr = [System.IO.Path]::GetTempFileName()
    try {
        $commandLine = ($Arguments | ForEach-Object {
            if ($_ -match '\s') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
        }) -join ' '

        $proc = Start-Process -FilePath $vb -ArgumentList $commandLine -Wait -PassThru `
            -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr

        $text = (Get-Content $stdout -Raw -ErrorAction SilentlyContinue) +
                (Get-Content $stderr -Raw -ErrorAction SilentlyContinue)
        return [pscustomobject]@{
            ExitCode = $proc.ExitCode
            Output   = ($text | Out-String).Trim()
        }
    }
    finally {
        Remove-Item $stdout, $stderr -Force -ErrorAction SilentlyContinue
    }
}

function Get-VmState {
    # 'showvminfo' can fail transiently while VBoxSVC holds the machine lock
    # during a power transition; retry instead of reporting the VM as missing.
    for ($attempt = 0; $attempt -lt 4; $attempt++) {
        $result = Invoke-VBox -Arguments @("showvminfo", $VmName, "--machinereadable")
        if ($result.ExitCode -eq 0) {
            $line = ($result.Output -split "`r?`n") | Where-Object { $_ -match '^VMState=' } | Select-Object -First 1
            if ($line) { return ($line -replace '^VMState="', '' -replace '"$', '') }
        }
        Start-Sleep -Milliseconds 750
    }
    return $null
}

$credential = New-Object System.Management.Automation.PSCredential(
    $UserName,
    (ConvertTo-SecureString $Password -AsPlainText -Force)
)

function Test-WinRmHandshake {
    # Cheap check: is the WS-Management listener answering at all?
    try {
        $null = Test-WSMan -ComputerName "127.0.0.1" -Port $WinRmPort -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

function Test-WinRmLogin {
    # Authoritative check: can we actually authenticate and run a command?
    try {
        $session = New-PSSession -ComputerName "127.0.0.1" -Port $WinRmPort `
            -Credential $credential -Authentication Negotiate -ErrorAction Stop
        try {
            $null = Invoke-Command -Session $session -ScriptBlock { $env:COMPUTERNAME } -ErrorAction Stop
        }
        finally {
            Remove-PSSession $session -ErrorAction SilentlyContinue
        }
        return $true
    } catch {
        return $false
    }
}

# ---------------------------------------------------------------------------
# Ensure the VM exists
# ---------------------------------------------------------------------------
if ($null -eq (Get-VmState)) {
    throw "VM '$VmName' not found. Run 'yarn vm:setup' first."
}

# ---------------------------------------------------------------------------
# Boot ordering: disk first. Avoids floppy/DVD probing and is reproducible.
# ---------------------------------------------------------------------------
Invoke-VBox -Arguments @(
    "modifyvm", $VmName,
    "--boot1", "disk", "--boot2", "dvd", "--boot3", "none", "--boot4", "none"
) | Out-Null

# ---------------------------------------------------------------------------
# Power on when needed
# ---------------------------------------------------------------------------
$state = Get-VmState
$startedAt = $null

if ($state -in @("running")) {
    Write-Host "VM '$VmName' is already running."
} else {
    if ($state -in @("saved")) {
        Write-Host "Discarding saved state of '$VmName'..."
        Invoke-VBox -Arguments @("discardstate", $VmName) | Out-Null
    }

    Write-Host "Starting VM '$VmName' ($(if ($Gui) { 'gui' } else { 'headless' }))..."
    $startedAt = Get-Date

    $startArgs = @("startvm", $VmName)
    if (-not $Gui) { $startArgs += @("--type", "headless") }

    $result = Invoke-VBox -Arguments $startArgs
    if ($result.ExitCode -ne 0) {
        throw "Failed to start VM '$VmName': $($result.Output)"
    }
}

Write-Host "WinRM: http://127.0.0.1:$WinRmPort"
Write-Host "RDP:   http://127.0.0.1:53389 (not started by this script)"

if ($NoWait) { return }

# ---------------------------------------------------------------------------
# Wait for a real login
# ---------------------------------------------------------------------------
Write-Host "Waiting for WinRM login (up to $TimeoutSeconds s)..."
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$handshake = $false

while ((Get-Date) -lt $deadline) {
    if (-not $handshake) {
        $handshake = Test-WinRmHandshake
        if (-not $handshake) {
            Start-Sleep -Seconds 5
            continue
        }
        Write-Host "  WinRM listener is up; verifying login..."
    }

    if (Test-WinRmLogin) {
        $elapsed = if ($startedAt) { [int]((Get-Date) - $startedAt).TotalSeconds } else { 0 }
        if ($elapsed -gt 0) {
            Write-Host "WinRM login OK after ${elapsed}s." -ForegroundColor Green
        } else {
            Write-Host "WinRM login OK." -ForegroundColor Green
        }
        return
    }

    Start-Sleep -Seconds 5
}

$elapsed = if ($startedAt) { [int]((Get-Date) - $startedAt).TotalSeconds } else { 0 }
$state = Get-VmState
throw @"
WinRM login did not succeed within ${TimeoutSeconds}s (VM state: $state, waited: ${elapsed}s).

What to check, in order:
  1. The guest may still be booting. Retry with a longer budget:
       powershell -ExecutionPolicy Bypass -File tools/vm/virtualbox/Start-Vm.ps1 -TimeoutSeconds 1200
  2. The guest disk may not be provisioned. Re-apply the known-good offline config:
       yarn vm:prepare
  3. Confirm the forward exists:
       VBoxManage showvminfo $VmName | findstr 55985
"@
