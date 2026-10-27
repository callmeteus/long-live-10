# Smoke test for the disposable VM.
#
# Assumes the VM is running and WinRM is reachable on localhost:55985
# (Start-Vm.ps1 waits for it by default). Retries until the timeout so it can be
# chained directly after 'yarn vm:start'.
param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 55985,
    [string]$UserName = "ll10",
    [string]$Password = "Ll10Test!2026",
    [int]$TimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

$secure = ConvertTo-SecureString $Password -AsPlainText -Force
$credential = New-Object System.Management.Automation.PSCredential($UserName, $secure)
$sessionOption = New-PSSessionOption -SkipCACheck -SkipCNCheck -SkipRevocationCheck

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$session = $null
$lastError = $null

while ((Get-Date) -lt $deadline) {
    try {
        $session = New-PSSession -ComputerName $HostName -Port $Port -Credential $credential `
            -Authentication Negotiate -SessionOption $sessionOption -ErrorAction Stop
        break
    } catch {
        $lastError = $_
        Start-Sleep -Seconds 5
    }
}

if (-not $session) {
    throw "Could not open a WinRM session to ${HostName}:$Port within $TimeoutSeconds s. Last error: $lastError"
}

try {
    $result = Invoke-Command -Session $session -ScriptBlock {
        Write-Output "ll10-smoke"
        (Get-CimInstance Win32_OperatingSystem).Caption
    }
    $result
} finally {
    Remove-PSSession $session
}
