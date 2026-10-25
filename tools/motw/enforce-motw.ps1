# Compensating mitigation for CVE-2026-96951 (Windows Shell Mark of the Web
# bypass).
#
# The vulnerability itself is fixed only by the Microsoft cumulative update
# (Windows 10 22H2 build 19045.7807). This script does NOT patch shell32.dll.
# It enforces the Attachment Manager and SmartScreen policies that restore and
# strengthen Mark of the Web handling as a bounded, reversible compensating
# control for unpatched machines.
#
# Modes:
#   Apply   - back up the previous registry state to a journal, then enforce
#             the MotW policies. Requires elevation (HKLM).
#   Verify  - deterministic check that every policy value matches the intended
#             state. Exits non-zero when any value is missing or wrong, so the
#             test fails without the mitigation and passes with it.
#   Revert  - restore the exact pre-Apply registry state from the journal.
#
# Usage:
#   .\enforce-motw.ps1 -Mode Apply
#   .\enforce-motw.ps1 -Mode Verify
#   .\enforce-motw.ps1 -Mode Revert

param(
    [ValidateSet("Apply", "Verify", "Revert")]
    [string]$Mode = "Apply",

    [string]$JournalPath = "$env:ProgramData\LongLive10\motw-journal.json",

    [ValidateSet("Warn", "Block")]
    [string]$SmartScreenLevel = "Warn"
)

$ErrorActionPreference = "Stop"

# Attachment Manager and SmartScreen values that enforce MotW handling.
# SaveZoneInformation=0: always preserve the zone information on files.
# ScanWithAntivirus=3: enable the MotW-driven Attachment Execution Service
# scan so untrusted files are checked before they run.
# EnableSmartScreen=1 with ShellSmartScreenLevel: run SmartScreen checks for
# downloaded programs and files.
$intended = @(
    @{
        Path  = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Attachments"
        Name  = "SaveZoneInformation"
        Type  = "DWord"
        Value = 0
    },

    @{
        Path  = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Attachments"
        Name  = "ScanWithAntivirus"
        Type  = "DWord"
        Value = 3
    },

    @{
        Path  = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\System"
        Name  = "EnableSmartScreen"
        Type  = "DWord"
        Value = 1
    },

    @{
        Path  = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\System"
        Name  = "ShellSmartScreenLevel"
        Type  = "String"
        Value = $SmartScreenLevel
    }
)

function Read-CurrentState {
    param($Entry)

    if (-not (Test-Path -LiteralPath $Entry.Path)) {
        return @{ existed = $false }
    }

    $item = Get-ItemProperty -LiteralPath $Entry.Path -ErrorAction SilentlyContinue
    $raw = $item.PSObject.Properties[$Entry.Name]

    if ($null -eq $raw) {
        return @{ existed = $false }
    }

    return @{
        existed = $true
        type    = switch ($Entry.Type) {
            "DWord"  { "DWord" }
            "String" { "String" }
            default  { "String" }
        }
        value   = $raw.Value
    }
}

function Invoke-Apply {
    if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Apply mode requires elevation because the mitigation writes HKLM policy keys."
    }

    if (Test-Path -LiteralPath $JournalPath) {
        throw "Journal already exists at $JournalPath. Run Revert before applying again."
    }

    $entries = @()

    foreach ($entry in $intended) {
        $current = Read-CurrentState -Entry $entry

        $entries += @{
            path    = $entry.Path
            name    = $entry.Name
            type    = $entry.Type
            existed = [bool]$current.existed
            value   = $current.value
        }
    }

    $journal = @{
        version    = 1
        created_at = (Get-Date).ToUniversalTime().ToString("o")
        entries    = $entries
    }

    $journalDir = Split-Path -Parent $JournalPath
    if ($journalDir) {
        New-Item -ItemType Directory -Force -Path $journalDir | Out-Null
    }

    $journal | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $JournalPath -Encoding UTF8

    foreach ($entry in $intended) {
        if (-not (Test-Path -LiteralPath $entry.Path)) {
            New-Item -Path $entry.Path -Force | Out-Null
        }

        Set-ItemProperty -LiteralPath $entry.Path -Name $entry.Name -Value $entry.Value -Type $entry.Type
        Write-Host "enforced $($entry.Path)\$($entry.Name) = $($entry.Value)"
    }

    Write-Host "MotW policies applied. Journal: $JournalPath"
}

function Invoke-Verify {
    $failures = 0

    foreach ($entry in $intended) {
        $current = Read-CurrentState -Entry $entry
        $actual = if ($current.existed) { $current.value } else { "<missing>" }
        $ok = $current.existed -and ("$($current.value)" -eq "$($entry.Value)")

        if ($ok) {
            Write-Host "PASS $($entry.Path)\$($entry.Name) = $($entry.Value)"
        } else {
            Write-Host "FAIL $($entry.Path)\$($entry.Name) expected $($entry.Value), found $actual"
            $failures++
        }
    }

    if ($failures -gt 0) {
        Write-Host "$failures MotW policy check(s) failed."
        exit 1
    }

    Write-Host "All MotW policy checks passed."
    exit 0
}

function Invoke-Revert {
    if (-not (Test-Path -LiteralPath $JournalPath)) {
        throw "No journal found at $JournalPath. Nothing to revert."
    }

    $journal = Get-Content -LiteralPath $JournalPath -Raw | ConvertFrom-Json

    foreach ($entry in $journal.entries) {
        if (-not (Test-Path -LiteralPath $entry.path)) {
            continue
        }

        if (-not [bool]$entry.existed) {
            Remove-ItemProperty -LiteralPath $entry.path -Name $entry.name -ErrorAction SilentlyContinue
            Write-Host "removed $($entry.path)\$($entry.name)"
        } else {
            Set-ItemProperty -LiteralPath $entry.path -Name $entry.name -Value $entry.value -Type $entry.type
            Write-Host "restored $($entry.path)\$($entry.name) = $($entry.value)"
        }
    }

    Remove-Item -LiteralPath $JournalPath -Force
    Write-Host "MotW policies reverted to the pre-apply state."
}

switch ($Mode) {
    "Apply"  { Invoke-Apply }
    "Verify" { Invoke-Verify }
    "Revert" { Invoke-Revert }
}