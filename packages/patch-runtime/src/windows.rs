#[cfg(windows)]
use anyhow::Result;
#[cfg(windows)]
use patch_core::PatchManifest;
#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
/// Identifier shared by pending.xml and the COMPONENTS registry value.
const PENDING_XML_IDENTIFIER: &str = "916ae75edb30da0146730000dc1be027";

pub struct WindowsBackend {
    pub journal_dir: PathBuf,
}

#[cfg(windows)]
impl WindowsBackend {
    pub fn new() -> Self {
        Self {
            journal_dir: std::path::PathBuf::from(r"C:\ProgramData\LongLive10\patches"),
        }
    }
}

#[cfg(windows)]
impl super::SystemBackend for WindowsBackend {
    fn system_state(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        // Read build from registry
        if let Ok(hklm) = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        {
            if let Ok(build) = hklm.get_value::<String, _>("CurrentBuildNumber") {
                m.insert("build".to_string(), build);
            }
            if let Ok(ubr) = hklm.get_value::<u32, _>("UBR") {
                m.insert("ubr".to_string(), ubr.to_string());
            }
        }
        // Architecture
        m.insert("architecture".to_string(), "x86_64".to_string());
        m
    }

    fn is_admin(&self) -> bool {
        // Check if running as admin via token
        use std::process::Command;
        Command::new("net").arg("session").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn file_hash(&self, path: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn backup_file(&self, path: &str) -> Result<String> {
        let backup = format!("{}.ll10.bak", path);
        // Try normal copy first
        if std::fs::copy(path, &backup).is_ok() {
            return Ok(backup);
        }
        // WRP-protected: take ownership, grant access, copy
        let _ = std::process::Command::new("takeown")
            .args(["/f", path, "/a"])
            .output();
        let _ = std::process::Command::new("icacls")
            .args([path, "/grant", "administrators:F"])
            .output();
        std::fs::copy(path, &backup)
            .map_err(|e| anyhow::anyhow!("backup failed: {}", e))?;
        Ok(backup)
    }

    fn stage_file(&self, source: &Path, dest: &str) -> Result<()> {
        // Expand %VAR% patterns
        let expanded = expand_env(dest);
        if let Some(parent) = Path::new(&expanded).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, &expanded)?;
        Ok(())
    }

    fn replace_file(&self, source: &str, dest: &str) -> Result<()> {
        let source = expand_env(source);
        let dest = expand_env(dest);

        // Fast path: the file is not in use and the ACL already allows us.
        if std::fs::copy(&source, &dest).is_ok() {
            return Ok(());
        }

        // Windows prevents overwriting a mapped image, but it allows renaming it
        // and then creating a new file with the original name. This is the
        // documented procedure for updating a DLL that is in use, and it is what
        // makes replacing shell32.dll possible without a reboot-time hack.
        replace_in_use(&source, &dest)
    }

    fn restore_file(&self, backup: &str, dest: &str) -> Result<()> {
        let backup = expand_env(backup);
        let dest = expand_env(dest);

        if std::fs::copy(&backup, &dest).is_ok() {
            return Ok(());
        }

        replace_in_use(&backup, &dest)
    }

    fn schedule_rename(&self, source: &str, dest: &str) -> Result<()> {
        // Kept for manifests that still ask for a reboot-time replacement. The
        // in-use rename installs the new file immediately, so there is nothing
        // to defer.
        self.replace_file(source, dest)
    }

    fn cancel_rename(&self, _dest: &str, _source: &str) -> Result<()> {
        // Remove from PendingFileRenameOperations
        // This is complex; for now, just log
        Ok(())
    }

    fn registry_get(&self, hive: &str, key: &str, name: &str) -> Result<String> {
        let hkey = match hive {
            "HKLM" => winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE),
            "HKCU" => winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER),
            _ => anyhow::bail!("unsupported hive: {}", hive),
        };
        let subkey = hkey.open_subkey(key)?;
        // Try reading as DWORD first, then as String
        if let Ok(val) = subkey.get_value::<u32, _>(name) {
            return Ok(val.to_string());
        }
        if let Ok(val) = subkey.get_value::<String, _>(name) {
            return Ok(val);
        }
        anyhow::bail!("could not read value: {}", name)
    }

    fn registry_set(&self, hive: &str, key: &str, name: &str, reg_type: &str, data: &serde_json::Value) -> Result<()> {
        let hkey = match hive {
            "HKLM" => winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE),
            "HKCU" => winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER),
            _ => anyhow::bail!("unsupported hive: {}", hive),
        };
        let (subkey, _) = hkey.create_subkey(key)?;
        match reg_type {
            "dword" => {
                let val = data.as_u64().unwrap_or(0) as u32;
                subkey.set_value(name, &val)?;
            }
            "string" => {
                let val = data.as_str().unwrap_or("");
                subkey.set_value(name, &val)?;
            }
            _ => anyhow::bail!("unsupported reg type: {}", reg_type),
        }
        Ok(())
    }

    fn registry_delete(&self, hive: &str, key: &str, name: &str) -> Result<()> {
        let hkey = match hive {
            "HKLM" => winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE),
            "HKCU" => winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER),
            _ => anyhow::bail!("unsupported hive: {}", hive),
        };
        if let Ok(subkey) = hkey.open_subkey_with_flags(key, winreg::enums::KEY_SET_VALUE) {
            let _ = subkey.delete_value(name);
        }
        Ok(())
    }

    fn journal_path(&self, patch_id: &str) -> PathBuf {
        self.journal_dir.join(patch_id).join("journal.json")
    }

    fn arm_auto_remediate(
        &self,
        patch_id: &str,
        manifest: &PatchManifest,
        config: &patch_core::AutoRemediate,
        patches_dir: &std::path::Path,
    ) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        // The subscription watches one file, so a manifest without a watch path
        // cannot be armed. Fall back to the variant target when it is omitted.
        let watch = config
            .watch_paths
            .first()
            .map(String::as_str)
            .or_else(|| manifest.variant_target_path())
            .ok_or_else(|| anyhow::anyhow!("auto_remediate needs watch_paths or a variant target"))?;

        let exe = std::env::current_exe()?.display().to_string();
        let log = r"C:\ProgramData\LongLive10\remediate.log";
        if let Some(parent) = std::path::Path::new(log).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // CommandLineEventConsumer is not a shell, so the redirection and the
        // quoting both need cmd.exe.
        let command = format!(
            "cmd.exe /c \"\"{exe}\" patches remediate {patch_id} --patches-dir \"{}\" --json >> \"{log}\" 2>&1\"",
            patches_dir.display()
        );

        let script = build_wmi_arm_script(patch_id, watch, &command, config.debounce_seconds.max(1));
        run_powershell(&script)
    }

    fn disarm_auto_remediate(&self, patch_id: &str) -> Result<()> {
        let script = build_wmi_disarm_script(patch_id);
        run_powershell(&script)
    }

}

