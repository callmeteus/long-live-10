use anyhow::Result;
use patch_core::PatchManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Abstract system backend for patch operations.
pub trait SystemBackend: Send + Sync {
    fn system_state(&self) -> HashMap<String, String>;
    fn is_admin(&self) -> bool;
    fn file_hash(&self, path: &str) -> Result<String>;
    fn backup_file(&self, path: &str) -> Result<String>;
    fn stage_file(&self, source: &Path, dest: &str) -> Result<()>;
    fn replace_file(&self, source: &str, dest: &str) -> Result<()>;
    fn restore_file(&self, backup: &str, dest: &str) -> Result<()>;
    fn schedule_rename(&self, source: &str, dest: &str) -> Result<()>;
    fn cancel_rename(&self, dest: &str, source: &str) -> Result<()>;
    fn registry_get(&self, hive: &str, key: &str, name: &str) -> Result<String>;
    fn registry_set(&self, hive: &str, key: &str, name: &str, reg_type: &str, data: &serde_json::Value) -> Result<()>;
    fn registry_delete(&self, hive: &str, key: &str, name: &str) -> Result<()>;
    fn journal_path(&self, patch_id: &str) -> PathBuf;
    fn arm_auto_remediate(
        &self,
        patch_id: &str,
        manifest: &PatchManifest,
        config: &patch_core::AutoRemediate,
        patches_dir: &std::path::Path,
    ) -> Result<()>;
    fn disarm_auto_remediate(&self, patch_id: &str) -> Result<()>;
}

/// In-memory backend for testing.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    pub files: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    pub registry: std::sync::Mutex<HashMap<String, String>>,
    pub admin: bool,
    pub build: String,
    pub arch: String,
    pub journal_dir: PathBuf,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            admin: true,
            build: "19045".to_string(),
            arch: "x86_64".to_string(),
            journal_dir: std::env::temp_dir().join("ll10-journal"),
            ..Default::default()
        }
    }
}

impl SystemBackend for MemoryBackend {
    fn system_state(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("build".to_string(), self.build.clone());
        m.insert("architecture".to_string(), self.arch.clone());
        m
    }
    fn is_admin(&self) -> bool { self.admin }
    fn file_hash(&self, path: &str) -> Result<String> {
        let files = self.files.lock().unwrap();
        if let Some(data) = files.get(path) {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(format!("{:x}", hasher.finalize()))
        } else {
            anyhow::bail!("file not found: {}", path)
        }
    }
    fn backup_file(&self, path: &str) -> Result<String> {
        let backup = format!("{}.bak", path);
        let data = {
            let files = self.files.lock().unwrap();
            files.get(path).cloned()
        };
        if let Some(data) = data {
            let mut files = self.files.lock().unwrap();
            files.insert(backup.clone(), data);
            Ok(backup)
        } else {
            anyhow::bail!("file not found: {}", path)
        }
    }
    fn stage_file(&self, source: &std::path::Path, dest: &str) -> Result<()> {
        let data = std::fs::read(source).unwrap_or_default();
        let mut files = self.files.lock().unwrap();
        files.insert(dest.to_string(), data);
        Ok(())
    }
    fn replace_file(&self, source: &str, dest: &str) -> Result<()> {
        let data = {
            let files = self.files.lock().unwrap();
            files.get(source).cloned().unwrap_or_default()
        };
        let mut files = self.files.lock().unwrap();
        files.insert(dest.to_string(), data);
        Ok(())
    }
    fn restore_file(&self, backup: &str, dest: &str) -> Result<()> {
        let data = {
            let files = self.files.lock().unwrap();
            files.get(backup).cloned()
        };
        if let Some(data) = data {
            let mut files = self.files.lock().unwrap();
            files.insert(dest.to_string(), data);
            Ok(())
        } else {
            anyhow::bail!("backup not found: {}", backup)
        }
    }
    fn schedule_rename(&self, source: &str, dest: &str) -> Result<()> {
        // The real backend installs the file immediately (rename, then copy),
        // so the in-memory test double must do the same; otherwise revert would
        // look like it worked while the target kept its patched content.
        self.replace_file(source, dest)
    }
    fn cancel_rename(&self, _dest: &str, _source: &str) -> Result<()> { Ok(()) }
    fn registry_get(&self, _hive: &str, key: &str, name: &str) -> Result<String> {
        let registry = self.registry.lock().unwrap();
        let k = format!("{}\\{}", key, name);
        registry.get(&k).cloned().ok_or_else(|| anyhow::anyhow!("not found"))
    }
    fn registry_set(&self, _hive: &str, key: &str, name: &str, _reg_type: &str, data: &serde_json::Value) -> Result<()> {
        let mut registry = self.registry.lock().unwrap();
        let k = format!("{}\\{}", key, name);
        registry.insert(k, data.to_string().trim_matches('"').to_string());
        Ok(())
    }
    fn registry_delete(&self, _hive: &str, key: &str, name: &str) -> Result<()> {
        let mut registry = self.registry.lock().unwrap();
        let k = format!("{}\\{}", key, name);
        registry.remove(&k);
        Ok(())
    }
    fn journal_path(&self, patch_id: &str) -> PathBuf {
        self.journal_dir.join(patch_id).join("journal.json")
    }
    fn arm_auto_remediate(
        &self,
        _patch_id: &str,
        _manifest: &PatchManifest,
        _config: &patch_core::AutoRemediate,
        _patches_dir: &std::path::Path,
    ) -> Result<()> { Ok(()) }
    fn disarm_auto_remediate(&self, _patch_id: &str) -> Result<()> { Ok(()) }
}
