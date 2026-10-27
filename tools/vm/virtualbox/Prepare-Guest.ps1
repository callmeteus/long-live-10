<#
.SYNOPSIS
    Provisions a powered-off VirtualBox Windows guest offline (via its VHD).

.DESCRIPTION
    Makes the VM "ready to use" before it is ever booted by an operator:

      * WinRM service starts automatically.
      * WinRM accepts local-account (Negotiate/NTLM) authentication.
      * Windows Firewall does not block the forwarded ports.
      * Automatic logon is enabled for the test account.
      * Leftover first-boot scripts that open console windows are removed.

    Provisioning happens directly on the guest disk, so it needs no RDP session,
    no keyboard automation and no operator interaction. It is idempotent:
    running it twice produces the same state.

    The VM must be powered off. The script shuts it down gracefully first so
    the registry hives are consistent (an unclean shutdown can replay hive
    logs and silently discard offline edits).

    Requires elevation (mounting a VHD and loading registry hives).

.PARAMETER VmName
    VirtualBox VM name. Default: ll10-22h2.

.PARAMETER VmDir
    Directory that holds the VM disk. Default: $env:LL10_VM_DIR or
    %USERPROFILE%\VMs\long-live-10.

.PARAMETER DiskPath
    Explicit path to the guest VHD. When omitted it is read from the VM's
    SATA port 0.

.PARAMETER UserName
    Account used for automatic logon. Default: ll10.

.PARAMETER Password
    Password used for automatic logon. Default: Ll10Test!2026.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tools/vm/virtualbox/Prepare-Guest.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tools/vm/virtualbox/Prepare-Guest.ps1 -DiskPath D:\vm\ll10.vhd
#>
[CmdletBinding()]
param(
    [string]$VmName = "ll10-22h2",

    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),

    [string]$DiskPath = "",

    [string]$UserName = "ll10",

    [string]$Password = "Ll10Test!2026",

    # Internal: set when the script re-launches itself elevated.
    [switch]$Elevated
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

# ---------------------------------------------------------------------------
# Elevation (VHD mount + reg load require administrator)
# ---------------------------------------------------------------------------
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Requesting administrator privileges..."
    $argList = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $PSCommandPath,
        "-VmName", $VmName,
        "-VmDir", $VmDir,
        "-UserName", $UserName,
        "-Password", $Password,
        "-Elevated"
    )
    if ($DiskPath) { $argList += @("-DiskPath", $DiskPath) }

    $proc = Start-Process -FilePath "powershell.exe" -ArgumentList $argList -Verb RunAs -Wait -PassThru
    exit $proc.ExitCode
}