#[cfg(windows)]
fn expand_env(s: &str) -> String {
    // Expand %VAR% patterns using Windows environment
    let mut result = s.to_string();
    // Simple expansion for common vars
    if let Ok(temp) = std::env::var("TEMP") {
        result = result.replace("%TEMP%", &temp);
    }
    if let Ok(tmp) = std::env::var("TMP") {
        result = result.replace("%TMP%", &tmp);
    }
    if let Ok(systemroot) = std::env::var("SYSTEMROOT") {
        result = result.replace("%SYSTEMROOT%", &systemroot);
        result = result.replace("%WINDIR%", &systemroot);
    }
    result
}

// ---------------------------------------------------------------------------
// In-use file replacement
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn take_ownership(path: &str) {
    // WRP is enforced with an ACL owned by TrustedInstaller, so ownership has to
    // move first. It moves to the current principal, not to Administrators
    // (/a): an owner always has implicit WRITE_DAC, which is what lets the
    // grants below apply when the caller is LocalSystem, a non-member of that
    // group. The group is named by SID because "Administrators" is localized
    // ("Administradores" on pt-BR, "Administratoren" on de-DE).
    let _ = std::process::Command::new("takeown")
        .args(["/f", path])
        .output();
    // The WMI consumer runs as LocalSystem, which is not a member of the
    // Administrators group. Without the SYSTEM grant, auto-remediation gets
    // access denied where an interactive admin apply succeeds.
    let _ = std::process::Command::new("icacls")
        .args([path, "/grant", "*S-1-5-32-544:F", "*S-1-5-18:F"])
        .output();
}

#[cfg(windows)]
fn schedule_delete_at_reboot(path: &str) {
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
    }
    const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 0x0000_0004;

    let wide = to_wide(path);
    // A null destination with DELAY_UNTIL_REBOOT means "delete at next boot".
    unsafe {
        MoveFileExW(wide.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT);
    }
}

