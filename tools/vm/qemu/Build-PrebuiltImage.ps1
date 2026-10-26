# Build-PrebuiltImage.ps1
# Creates a bootable Windows 10 22H2 qcow2 image from the install.wim,
# with WinRM pre-configured. Run this script as Administrator.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File Build-PrebuiltImage.ps1
#
# Output: $env:USERPROFILE\VMs\long-live-10\vm\ll10-22h2.qcow2
#
# After running, use: yarn vm:start ; yarn vm:smoke ; yarn motw:test-cycle

param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),

    [string]$WimPath = "$env:USERPROFILE\VMs\long-live-10\iso\wim\install.wim",

    [string]$ImageIndex = "6",  # Windows 10 Pro (index 6 in multi-edition WIM)

    [int]$DiskSizeGB = 50
)

$ErrorActionPreference = "Stop"

# --- Admin check ---
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator." -ForegroundColor Red
    Write-Host "Right-click PowerShell -> Run as Administrator, then run this script." -ForegroundColor Yellow
    exit 1
}

$qemuImg = if ($env:LL10_QEMU_IMG) { $env:LL10_QEMU_IMG } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-img.exe" }
if (-not (Test-Path $qemuImg)) { throw "qemu-img not found at $qemuImg." }
if (-not (Test-Path $WimPath)) { throw "WIM not found at $WimPath." }

New-Item -ItemType Directory -Force (Join-Path $VmDir "vm"), (Join-Path $VmDir "logs") | Out-Null

$vhdPath = Join-Path $VmDir "vm\ll10-22h2.vhd"
$qcow2Path = Join-Path $VmDir "vm\ll10-22h2.qcow2"

# Clean up any previous attempt
Remove-Item $vhdPath -Force -ErrorAction SilentlyContinue
Remove-Item $qcow2Path -Force -ErrorAction SilentlyContinue

# ---------------------------------------------------------------------------
# 1. Create VHD
# ---------------------------------------------------------------------------
Write-Host "[1/7] Creating VHD ($DiskSizeGB GB)..."
$vhdScript = @"
create vdisk file="$vhdPath" maximum=$($DiskSizeGB * 1024) type=expandable
select vdisk file="$vhdPath"
attach vdisk
create partition primary
format fs=ntfs label="Windows" quick
active
assign letter=W
exit
"@
$vhdScript | diskpart | Out-Null
if ($LASTEXITCODE -ne 0) { throw "diskpart failed." }
Write-Host "  VHD created and formatted."

# ---------------------------------------------------------------------------
# 2. Apply WIM to Windows partition
# ---------------------------------------------------------------------------
Write-Host "[2/7] Applying Windows 10 image (index $ImageIndex) to VHD..."
dism /Apply-Image /ImageFile:"$WimPath" /Index:$ImageIndex /ApplyDir:W:\
if ($LASTEXITCODE -ne 0) { throw "DISM Apply-Image failed (exit $LASTEXITCODE)." }
Write-Host "  Image applied."

# ---------------------------------------------------------------------------
# 3. Make bootable
# ---------------------------------------------------------------------------
Write-Host "[3/7] Writing boot files..."
bcdboot W:\Windows /s W: /f BIOS
if ($LASTEXITCODE -ne 0) { throw "bcdboot failed." }
Write-Host "  Boot files written."

# ---------------------------------------------------------------------------
# 4. Configure WinRM offline via registry
# ---------------------------------------------------------------------------
Write-Host "[4/7] Configuring WinRM and firewall offline..."

# Load the SYSTEM hive from the VHD
$systemHive = "W:\Windows\System32\config\SYSTEM"
reg load HKLM\LL10_SYSTEM $systemHive | Out-Null

# Enable WinRM service (auto-start)
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\WinRM" /v Start /t REG_DWORD /d 2 /f | Out-Null

# LocalAccountTokenFilterPolicy (allow remote admin)
reg add "HKLM\LL10_SYSTEM\ControlSet001\Control\Lsa" /v LocalAccountTokenFilterPolicy /t REG_DWORD /d 1 /f | Out-Null

# Disable firewall (all profiles)
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\DomainProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\StandardProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null

# Suppress network discovery dialog
reg add "HKLM\LL10_SYSTEM\ControlSet001\Control\Network\NewNetworkWindowOff" /t REG_DWORD /d 1 /f | Out-Null

