# Full MotW mitigation validation cycle against the guest over WinRM.
#
# Cycle:
#   1. Verify  - must FAIL on the unmodified guest (exit 1)
#   2. Apply   - enforce policies (writes journal in the guest)
#   3. Verify  - must PASS (exit 0)
#   4. Revert  - restore pre-apply state from the journal
#   5. Verify  - must FAIL again (exit 1)
#
# Prints PASS/FAIL per phase and exits non-zero if any phase misbehaves.

param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 55985,
    [string]$UserName = "ll10",
    [string]$Password = "Ll10Test!2026",
    [string]$ScriptPath = "C:\ll10\enforce-motw.ps1"
)

$ErrorActionPreference = "Stop"
$secure = ConvertTo-SecureString $Password -AsPlainText -Force
$credential = New-Object System.Management.Automation.PSCredential($UserName, $secure)
$session = New-PSSession -ComputerName $HostName -Port $Port -Credential $credential -Authentication Negotiate -SessionOption (New-PSSessionOption -SkipCACheck -SkipCNCheck -SkipRevocationCheck)

# Copy enforce-motw.ps1 to the guest before running tests.
$localScript = Join-Path $PSScriptRoot "enforce-motw.ps1"
if (-not (Test-Path $localScript)) {
    throw "Local script not found: $localScript"
}
$scriptContent = Get-Content $localScript -Raw
Invoke-Command -Session $session -ScriptBlock {
    param($content, $path)
    $dir = Split-Path $path -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    Set-Content -Path $path -Value $content -Encoding UTF8
    Write-Output "Script copied to $path"
} -ArgumentList $scriptContent, $ScriptPath

$failures = 0

function Invoke-Phase {
    param([string]$Name, [scriptblock]$Body, [bool]$ExpectSuccess)

    Write-Output "=== $Name ==="
    $ok = $false
    try {
        & $Body
        $ok = $true
    } catch {
        Write-Output "phase-error: $($_.Exception.Message)"
        $ok = $false
    }

    if ($ok -eq $ExpectSuccess) {
        Write-Output "PHASE-PASS: $Name"
    } else {
        Write-Output "PHASE-FAIL: $Name (expected success=$ExpectSuccess, got success=$ok)"
        $script:failures++
    }
}

try {
    # Guest state snapshot before anything.
    Invoke-Command -Session $session -ScriptBlock {
        param($script)
        Write-Output "host: $env:COMPUTERNAME"
        Write-Output "user: $env:USERNAME"
        $os = Get-CimInstance Win32_OperatingSystem
        Write-Output "os: $($os.Caption) $($os.Version)"
        Write-Output "elevated: $(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"
        Write-Output "script-exists: $(Test-Path $script)"
    } -ArgumentList $ScriptPath

    # Phase 1: Verify must FAIL before apply.
    Invoke-Phase "verify-before" {
        Invoke-Command -Session $session -ScriptBlock {
            param($script)
            & powershell -NoProfile -ExecutionPolicy Bypass -File $script -Mode Verify
            if ($LASTEXITCODE -ne 0) { throw "verify exited $LASTEXITCODE (expected failure before apply)" }
        } -ArgumentList $ScriptPath
    } -ExpectSuccess $false

    # Phase 2: Apply must succeed.
    Invoke-Phase "apply" {
        Invoke-Command -Session $session -ScriptBlock {
            param($script)
            & powershell -NoProfile -ExecutionPolicy Bypass -File $script -Mode Apply
            if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) { throw "apply exited $LASTEXITCODE" }
        } -ArgumentList $ScriptPath
    } -ExpectSuccess $true

    # Phase 3: Verify must PASS after apply.
    Invoke-Phase "verify-after" {
        Invoke-Command -Session $session -ScriptBlock {
            param($script)
            & powershell -NoProfile -ExecutionPolicy Bypass -File $script -Mode Verify
            if ($LASTEXITCODE -ne 0) { throw "verify exited $LASTEXITCODE after apply" }
        } -ArgumentList $ScriptPath
    } -ExpectSuccess $true

    # Phase 4: Revert must succeed.
    Invoke-Phase "revert" {
        Invoke-Command -Session $session -ScriptBlock {
            param($script)
            & powershell -NoProfile -ExecutionPolicy Bypass -File $script -Mode Revert
            if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) { throw "revert exited $LASTEXITCODE" }
        } -ArgumentList $ScriptPath
    } -ExpectSuccess $true

    # Phase 5: Verify must FAIL again after revert.
    Invoke-Phase "verify-after-revert" {
        Invoke-Command -Session $session -ScriptBlock {
            param($script)
            & powershell -NoProfile -ExecutionPolicy Bypass -File $script -Mode Verify
            if ($LASTEXITCODE -ne 0) { throw "verify exited $LASTEXITCODE after revert (expected failure)" }
        } -ArgumentList $ScriptPath
    } -ExpectSuccess $false

} finally {
    Remove-PSSession $session
}

Write-Output "=== RESULT ==="
if ($failures -eq 0) {
    Write-Output "MOTW-CYCLE: ALL PHASES PASSED"
    exit 0
} else {
    Write-Output "MOTW-CYCLE: $failures PHASE(S) FAILED"
    exit 1
}