#[cfg(windows)]
fn replace_in_use(source: &str, dest: &str) -> Result<()> {
    let mut staged_old = format!("{dest}.ll10-old");
    let _ = std::fs::remove_file(&staged_old);

    // The previous image is still mapped by running processes, so it cannot be
    // deleted before the reboot that schedules it. A second replacement needs
    // its own name: renaming onto the leftover fails with access denied.
    if std::path::Path::new(&staged_old).exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0);
        staged_old = format!("{dest}.ll10-old.{stamp}");
    }

    take_ownership(dest);

    // Step 1: rename the loaded image out of the way. The mapped section keeps
    // the old bytes alive for processes that already loaded it.
    if let Err(error) = std::fs::rename(dest, &staged_old) {
        // Ownership may not have applied yet; retry once after a short pause.
        std::thread::sleep(std::time::Duration::from_millis(250));
        take_ownership(dest);
        std::fs::rename(dest, &staged_old).map_err(|second| {
            anyhow::anyhow!(
                "cannot rename {dest} (first attempt: {error}; second attempt: {second})"
            )
        })?;
    }

    // Step 2: put the replacement in place under the original name.
    if let Err(error) = std::fs::copy(source, dest) {
        // Never leave the system without the file.
        let _ = std::fs::rename(&staged_old, dest);
        return Err(anyhow::anyhow!("cannot write {dest}: {error}"));
    }

    // Step 3: the old image is still mapped, so it can only be removed on reboot.
    schedule_delete_at_reboot(&staged_old);

    Ok(())
}

// ---------------------------------------------------------------------------
// Auto-remediation: a permanent WMI event subscription.
//
// WMI is the only mechanism that reacts to a file changing underneath us (SFC
// repair, a servicing update). A scheduled task would only notice at boot.
//
// The subscription runs our own agent binary; a patch never supplies the
// command. Everything is removed again on revert.
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn run_powershell(script: &str) -> Result<()> {
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to start PowerShell: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "auto-remediation setup failed: {}",
            if stderr.trim().is_empty() { stdout } else { stderr }
        );
    }
    Ok(())
}

#[cfg(windows)]
fn build_wmi_disarm_script(patch_id: &str) -> String {
    let filter = ps_quote(&format!("LL10Remediate_{patch_id}_filter"));
    let consumer = ps_quote(&format!("LL10Remediate_{patch_id}_consumer"));

    format!(
        r#"$ErrorActionPreference='SilentlyContinue'
$ns='root\subscription'
Get-WmiObject -Namespace $ns -Class __FilterToConsumerBinding -ErrorAction SilentlyContinue |
  Where-Object {{ "$($_.Filter)" -like "*{filter}*" -or "$($_.Consumer)" -like "*{consumer}*" }} |
  Remove-WmiObject -ErrorAction SilentlyContinue
Get-WmiObject -Namespace $ns -Class __EventFilter -Filter "Name='{filter}'" -ErrorAction SilentlyContinue |
  Remove-WmiObject -ErrorAction SilentlyContinue
Get-WmiObject -Namespace $ns -Class CommandLineEventConsumer -Filter "Name='{consumer}'" -ErrorAction SilentlyContinue |
  Remove-WmiObject -ErrorAction SilentlyContinue
'disarmed'"#
    )
}

#[cfg(windows)]
fn build_wmi_arm_script(patch_id: &str, watch_path: &str, command: &str, debounce: u32) -> String {
    let filter = ps_quote(&format!("LL10Remediate_{patch_id}_filter"));
    let consumer = ps_quote(&format!("LL10Remediate_{patch_id}_consumer"));
    // WQL string literals need every backslash doubled; "C:\Windows\..." inside
    // a query is rejected as an unparsable query.
    let path = ps_quote(&watch_path.replace('\\', "\\\\"));
    let command = ps_quote(command);

    format!(
        r#"$ErrorActionPreference='Stop'
$ns='root\subscription'
{disarm}
$ErrorActionPreference='Stop'
$query="SELECT * FROM __InstanceModificationEvent WITHIN {debounce} WHERE TargetInstance ISA 'CIM_DataFile' AND TargetInstance.Name = '{path}'"
$filter = Set-WmiInstance -Namespace $ns -Class __EventFilter -Arguments @{{
  Name='{filter}'; EventNamespace='root\cimv2'; QueryLanguage='WQL'; Query=$query }}
$consumer = Set-WmiInstance -Namespace $ns -Class CommandLineEventConsumer -Arguments @{{
  Name='{consumer}'; CommandLineTemplate='{command}'; ExecutablePath='' }}
$existing = Get-WmiObject -Namespace $ns -Class __FilterToConsumerBinding -ErrorAction SilentlyContinue |
  Where-Object {{ "$($_.Filter)" -like "*{filter}*" -or "$($_.Consumer)" -like "*{consumer}*" }}
if ($existing) {{
  'already armed'
}} else {{
  Set-WmiInstance -Namespace $ns -Class __FilterToConsumerBinding -Arguments @{{
    Filter=$filter; Consumer=$consumer }} | Out-Null
  'armed'
}}"#,
        disarm = build_wmi_disarm_script(patch_id),
        debounce = debounce,
        path = path,
        filter = filter,
        consumer = consumer,
        command = command,
    )
}
