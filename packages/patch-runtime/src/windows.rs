#[cfg(windows)]
use anyhow::Result;
#[cfg(windows)]
use patch_core::PatchManifest;
#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
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
        // Try normal copy first
        if std::fs::copy(&source, &dest).is_ok() {
            return Ok(());
        }
        // WRP-protected: take ownership, grant access, then use MoveFileEx for pending rename
        let _ = std::process::Command::new("takeown")
            .args(["/f", &dest, "/a"])
            .output();
        let _ = std::process::Command::new("icacls")
            .args([&dest, "/grant", "administrators:F"])
            .output();
        // Use MoveFileEx with DELAY_UNTIL_REBOOT
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(lpExistingFileName: *const u16, lpNewFileName: *const u16, dwFlags: u32) -> i32;
        }
        const MOVEFILE_REPLACE_EXISTING: u32 = 0x00000001;
        const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 0x00000004;
        let src: Vec<u16> = OsStr::new(&source).encode_wide().chain(std::iter::once(0)).collect();
        let dst: Vec<u16> = OsStr::new(&dest).encode_wide().chain(std::iter::once(0)).collect();
        let result = unsafe {
            MoveFileExW(src.as_ptr(), dst.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_DELAY_UNTIL_REBOOT)
        };
        if result == 0 {
            return Err(anyhow::anyhow!("MoveFileEx failed: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn restore_file(&self, backup: &str, dest: &str) -> Result<()> {
        let backup = expand_env(backup);
        let dest = expand_env(dest);
        if std::fs::copy(&backup, &dest).is_ok() {
            return Ok(());
        }
        crate::trusted_installer::replace_file_trusted(&backup, &dest)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    fn schedule_rename(&self, source: &str, dest: &str) -> Result<()> {
        let source = expand_env(source);
        let dest = expand_env(dest);
        let nt_source = format!("\\??\\{}", source);
        let nt_dest = format!("\\??\\{}", dest);

        // Create pending.xml using line-by-line construction to avoid format string issues
        let mut xml = String::new();
        xml.push_str("<?xml version='1.0' encoding='utf-8'?>\n\n");
        xml.push_str("<PendingTransaction Version=\"3.1\" WcpVersion=\"10.0.22621.2567 (WinBuild.160101.0800)\" Identifier=\"916ae75edb30da0146730000dc1be027\">\n");
        xml.push_str("\t<Transactions>\n");
        xml.push_str("\t</Transactions>\n");
        xml.push_str("\t<ChangeList>\n");
        xml.push_str("\t</ChangeList>\n");
        xml.push_str("\t<POQ postAction=\"reboot\">\n");
        xml.push_str(&format!("\t\t<HardlinkFile source=\"{}\" destination=\"{}\"/>\n", nt_source, nt_dest));
        xml.push_str("\t</POQ>\n");
        xml.push_str("\t<POQ>\n");
        xml.push_str("\t</POQ>\n");
        xml.push_str("\t<InstallerQueue Length=\"0x00000000\">\n");
        xml.push_str("\t</InstallerQueue>\n");
        xml.push_str("\t<RollbackInformation>\n");
        xml.push_str("\t\t<AdminHints>\n");
        xml.push_str("\t\t</AdminHints>\n");
        xml.push_str("\t</RollbackInformation>\n");
        xml.push_str("</PendingTransaction>");

        let pending_path = "C:\\\\Windows\\\\WinSxS\\\\pending.xml";
        std::fs::write(&pending_path, &xml)
            .map_err(|e| anyhow::anyhow!("failed to write pending.xml: {}", e))?;

        // Set TrustedInstaller to auto-start
        let _ = std::process::Command::new("sc")
            .args(["config", "TrustedInstaller", "start=", "auto"])
            .output();

        // Set PoqexecCmdline registry value (REG_MULTI_SZ)
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        let (key, _) = hklm.create_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\SideBySide\Configuration")?;
        let poqexec_cmd = format!("C:\\Windows\\System32\\poqexec.exe /display_progress \\SystemRoot\\WinSxS\\pending.xml");
        key.set_value("PoqexecCmdline", &vec![poqexec_cmd])
            .map_err(|e| anyhow::anyhow!("failed to set PoqexecCmdline: {}", e))?;

        // Set PendingXmlIdentifier in COMPONENTS hive (REG_BINARY, UTF-16)
        // Use reg.exe to set it (runs as admin, may need TrustedInstaller)
        let identifier = "916ae75edb30da0146730000dc1be027";
        let mut bytes = Vec::new();
        for c in identifier.chars() {
            bytes.push(c as u8);
            bytes.push(0);
        }
        bytes.push(0);
        bytes.push(0);
        // Write bytes to a file and use reg.exe import
        let hex_str: String = bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join("");
        let reg_content = format!(
            "Windows Registry Editor Version 5.00\n\n[HKEY_LOCAL_MACHINE\\COMPONENTS]\n\"PendingXmlIdentifier\"=hex:{}",
            hex_str
        );
        let reg_file = "C:\\\\Windows\\\\Temp\\\\pending_id.reg";
        std::fs::write(&reg_file, &reg_content)
            .map_err(|e| anyhow::anyhow!("failed to write reg file: {}", e))?;
        let _ = std::process::Command::new("reg")
            .args(["import", &reg_file])
            .output();
        // Also try via winreg
        let components_key = hklm.open_subkey("COMPONENTS");
        if let Ok(key) = components_key {
            use winreg::RegValue;
            let raw = winreg::RegValue {
                vtype: winreg::enums::RegType::REG_BINARY,
                bytes: bytes.clone(),
            };
            let _ = key.set_raw_value("PendingXmlIdentifier", &raw);
        }
        let _ = std::fs::remove_file(&reg_file);

        Ok(())
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

    fn arm_auto_remediate(&self, patch_id: &str, _manifest: &PatchManifest, config: &patch_core::AutoRemediate) -> Result<()> {
        if !config.enabled { return Ok(()); }
        // Create WMI event subscription (simplified - would need WMI COM calls)
        // For now, create a scheduled task as fallback
        let task_name = format!("LL10_AutoRemediate_{}", patch_id);
        let action = format!(
            "powershell.exe -ExecutionPolicy Bypass -WindowStyle Hidden -Command \"& ll10-agent patches verify {} --json\"",
            patch_id
        );
        // Use schtasks to create a boot-time task
        let _ = std::process::Command::new("schtasks")
            .args(["/create", "/tn", &task_name, "/tr", &action, "/sc", "onstart", "/ru", "SYSTEM", "/rl", "HIGHEST", "/f"])
            .output();
        Ok(())
    }





    fn disarm_auto_remediate(&self, patch_id: &str) -> Result<()> {
        let task_name = format!("LL10_AutoRemediate_{}", patch_id);
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", &task_name, "/f"])
            .output();
        Ok(())
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
