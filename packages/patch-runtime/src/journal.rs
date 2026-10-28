use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub op_id: String,
    pub path: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryBackup {
    pub op_id: String,
    pub hive: String,
    pub key: String,
    pub name: String,
    pub reg_type: String,
    pub previous_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedEntry {
    pub op_id: String,
    pub artifact: String,
    pub dest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    pub patch_id: String,
    pub created_at: String,
    pub reverted: bool,
    pub reverted_at: Option<String>,
    pub file_changes: Vec<JournalEntry>,
    pub registry_backups: Vec<RegistryBackup>,
    pub staged: Vec<StagedEntry>,
    pub pending_reboot: bool,
    pub pending_reboot_entries: Vec<(String, String)>,
}

impl Journal {
    pub fn new(patch_id: String) -> Self {
        Self {
            patch_id,
            created_at: format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
            reverted: false,
            reverted_at: None,
            file_changes: Vec::new(),
            registry_backups: Vec::new(),
            staged: Vec::new(),
            pending_reboot: false,
            pending_reboot_entries: Vec::new(),
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let journal: Journal = serde_json::from_str(&content)?;
        Ok(journal)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_backup(&mut self, op_id: String, path: String, backup: String) {
        self.file_changes.push(JournalEntry {
            op_id,
            path,
            backup_path: Some(backup),
        });
    }

    pub fn add_change(&mut self, op_id: String, path: String) {
        self.file_changes.push(JournalEntry {
            op_id,
            path,
            backup_path: None,
        });
    }

    pub fn add_staged(&mut self, op_id: String, artifact: String, dest: String) {
        self.staged.push(StagedEntry { op_id, artifact, dest });
    }

    pub fn staged_path(&self, op_id: &str) -> Option<String> {
        self.staged.iter().find(|s| s.op_id == op_id).map(|s| s.dest.clone())
    }

    pub fn add_registry_backup(&mut self, op_id: String, hive: String, key: String, name: String, prev: Option<String>) {
        self.registry_backups.push(RegistryBackup {
            op_id,
            hive,
            key,
            name,
            reg_type: "string".to_string(),
            previous_value: prev,
        });
    }

    pub fn add_registry_change(&mut self, _op_id: String, _hive: String, _key: String, _name: String) {}

    pub fn add_pending(&mut self, _op_id: String, dest: String, source: String) {
        self.pending_reboot_entries.push((dest, source));
    }
}
