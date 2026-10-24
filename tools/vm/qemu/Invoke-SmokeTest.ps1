# Smoke test for the disposable QEMU VM.
#
# This script assumes the VM is already running and that WinRM is reachable on
# localhost:55985. It runs a harmless command and prints the result.

param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 55985,
    [string]$UserName = "ll10",
    [string]$Password = "Ll10Test!2026"
)

$secure = ConvertTo-SecureString $Password -AsPlainText -Force
$credential = New-Object System.Management.Automation.PSCredential($UserName, $secure)

$session = New-PSSession -ComputerName $HostName -Port $Port -Credential $credential -Authentication Negotiate -SessionOption (New-PSSessionOption -SkipCACheck -SkipCNCheck -SkipRevocationCheck)

try {
    $result = Invoke-Command -Session $session -ScriptBlock {
        Write-Output "ll10-smoke"
        (Get-CimInstance Win32_OperatingSystem).Caption
    }
    $result
} finally {
    Remove-PSSession $session
}