# ---------------------------------------------------------------------------
# Native command helper
#
# VirtualBox and reg.exe write diagnostics to stderr. With
# $ErrorActionPreference = "Stop", PowerShell turns that into a terminating
# NativeCommandError, so every native call goes through this helper and the
# exit code is checked explicitly.
# ---------------------------------------------------------------------------
function Invoke-Native {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @()
    )

    $stdout = [System.IO.Path]::GetTempFileName()
    $stderr = [System.IO.Path]::GetTempFileName()
    try {
        # Start-Process joins ArgumentList with spaces without quoting, so any
        # argument containing whitespace (e.g. "...\Windows NT\CurrentVersion")
        # must be quoted explicitly.
        $commandLine = ($Arguments | ForEach-Object {
            if ($_ -match '\s') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
        }) -join ' '

        $proc = Start-Process -FilePath $FilePath -ArgumentList $commandLine -Wait -PassThru `
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

function Invoke-VBox {
    param([string[]]$Arguments)
    if (-not (Test-Path $vb)) { throw "VirtualBox not found at $vb." }
    return Invoke-Native -FilePath $vb -Arguments $Arguments
}

function Invoke-Reg {
    param([string[]]$Arguments)
    $result = Invoke-Native -FilePath "reg.exe" -Arguments $Arguments
    if ($result.ExitCode -ne 0) {
        throw "reg $($Arguments -join ' ') failed: $($result.Output)"
    }
    return $result.Output
}

# ---------------------------------------------------------------------------
# VM state helpers
# ---------------------------------------------------------------------------
function Get-VmState {
    param([string]$Name)
    $result = Invoke-VBox -Arguments @("showvminfo", $Name, "--machinereadable")
    if ($result.ExitCode -ne 0) { return $null }
    $line = ($result.Output -split "`r?`n") | Where-Object { $_ -match '^VMState=' } | Select-Object -First 1
    if (-not $line) { return "unknown" }
    return ($line -replace '^VMState="', '' -replace '"$', '')
}

function Stop-VmGracefully {
    param([string]$Name, [int]$TimeoutSeconds = 180)

    $state = Get-VmState -Name $Name
    if ($null -eq $state) { return }   # VM is not registered

    switch ($state) {
        { $_ -in @("poweroff", "powered off", "aborted") } { return }
        "saved" {
            Write-Host "Discarding saved state of '$Name'..."
            Invoke-VBox -Arguments @("discardstate", $Name) | Out-Null
            return
        }
    }

    Write-Host "Shutting down '$Name' gracefully (state: $state)..."
    Invoke-VBox -Arguments @("controlvm", $Name, "acpipowerbutton") | Out-Null

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 3
        $now = Get-VmState -Name $Name
        if ($now -in @("poweroff", "powered off")) {
            Write-Host "  Guest powered off cleanly."
            return
        }
    }

    Write-Warning "Guest did not shut down within $TimeoutSeconds s; forcing power off. Registry hive logs may be replayed on next boot."
    Invoke-VBox -Arguments @("controlvm", $Name, "poweroff") | Out-Null
    Start-Sleep -Seconds 3
}

function Get-VhdWindowsVolume {
    param([string]$Path)

    $image = Get-DiskImage -ImagePath $Path -ErrorAction Stop
    if (-not $image.Attached) {
        Mount-DiskImage -ImagePath $Path -ErrorAction Stop | Out-Null
        $image = Get-DiskImage -ImagePath $Path -ErrorAction Stop
    }

    $diskNumber = $image.Number

    for ($i = 0; $i -lt 30; $i++) {
        $volumes = Get-Partition -DiskNumber $diskNumber -ErrorAction SilentlyContinue |
            Get-Volume -ErrorAction SilentlyContinue |
            Where-Object { $_.DriveLetter -and $_.FileSystem -eq "NTFS" }

        # Prefer the volume explicitly labelled "Windows"; otherwise the largest.
        $volume = $volumes | Where-Object { $_.FileSystemLabel -eq "Windows" } | Select-Object -First 1
        if (-not $volume) {
            $volume = $volumes | Sort-Object Size -Descending | Select-Object -First 1
        }
        if ($volume) { return $volume }
        Start-Sleep -Seconds 1
    }
    throw "Mounted '$Path' but no Windows volume with a drive letter was found."
}

# ---------------------------------------------------------------------------
# Resolve the guest disk
# ---------------------------------------------------------------------------
if (-not $DiskPath) {
    $result = Invoke-VBox -Arguments @("showvminfo", $VmName, "--machinereadable")
    if ($result.ExitCode -ne 0) { throw "VM '$VmName' not found. Run yarn vm:setup first." }
    $line = ($result.Output -split "`r?`n") | Where-Object { $_ -match '^"SATA-0-0"=' } | Select-Object -First 1
    if (-not $line) { throw "VM '$VmName' has no disk attached to SATA port 0." }
    $DiskPath = ($line -replace '^"SATA-0-0"="', '' -replace '"$', '')
}

if (-not (Test-Path $DiskPath)) { throw "Guest disk not found: $DiskPath" }

if ([System.IO.Path]::GetExtension($DiskPath).ToLowerInvariant() -ne ".vhd") {
    throw @"
Guest disk is not a VHD: $DiskPath

Offline provisioning mounts the disk on the host, which requires VHD/VHDX.
Recreate the VM with 'yarn vm:setup' (it stores the guest disk as VHD), or
convert it first:

  VBoxManage clonemedium disk "<current>" "<new.vhd>" --format VHD
  VBoxManage storageattach $VmName --storagectl SATA --port 0 --device 0 --type hdd --medium "<new.vhd>"
"@
}

Write-Host "Preparing guest disk: $DiskPath"

# ---------------------------------------------------------------------------
# Stop the VM so hives are consistent
# ---------------------------------------------------------------------------
Stop-VmGracefully -Name $VmName

# ---------------------------------------------------------------------------
# Mount the disk
# ---------------------------------------------------------------------------
$drive = $null
$mounted = $false
$systemLoaded = $false
$softwareLoaded = $false
try {
    $volume = Get-VhdWindowsVolume -Path $DiskPath
    $drive = [string]$volume.DriveLetter
    $mounted = $true
    Write-Host "Mounted as ${drive}: ($($volume.FileSystemType))"

    $windowsDir = "${drive}:\Windows"
    if (-not (Test-Path "$windowsDir\System32\config\SYSTEM")) {
        throw "Mounted volume does not look like a Windows installation (no $windowsDir\System32\config\SYSTEM)."
    }

    $systemHive = "$windowsDir\System32\config\SYSTEM"
    $softwareHive = "$windowsDir\System32\config\SOFTWARE"

    Invoke-Reg @("load", "HKLM\LL10_SYSTEM", $systemHive) | Out-Null
    $systemLoaded = $true
    Invoke-Reg @("load", "HKLM\LL10_SOFTWARE", $softwareHive) | Out-Null
    $softwareLoaded = $true

    try {
        # Active control set is ControlSet001..N - never assume 001.
        $select = Invoke-Reg @("query", "HKLM\LL10_SYSTEM\Select", "/v", "Current")
        $match = [regex]::Match($select, '0x([0-9a-fA-F]+)')
        if (-not $match.Success) { throw "Could not read HKLM\SYSTEM\Select\Current from the offline hive." }
        $controlSet = "ControlSet{0:D3}" -f [Convert]::ToInt32($match.Groups[1].Value, 16)
        Write-Host "Active control set: $controlSet"

        $firewallRoot = "HKLM\LL10_SYSTEM\$controlSet\Services\SharedAccess\Parameters\FirewallPolicy"

        Write-Host "  WinRM service: automatic start"
        Invoke-Reg @("add", "HKLM\LL10_SYSTEM\$controlSet\Services\WinRM", "/v", "Start", "/t", "REG_DWORD", "/d", "2", "/f") | Out-Null

        Write-Host "  Remote UAC filtering: disabled for local accounts"
        Invoke-Reg @("add", "HKLM\LL10_SYSTEM\$controlSet\Control\Lsa", "/v", "LocalAccountTokenFilterPolicy", "/t", "REG_DWORD", "/d", "1", "/f") | Out-Null

        Write-Host "  Firewall: disabled for all profiles"
        foreach ($profile in @("DomainProfile", "StandardProfile", "PublicProfile")) {
            Invoke-Reg @("add", "$firewallRoot\$profile", "/v", "EnableFirewall", "/t", "REG_DWORD", "/d", "0", "/f") | Out-Null
        }

        Write-Host "  Network location prompt: suppressed"
        Invoke-Reg @("add", "HKLM\LL10_SYSTEM\$controlSet\Control\Network\NewNetworkWindowOff", "/t", "REG_DWORD", "/d", "1", "/f") | Out-Null

        $winlogon = "HKLM\LL10_SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon"
        Write-Host "  Auto-logon: $UserName"
        Invoke-Reg @("add", $winlogon, "/v", "AutoAdminLogon", "/t", "REG_SZ", "/d", "1", "/f") | Out-Null
        Invoke-Reg @("add", $winlogon, "/v", "DefaultUserName", "/t", "REG_SZ", "/d", $UserName, "/f") | Out-Null
        Invoke-Reg @("add", $winlogon, "/v", "DefaultDomainName", "/t", "REG_SZ", "/d", ".", "/f") | Out-Null
        Invoke-Reg @("add", $winlogon, "/v", "DefaultPassword", "/t", "REG_SZ", "/d", $Password, "/f") | Out-Null
        Invoke-Reg @("add", $winlogon, "/v", "ForceAutoLogon", "/t", "REG_SZ", "/d", "1", "/f") | Out-Null
        Invoke-Native -FilePath "reg.exe" -Arguments @("delete", $winlogon, "/v", "AutoLogonCount", "/f") | Out-Null

        # -------------------------------------------------------------------
        # Remove first-boot leftovers that flash console windows
        # -------------------------------------------------------------------
        Write-Host "  Removing boot-time scripts that open console windows"
        $startupDirs = @(
            "$windowsDir\ProgramData\Microsoft\Windows\Start Menu\Programs\StartUp",
            "$windowsDir\Users\Default\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup"
        )
        foreach ($dir in $startupDirs) {
            if (Test-Path $dir) {
                Get-ChildItem $dir -File -ErrorAction SilentlyContinue |
                    Where-Object { $_.Name -match 'winrm|ll10|unattend' } |
                    Remove-Item -Force -ErrorAction SilentlyContinue
            }
        }
        foreach ($leftover in @("$windowsDir\winrm-setup.bat", "${drive}:\ConfigureWinRM.ps1", "${drive}:\ConfigureWinRM.bat")) {
            if (Test-Path $leftover) { Remove-Item $leftover -Force -ErrorAction SilentlyContinue }
        }

        Write-Host "  Clearing stale RunOnce entries"
        # RunOnce entries are values (REG_SZ commands), not subkeys.
        $runOnce = Invoke-Native -FilePath "reg.exe" -Arguments @(
            "query", "HKLM\LL10_SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"
        )
        if ($runOnce.ExitCode -eq 0) {
            foreach ($row in ($runOnce.Output -split "`r?`n")) {
                if ($row -match '^\s+(?<name>\S.*?)\s+REG_' -and $row -match 'winrm|unattend|ll10') {
                    $name = $Matches['name']
                    Invoke-Native -FilePath "reg.exe" -Arguments @(
                        "delete", "HKLM\LL10_SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce", "/v", $name, "/f"
                    ) | Out-Null
                }
            }
        }

        # -------------------------------------------------------------------
        # Marker file for diagnostics (readable from the running guest)
        # -------------------------------------------------------------------
        $markerDir = "${drive}:\ll10"
        New-Item -ItemType Directory -Force $markerDir | Out-Null
        @(
            "provisioned_at=$(Get-Date -Format o)"
            "provisioner=Prepare-Guest.ps1"
            "auto_logon_user=$UserName"
            "winrm_start=automatic"
            "firewall=disabled"
        ) | Set-Content -Path (Join-Path $markerDir "provision.txt") -Encoding ASCII

        Write-Host "  Provisioning complete."
    }
    finally {
        if ($softwareLoaded) { Invoke-Native -FilePath "reg.exe" -Arguments @("unload", "HKLM\LL10_SOFTWARE") | Out-Null }
        if ($systemLoaded) { Invoke-Native -FilePath "reg.exe" -Arguments @("unload", "HKLM\LL10_SYSTEM") | Out-Null }
    }
}
finally {
    if ($mounted) {
        for ($i = 0; $i -lt 10; $i++) {
            try {
                Dismount-DiskImage -ImagePath $DiskPath -ErrorAction Stop | Out-Null
                break
            } catch {
                Start-Sleep -Seconds 2
            }
        }
    }
}

Write-Host ""
Write-Host "Guest '$VmName' is provisioned." -ForegroundColor Green
Write-Host "  Disk:        $DiskPath"
Write-Host "  Auto-logon:  $UserName"
Write-Host "  WinRM:       automatic, http://127.0.0.1:55985"
Write-Host "Next: yarn vm:start"
