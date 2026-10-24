# Suppresses Microsoft end-of-support migration prompts on Windows 10.
#
# This is a best-effort, reversible helper for disposable test machines. It does
# not change licensing, activation, or update policy.

$ErrorActionPreference = "Continue"

# Stop the recurring end-of-support notification.
New-Item -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notify" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notify" -Name "DiscontinueEOS" -Value 1 -Type DWord

# Disable the "finish setting up your device" prompts.
New-Item -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement" -Name "ScoobeSystemSettingEnabled" -Value 0 -Type DWord

# Disable consumer content prompts that are commonly used for migration nudges.
$contentPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager"
New-Item -Path $contentPath -Force | Out-Null
foreach ($name in @("SubscribedContent-338388Enabled", "SubscribedContent-338389Enabled", "SubscribedContent-353696Enabled")) {
    Set-ItemProperty -Path $contentPath -Name $name -Value 0 -Type DWord
}

# Disable the scheduled tasks that raise EOS and upgrade prompts when present.
foreach ($pattern in @("EOSNotify*", "GWX*")) {
    Get-ScheduledTask -TaskName $pattern -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Disable-ScheduledTask -TaskName $_.TaskName -TaskPath $_.TaskPath -ErrorAction Stop | Out-Null
            Write-Host "Disabled scheduled task $($_.TaskPath)$($_.TaskName)"
        } catch {
            Write-Warning "Could not disable $($_.TaskPath)$($_.TaskName): $($_.Exception.Message)"
        }
    }
}

Write-Host "End-of-support migration prompts suppressed for the current user."
