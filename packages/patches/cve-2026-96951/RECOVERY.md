# RECOVERY GUIDE: If shell32.dll patch breaks the system
#
# The patch creates a backup at: <repo>/.data/backups/shell32.dll.bak
# The patched file is scheduled via MoveFileEx (delayed until reboot).
#
# === OPTION 1: Before reboot (patch scheduled but not applied) ===
# The original shell32.dll is still in place. Just cancel the pending rename:
#   1. Open Registry Editor (regedit)
#   2. Navigate to: HKLM\SYSTEM\CurrentControlSet\Control\Session Manager
#   3. Delete the "PendingFileRenameOperations" value
#   4. Reboot normally - original shell32.dll remains
#
# === OPTION 2: After reboot (patched shell32.dll is active but broken) ===
#
# Method A - Safe Mode:
#   1. Restart PC, hold Shift while clicking Restart (or press F8 during boot)
#   2. Troubleshoot > Advanced options > Startup Settings > Restart
#   3. Press 4 (Enable Safe Mode) or 5 (Safe Mode with Networking)
#   4. In Safe Mode, open PowerShell as Admin:
#      takeown /f C:\Windows\System32\shell32.dll /a
#      icacls C:\Windows\System32\shell32.dll /grant Administrators:F
#      Copy-Item <repo>\.data\backups\shell32.dll.bak C:\Windows\System32\shell32.dll -Force
#   5. Reboot normally
#
# Method B - Windows Recovery Environment (WinRE):
#   1. Restart PC 3 times during boot (force shutdown) to trigger WinRE
#   2. Troubleshoot > Advanced options > Command Prompt
#   3. In WinRE, the Windows drive may be D: or E: (not C:)
#      dir D:\Windows\System32\shell32.dll
#      (find the correct drive letter)
#   4. Replace the file:
#      copy D:\Windows\System32\shell32.dll.bak D:\Windows\System32\shell32.dll
#   5. Type: exit, then Continue to Windows
#
# Method C - Windows Install USB:
#   1. Boot from Windows 10/11 install USB
#   2. Press Shift+F10 for command prompt
#   3. Find Windows drive: dir C:\Windows\System32\shell32.dll
#   4. Replace: copy C:\Windows\System32\shell32.dll.bak C:\Windows\System32\shell32.dll
#   5. Type: exit, then restart without USB
#
# Method D - SFC (if system boots but is unstable):
#   sfc /scannow
#   (This restores shell32.dll from the component store, but may restore
#    the original unpatched version)
#
# === PREVENTION ===
# - Always test in VM first: yarn vm:start + yarn motw:test-cycle
# - Keep the .bak file: C:\Windows\System32\shell32.dll.bak
# - The MoveFileEx approach is reversible via registry (Option 1)
# - Journal is at: C:\ProgramData\LongLive10\cve-2026-96951-journal.json
#
# === VERIFICATION ===
# After recovery, verify:
#   yarn shell32:verify
# Should show: "STATUS: Not patched (original)." with hash 936c33a8...
