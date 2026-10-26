# Build-PrebuiltImage.ps1 (VirtualBox) - Creates a bootable VDI with WinRM pre-configured
# Run this script as Administrator.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File tools/vm/virtualbox/Build-PrebuiltImage.ps1
#
# Output: $env:USERPROFILE\VMs\long-live-10\vm\ll10-22h2-prebuilt.vdi
#
# After running: yarn vm:import -- -BaseImage "$env:USERPROFILE\VMs\long-live-10\vm\ll10-22h2-prebuilt.vdi"

param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),

    [string]$WimPath = "$env:USERPROFILE\VMs\long-live-10\iso\wim\install.wim",

    [string]$ImageIndex = "6",

    [int]$DiskSizeGB = 50
)

$ErrorActionPreference = "Stop"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: Run as Administrator." -ForegroundColor Red
    exit 1
}

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }
if (-not (Test-Path $WimPath)) { throw "WIM not found at $WimPath." }

New-Item -ItemType Directory -Force (Join-Path $VmDir "vm") | Out-Null

$vdiPath = Join-Path $VmDir "vm\ll10-22h2-prebuilt.vhd"
Remove-Item $vdiPath -Force -ErrorAction SilentlyContinue

# ---------------------------------------------------------------------------
# 1. Create VDI
# ---------------------------------------------------------------------------
Write-Host "[1/6] Creating VDI ($DiskSizeGB GB)..."
# Use diskpart to create VHD (non-sparse, mountable)
$vhdScript = @"
create vdisk file="$vdiPath" maximum=$($DiskSizeGB * 1024) type=expandable
exit
"@
$vhdScript | Out-File -FilePath "$env:TEMP\vb-createvhd.txt" -Encoding ASCII
$cmd = "diskpart /s $env:TEMP\vb-createvhd.txt > C:\temp\vb-createvhdout.txt 2>&1"
$cmd | Out-File -FilePath "$env:TEMP\vb-createvhd.cmd" -Encoding ASCII
$proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c","$env:TEMP\vb-createvhd.cmd" -Verb RunAs -Wait -PassThru
if ($proc.ExitCode -ne 0) { throw "diskpart create VHD failed." }
if (-not (Test-Path $vdiPath)) { throw "Failed to create VHD." }

# ---------------------------------------------------------------------------
# 2. Mount VDI on host via diskpart
# ---------------------------------------------------------------------------
Write-Host "[2/6] Mounting VDI on host..."
$mountScript = @"
select vdisk file="$vdiPath"
attach vdisk
create partition primary
format fs=ntfs label="Windows" quick
active
assign letter=W
exit
"@
$mountScript | Out-File -FilePath "$env:TEMP\vb-mount.txt" -Encoding ASCII
$cmd = "diskpart /s $env:TEMP\vb-mount.txt > C:\temp\vb-mountout.txt 2>&1"
$cmd | Out-File -FilePath "$env:TEMP\vb-mount.cmd" -Encoding ASCII
$proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c","$env:TEMP\vb-mount.cmd" -Verb RunAs -Wait -PassThru
if ($proc.ExitCode -ne 0) { throw "diskpart mount failed." }

# ---------------------------------------------------------------------------
# 3. Apply WIM
# ---------------------------------------------------------------------------
Write-Host "[3/6] Applying Windows image (index $ImageIndex)..."
& dism /Apply-Image /ImageFile:"$WimPath" /Index:$ImageIndex /ApplyDir:W:\
if ($LASTEXITCODE -ne 0) { throw "DISM Apply-Image failed." }

# ---------------------------------------------------------------------------
# 4. Boot files + unattend + WinRM offline
# ---------------------------------------------------------------------------
Write-Host "[4/6] Configuring boot and WinRM..."

# Boot files
bcdboot W:\Windows /s W: /f BIOS

# Place unattend.xml
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
                <Password><Value>Ll10Test!2026</Value><PlainText>true</PlainText></Password>
                <Enabled>true</Enabled>
                <LogonCount>1</LogonCount>
                <Username>ll10</Username>
            </AutoLogon>
            <UserAccounts>
                <LocalAccounts>
                    <LocalAccount wcm:action="add">
                        <Password><Value>Ll10Test!2026</Value><PlainText>true</PlainText></Password>
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
                    <CommandLine>powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "netsh advfirewall set allprofiles state off; Enable-PSRemoting -Force -SkipNetworkProfileCheck; Set-Item -Path WSMan:\localhost\Service\Auth\Basic -Value $true; Set-Item -Path WSMan:\localhost\Service\AllowUnencrypted -Value $true; Set-Service WinRM -StartupType Automatic; Start-Service WinRM; 'DONE' | Out-File C:\ll10-done.txt"</CommandLine>
                    <Description>Configure WinRM</Description>
                    <RequiresUserInput>false</RequiresUserInput>
                </SynchronousCommand>
            </FirstLogonCommands>
        </component>
    </settings>
</unattend>
"@
$unattendContent | Set-Content -Path "$pantherDir\unattend.xml" -Encoding UTF8

# Offline registry: WinRM auto-start
$systemHive = "W:\Windows\System32\config\SYSTEM"
reg load HKLM\LL10_SYSTEM $systemHive | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\WinRM" /v Start /t REG_DWORD /d 2 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Control\Lsa" /v LocalAccountTokenFilterPolicy /t REG_DWORD /d 1 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\DomainProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\StandardProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile" /v EnableFirewall /t REG_DWORD /d 0 /f | Out-Null
reg add "HKLM\LL10_SYSTEM\ControlSet001\Control\Network\NewNetworkWindowOff" /t REG_DWORD /d 1 /f | Out-Null
reg unload HKLM\LL10_SYSTEM | Out-Null

Write-Host "  Boot + WinRM configured."

# ---------------------------------------------------------------------------
# 5. Detach VDI
# ---------------------------------------------------------------------------
Write-Host "[5/6] Detaching VDI..."
$detachScript = "select vdisk file=`"$vdiPath`"`ndetach vdisk`nexit"
$detachScript | Out-File -FilePath "$env:TEMP\vb-detach.txt" -Encoding ASCII
$cmd = "diskpart /s $env:TEMP\vb-detach.txt > C:\temp\vb-detachout.txt 2>&1"
$cmd | Out-File -FilePath "$env:TEMP\vb-detach.cmd" -Encoding ASCII
$proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c","$env:TEMP\vb-detach.cmd" -Verb RunAs -Wait -PassThru
if ($proc.ExitCode -ne 0) { throw "diskpart detach failed." }

# ---------------------------------------------------------------------------
# 6. Done
# ---------------------------------------------------------------------------
Write-Host "[6/6] Build complete."
@"
VM: ll10-22h2 (pre-built)
Source: built from install.wim index $ImageIndex
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
"@ | Set-Content -Path (Join-Path $VmDir "vm\credentials.txt") -Encoding UTF8

Write-Host ""
Write-Host "=== BUILD COMPLETE ===" -ForegroundColor Green
Write-Host "Image: $vdiPath"
Write-Host "Import with: yarn vm:import -- -BaseImage `"$vdiPath`""
