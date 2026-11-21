# Start menu search dead on typing

## Summary

On Windows 10 22H2, typing in the Start menu sometimes produces no search
results; reopening Start fixes it temporarily. Investigation showed the root
cause is **not a code defect in `SearchApp.exe`**. It is a **degraded AppX
registration** of `Microsoft.Windows.Search`:

- Event `Microsoft-Windows-TWinUI/Operational` 5961:
  `Activation for Microsoft.Windows.Search_cw5n1h2txyewy!CortanaUI failed`,
  `ErrorCode 0x80073D07` (AppX deployment/registration error),
  `PhaseFlags 34` (`COM ActivateExtension`).
- `Get-AppxPackage -Name Microsoft.Windows.Search` returned `Status: Ok` but
  `PackageUserInformation: {}` — the package was installed but not registered
  for the user, so the CortanaUI activation failed.
- Re-registering with `Add-AppxPackage -Register` made the symptom disappear:
  406 consecutive Start/search iterations with 0 failures (before: 11/57,
  11/57, 1/101 failures).

## How the patch works

This package adds two declarative capabilities to the patch engine:

- `Check::AppxPackageRegistration`: detects whether a packaged app is
  registered for the current user.
- `Operation::ReRegisterAppx`: runs the documented `Add-AppxPackage -Register`
  repair with the manifest path declared in this manifest. No script text is
  taken from the manifest; the operation is implemented by the trusted backend.

The patch is fail-closed: `apply` only re-registers when the check says the
package is degraded. Re-applying a healthy package is a no-op.

## Why there is no binary artifact

The repository rule "every patch needs a binary artifact produced by reverse
engineering" applies to **security mitigations**, where the stock binary is
vulnerable and must be changed. Here the stock binary is correct; changing it
actually breaks the AppX registration (we tested two binary patches and both
made the failure worse, producing `0x80073D07`). The correct repair is to
restore the package registration, not to modify the binary. That is why this
package declares no `artifacts` and no `build_variants`: the only operation is
the declarative AppX repair.

## Verification

```powershell
ll10-agent patches verify start-menu-search-repair
ll10-agent patches apply  start-menu-search-repair --confirm
ll10-agent patches verify start-menu-search-repair
```

`verify` should report `Drifted` while the registration is degraded and
`Applied` after the repair.

## Recovery

See `RECOVERY.md`.
