# Full smoke test for an installed agent in the disposable QEMU VM.
#
# Prerequisites:
# - The VM is running and WinRM is available.
# - ll10-agent.exe and the catalog are installed in the guest.

param(
    [string]$HostName = "127.0.0.1",
    [int]$Port = 55985,
    [string]$UserName = "ll10",
    [string]$Password = "Ll10Test!2026",
    [string]$AgentPath = "C:\ll10\ll10-agent.exe"
)

$secure = ConvertTo-SecureString $Password -AsPlainText -Force
$credential = New-Object System.Management.Automation.PSCredential($UserName, $secure)
$session = New-PSSession -ComputerName $HostName -Port $Port -Credential $credential -Authentication Basic -SessionOption (New-PSSessionOption -SkipCACheck -SkipCNCheck -SkipRevocationCheck)

try {
    Invoke-Command -Session $session -ScriptBlock {
        param($agent)
        & $agent diagnostics
        & $agent plan --help | Out-Null
        New-Item -ItemType Directory -Force C:\ll10-smoke | Out-Null
        "baseline" | Set-Content -Path C:\ll10-smoke\target.txt
        Write-Output "smoke-ready"
    } -ArgumentList $AgentPath
} finally {
    Remove-PSSession $session
}