# Unload hive
reg unload HKLM\LL10_SYSTEM | Out-Null
Write-Host "  Registry configured."

# ---------------------------------------------------------------------------
# 5. Place unattend.xml for OOBE (WinRM setup on first logon)
# ---------------------------------------------------------------------------
Write-Host "[5/7] Placing unattend.xml for first-logon WinRM setup..."

$pantherDir = "W:\Windows\Panther\Unattend"
New-Item -ItemType Directory -Force $pantherDir | Out-Null

$unattendContent = @"
<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend">
    <settings pass="oobeSystem">
        <component name="Microsoft-Windows-Shell-Setup" processorArchitecture="amd64" publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <OOBE>
                <HideEULAPage>true</HideEULAPage>
                <HideOEMRegistrationScreen>true</HideOEMRegistrationScreen>
                <HideOnlineAccountScreens>true</HideOnlineAccountScreens>
                <HideWirelessSetupInOOBE>true</HideWirelessSetupInOOBE>
                <SkipMachineOOBE>true</SkipMachineOOBE>
                <NetworkLocation>Work</NetworkLocation>
                <ProtectYourPC>3</ProtectYourPC>
            </OOBE>
            <AutoLogon>
                <Password>
                    <Value>Ll10Test!2026</Value>
                    <PlainText>true</PlainText>
                </Password>
                <Enabled>true</Enabled>
                <LogonCount>1</LogonCount>
                <Username>ll10</Username>
            </AutoLogon>
            <UserAccounts>
                <LocalAccounts>
                    <LocalAccount wcm:action="add">
                        <Password>
                            <Value>Ll10Test!2026</Value>
                            <PlainText>true</PlainText>
                        </Password>
                        <Description>Long Live 10 test account</Description>
                        <DisplayName>ll10</DisplayName>
                        <Group>Administrators</Group>
                        <Name>ll10</Name>
                    </LocalAccount>
                </LocalAccounts>
            </UserAccounts>
            <FirstLogonCommands>
                <SynchronousCommand wcm:action="add">
                    <Order>1</Order>
                    <CommandLine>powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "netsh advfirewall set allprofiles state off; Enable-PSRemoting -Force -SkipNetworkProfileCheck; Set-Item -Path WSMan:\localhost\Service\Auth\Basic -Value `$true; Set-Item -Path WSMan:\localhost\Service\AllowUnencrypted -Value `$true; Set-Service WinRM -StartupType Automatic; Start-Service WinRM; 'DONE' | Out-File C:\ll10-done.txt"</CommandLine>
                    <Description>Configure WinRM</Description>
                    <RequiresUserInput>false</RequiresUserInput>
                </SynchronousCommand>
            </FirstLogonCommands>
        </component>
    </settings>
</unattend>
"@
$unattendContent | Set-Content -Path "$pantherDir\unattend.xml" -Encoding UTF8
Write-Host "  Unattend placed at $pantherDir\unattend.xml."

# ---------------------------------------------------------------------------
# 6. Detach VHD
# ---------------------------------------------------------------------------
Write-Host "[6/7] Detaching VHD..."
$detachScript = @"
select vdisk file="$vhdPath"
detach vdisk
exit
"@
$detachScript | diskpart | Out-Null
Write-Host "  VHD detached."

# ---------------------------------------------------------------------------
# 7. Keep as VHD (QEMU supports VHD natively; avoids qcow2 corruption)
# ---------------------------------------------------------------------------
Write-Host "[7/7] Keeping image as VHD (QEMU native format)..."
$finalPath = Join-Path $VmDir "vm\ll10-22h2.vhd"
Move-Item $vhdPath $finalPath -Force

# Clean up any leftover qcow2
Remove-Item $qcow2Path -Force -ErrorAction SilentlyContinue

# Write credentials
@"
VM: ll10-22h2
Source: pre-built image (built from install.wim index $ImageIndex)
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
"@ | Set-Content -Path (Join-Path $VmDir "vm\credentials.txt") -Encoding UTF8

Write-Host ""
Write-Host "=== BUILD COMPLETE ===" -ForegroundColor Green
Write-Host "Image: $qcow2Path"
Write-Host "Next steps:"
Write-Host "  yarn vm:start"
Write-Host "  yarn vm:smoke"
Write-Host "  yarn motw:test-cycle"
