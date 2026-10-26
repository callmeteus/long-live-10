# Apply-Shell32Patch.ps1
# Applies a binary patch to shell32.dll using MoveFileEx (delayed until reboot).
# No recovery mode required. Run as Administrator.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File Apply-Shell32Patch.ps1
#
# The patch modifies the Zone.Identifier:$DATA construction in FUN_1802c9b40
# to always skip the vulnerable code path (JA -> JMP short + NOPs).
#
# After running, reboot the system. The patched shell32.dll will be active after reboot.

param(
    [string]$PatchedDll = (Join-Path $PSScriptRoot "shell32-patched.dll"),

    [switch]$Verify
)

$ErrorActionPreference = "Stop"

# --- Admin check ---
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: Run as Administrator." -ForegroundColor Red
    exit 1
}

# --- Verify current shell32.dll ---
$systemShell32 = "C:\Windows\System32\shell32.dll"
$expectedHash = "936c33a8283196a070aa6f844a65a29a8f99f760eae0016754c1cbf17d62bdb7"
$currentHash = (Get-FileHash $systemShell32).Hash.ToLower()

if ($currentHash -ne $expectedHash) {
    Write-Host "WARNING: Current shell32.dll hash ($currentHash) does not match expected ($expectedHash)." -ForegroundColor Yellow
    Write-Host "The system may already be patched or running a different build." -ForegroundColor Yellow
    if (-not $Verify) {
        $confirm = Read-Host "Continue anyway? (y/N)"
        if ($confirm -ne "y") { exit 1 }
    }
}

if ($Verify) {
    $patchedHash = (Get-FileHash $PatchedDll).Hash.ToLower()
    Write-Host "Current shell32.dll:  $currentHash"
    Write-Output "Patched shell32.dll:  $patchedHash"
    Write-Output "Expected pre-fix:    $expectedHash"
    if ($currentHash -eq $patchedHash) {
        Write-Host "STATUS: Already patched." -ForegroundColor Green
    } elseif ($currentHash -eq $expectedHash) {
        Write-Host "STATUS: Not patched (original)." -ForegroundColor Yellow
    } else {
        Write-Host "STATUS: Unknown version." -ForegroundColor Red
    }
    exit 0
}

# --- Verify patched file exists ---
if (-not (Test-Path $PatchedDll)) {
    throw "Patched DLL not found: $PatchedDll"
}

$patchedHash = (Get-FileHash $PatchedDll).Hash.ToLower()
$expectedPatchedHash = "66049f127566ef448326dbcb52ffb51c25a057637a9c3cb4930b0cd2af9eed09"
if ($patchedHash -ne $expectedPatchedHash) {
    throw "Patched DLL hash mismatch. Expected: $expectedPatchedHash, Got: $patchedHash"
}

# --- Take ownership and grant permissions ---
Write-Host "Taking ownership of shell32.dll..."
takeown /f $systemShell32 /a 2>$null | Out-Null
icacls $systemShell32 /grant Administrators:F 2>$null | Out-Null

# --- Create backup ---
$backupPath = "C:\Windows\System32\shell32.dll.bak"
if (-not (Test-Path $backupPath)) {
    Copy-Item $systemShell32 $backupPath -Force
    Write-Host "Backup created: $backupPath"
}

# --- Schedule replacement on reboot using MoveFileEx ---
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class MoveFileEx {
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern bool MoveFileEx(string lpExistingFileName, string lpNewFileName, uint dwFlags);
    public const uint MOVEFILE_DELAY_UNTIL_REBOOT = 0x4;
    public const uint MOVEFILE_REPLACE_EXISTING = 0x1;
}
"@

# Copy patched DLL to a temp location (can't replace in-use file directly)
$tempPatched = "C:\Windows\Temp\shell32-patched.dll"
Copy-Item $PatchedDll $tempPatched -Force

# Schedule: replace shell32.dll with patched version on reboot
$flags = [MoveFileEx]::MOVEFILE_DELAY_UNTIL_REBOOT -bor [MoveFileEx]::MOVEFILE_REPLACE_EXISTING
$result = [MoveFileEx]::MoveFileEx($tempPatched, $systemShell32, $flags)
if (-not $result) {
    $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
    throw "MoveFileEx failed. Error code: $err"
}

Write-Host ""
Write-Host "=== PATCH SCHEDULED ===" -ForegroundColor Green
Write-Host "The patched shell32.dll will replace the original on next reboot."
Write-Host "Reboot now to apply: Restart-Computer"
Write-Host ""
Write-Host "RECOVERY (if something breaks):" -ForegroundColor Yellow
Write-Host "  1. Before reboot: delete PendingFileRenameOperations from registry"
Write-Host "  2. After reboot: Safe Mode -> copy shell32.dll.bak over shell32.dll"
Write-Host "  3. See tools/shell32-patch/RECOVERY.md for full guide"
Write-Host "  Backup: C:\Windows\System32\shell32.dll.bak"
Write-Host ""
Write-Host "To verify after reboot: yarn shell32:verify"
Write-Host "To revert: restore from $backupPath"