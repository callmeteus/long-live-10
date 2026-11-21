# Recovery: start-menu-search-repair

The repair does not modify any file, so there is nothing to restore from a
backup. It re-registers an AppX package from its signed manifest.

If the Start-menu search regresses after this patch:

1. Check the event log for a fresh TWinUI 5961 error:

   ```powershell
   Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-TWinUI/Operational'; Id=5961} |
     Select-Object -First 5 TimeCreated, Message
   ```

2. Check the package registration state:

   ```powershell
   Get-AppxPackage -Name Microsoft.Windows.Search |
     Select-Object Name, Status, PackageUserInformation
   ```

   `PackageUserInformation: {}` means the registration is degraded.

3. Re-register manually (this is exactly what the patch does):

   ```powershell
   Add-AppxPackage -Path C:\Windows\SystemApps\Microsoft.Windows.Search_cw5n1h2txyewy\AppxManifest.xml -DisableDevelopmentMode -Register
   ```

4. Re-verify with the agent:

   ```powershell
   ll10-agent patches verify start-menu-search-repair
   ```

No reboot is required. If the error code in the event is `0x80070057`
(`E_INVALIDARG`) instead of `0x80073D07`, see
`docs/patches/start-menu-search-investigation.md` for the binary-level
investigation; that is a different failure mode.
