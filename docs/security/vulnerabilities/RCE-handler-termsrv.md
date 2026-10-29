# RCE handler identification - termsrv.dll (CVE-2026-70187)

## Identified handler

- **File path:** `C:\Windows\System32\termsrv.dll`
- **Build (reference):** 10.0.19041.6456 (host), target Windows 10 22H2 (19045.x)
- **Function:** `FUN_18008e1f8` (Ghidra/REA, x86_64)
- **String anchor:** `"failed to query user authentication (NLA)"` at VA `0x1800db5b0`
- **References (xrefs):** `0x18008e633`, `0x18008e63a`

## Evidence (REA)

- Analysis: `rea analyze .../termsrv.dll --provider ghidra` (Ghidra 12.1.4), 4261 procedures / 5147 strings.
- Search: `rea search ... "NLA"` -> string at `0x1800db5b0`.
- Xrefs: `rea xrefs ... 0x1800db5b0` -> `0x18008e633`, `0x18008e63a`.
- Decompilation: `rea function ... 0x18008e633` -> `FUN_18008e1f8`, which calls `RegWinstationQuerySecurityConfig_Merged` and logs the NLA failure.

## Nature of the handler

`FUN_18008e1f8` is a listener status/logging routine: it enumerates RDP listeners,
queries each one's security configuration (`RegWinstationQuerySecurityConfig_Merged`,
including the `UserAuthentication`/NLA flag), and logs `"failed to query user
authentication (NLA)"` when the query fails. It is the code path that observes
the NLA state, but it is not itself the pre-authentication RCE handler; the
actual RCE surface is the RDP connection handler that consumes a disabled NLA
state before authentication.

## Reproduction steps (VM only, no external systems)

1. Provision the test VM (`ll10-22h2`) from the `baseline` snapshot; ensure RDP
   is enabled (`fDenyTSConnections = 0`) and NLA is disabled
   (`UserAuthentication = 0`) to expose the pre-auth path.
2. Confirm the VM is reachable only from the host (WinRM `127.0.0.1:55985`);
   do not target any external host.
3. Send a crafted pre-authentication RDP packet sequence to the VM's RDP port
   (3389) using a controlled harness (e.g. a modified FreeRDP/rdp crate) that
   exercises the listener handler path identified above.
4. Observe code execution in the VM (e.g. a marker file written under
   `C:\ll10\` or a reverse shell to a listener bound on the host loopback only).
5. Revert the VM to `baseline` after the test.

## Status

A live PoC reproduction has NOT been executed: no exploit payload is available in
the repository and no VM is provisioned in this environment. The handler's file
path and function are identified above via REA. Reproduction requires an actual
RDS exploit payload and a running `ll10-22h2` VM.

## References

- REA evidence: termsrv.dll analysis above.
- Related patch work: docs/security/vulnerabilities (CVE-2026-70187 removed as
  non-compliant; rule in AGENTS.md requires R&A binary patches).
