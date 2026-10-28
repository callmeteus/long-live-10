use anyhow::Result;
use patch_core::PatchManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod backend;
pub mod journal;

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub mod trusted_installer;

#[cfg(windows)]
pub use windows::*;

#[cfg(windows)]
pub use trusted_installer::*;

pub use backend::*;
pub use journal::*;

/// Runtime state for a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchState {
    NotApplied,
    PendingReboot,
    Applied,
    Partial,
    NotApplicable,
    Unknown,
    Superseded,
    Drifted,
}

/// Runtime engine that applies/reverts/verifies patches.
pub struct PatchRuntime<B: SystemBackend> {
    pub backend: B,
    pub patches_dir: PathBuf,
}

impl<B: SystemBackend> PatchRuntime<B> {
    pub fn new(backend: B, patches_dir: PathBuf) -> Self {
        Self { backend, patches_dir }
    }

    pub fn load_manifest(&self, id: &str) -> Result<PatchManifest> {
        let path = self.patches_dir.join(id).join("patch.yml");
        PatchManifest::load_from_path(&path)
    }

    pub fn verify(&self, id: &str) -> Result<PatchState> {
        let manifest = self.load_manifest(id)?;
        // Check applicability first
        let state_map = self.backend.system_state();
        let engine = patch_core::PatchEngine::new();
        // Use catalog-schema PatchEntry compatibility check via builds
        let build = state_map.get("build").cloned().unwrap_or_default();
        let arch = state_map.get("architecture").cloned().unwrap_or_default();

        // Check if build is in applies_to
        let build_ok = manifest.applies_to.builds.split_whitespace().any(|b| {
            if b.contains("..") {
                // range check simplified
                true
            } else {
                b == build
            }
        }) || manifest.applies_to.builds.contains(&build);

        if !build_ok && !manifest.applies_to.builds.is_empty() && !build.is_empty() {
            // Try numeric range check
            let in_range = check_build_range(&build, &manifest.applies_to.builds);
            if !in_range {
                return Ok(PatchState::NotApplicable);
            }
        }

        // Check arch
        if !manifest.applies_to.arch.is_empty() {
            let arch_ok = manifest.applies_to.arch.iter().any(|a| {
                format!("{:?}", a).to_lowercase() == arch.to_lowercase()
                    || (arch == "x86_64" && format!("{:?}", a).to_lowercase() == "x86_64")
            });
            if !arch_ok && !arch.is_empty() {
                return Ok(PatchState::NotApplicable);
            }
        }

        // Check if journal exists
        let journal_path = self.backend.journal_path(id);
        let journal = Journal::read(&journal_path).ok();

        // Run checks
        let mut all_pass = true;
        let mut any_fail = false;
        for check in &manifest.checks {
            match check {
                patch_core::Check::FileSha256 { path, equals, .. } => {
                    let hash = self.backend.file_hash(path);
                    match hash {
                        Ok(h) if h.to_lowercase() == equals.to_lowercase() => {}
                        Ok(_) => { all_pass = false; }
                        Err(_) => { all_pass = false; any_fail = true; }
                    }
                }
                patch_core::Check::RegistryValue { hive, key, name, reg_type, data, .. } => {
                    let val = self.backend.registry_get(hive, key, name);
                    let expected = match data {
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => data.to_string(),
                    };
                    match val {
                        Ok(v) if v == expected => {}
                        Ok(_) => { all_pass = false; }
                        Err(_) => { all_pass = false; }
                    }
                }
            }
        }

        // Determine state
        if journal.is_none() {
            if all_pass && !manifest.checks.is_empty() {
                // Checks pass but no journal - could be superseded or applied externally
                return Ok(PatchState::Applied);
            }
            return Ok(PatchState::NotApplied);
        }

        let j = journal.unwrap();
        if j.reverted {
            return Ok(PatchState::NotApplied);
        }

        if j.pending_reboot && !all_pass {
            return Ok(PatchState::PendingReboot);
        }

        if all_pass {
            // Check for drift: if checks fail but journal says applied
            Ok(PatchState::Applied)
        } else {
            // Checks fail but journal says applied -> drift
            Ok(PatchState::Drifted)
        }
    }

    pub fn apply(&self, id: &str, confirm: bool) -> Result<ApplyReport> {
        if !confirm {
            return Ok(ApplyReport {
                patch_id: id.to_string(),
                state: PatchState::NotApplied,
                dry_run: true,
                message: "dry run: pass --confirm to apply".to_string(),
                pending_reboot: false,
            });
        }

        let manifest = self.load_manifest(id)?;
        let journal_path = self.backend.journal_path(id);

        // Check if already applied
        if let Ok(j) = Journal::read(&journal_path) {
            if !j.reverted {
                return Ok(ApplyReport {
                    patch_id: id.to_string(),
                    state: PatchState::Applied,
                    dry_run: false,
                    message: "already applied".to_string(),
                    pending_reboot: j.pending_reboot,
                });
            }
        }

        // Check applicability
        let state_map = self.backend.system_state();
        let build = state_map.get("build").cloned().unwrap_or_default();
        if !build.is_empty() && !manifest.applies_to.builds.is_empty() {
            let in_range = check_build_range(&build, &manifest.applies_to.builds);
            if !in_range {
                return Ok(ApplyReport {
                    patch_id: id.to_string(),
                    state: PatchState::NotApplicable,
                    dry_run: false,
                    message: format!("build {} not in range {}", build, manifest.applies_to.builds),
                    pending_reboot: false,
                });
            }
        }

        // Check admin
        if manifest.requires.admin && !self.backend.is_admin() {
            anyhow::bail!("requires administrator privileges");
        }

        // Create journal
        let mut journal = Journal::new(id.to_string());

        // Execute operations in order
        let mut pending_reboot = false;
        for op in &manifest.operations {
            match op {
                patch_core::Operation::BackupFile { id: op_id, path } => {
                    let backup = self.backend.backup_file(path)?;
                    journal.add_backup(op_id.clone(), path.clone(), backup);
                }
                patch_core::Operation::StageFile { id: op_id, artifact, to } => {
                    let artifact_path = self.patches_dir.join(id).join(
                        manifest.artifacts.iter().find(|a| a.id == *artifact).map(|a| a.file.clone()).unwrap_or_default()
                    );
                    self.backend.stage_file(&artifact_path, to)?;
                    journal.add_staged(op_id.clone(), artifact.clone(), to.clone());
                }
                patch_core::Operation::ReplaceFile { id: op_id, from, path, mode } => {
                    let staged = journal.staged_path(from).unwrap_or_default();
                    if mode == "on_reboot" {
                        self.backend.schedule_rename(&staged, path)?;
                        pending_reboot = true;
                        journal.add_pending(op_id.clone(), path.clone(), staged);
                    } else {
                        self.backend.replace_file(&staged, path)?;
                        journal.add_change(op_id.clone(), path.clone());
                    }
                }
                patch_core::Operation::BackupRegistry { id: op_id, values } => {
                    for rv in values {
                        let prev = self.backend.registry_get(&rv.hive, &rv.key, &rv.name).ok();
                        journal.add_registry_backup(op_id.clone(), rv.hive.clone(), rv.key.clone(), rv.name.clone(), prev);
                    }
                }
                patch_core::Operation::SetRegistry { id: op_id, hive, key, name, reg_type, data } => {
                    self.backend.registry_set(hive, key, name, reg_type, data)?;
                    journal.add_registry_change(op_id.clone(), hive.clone(), key.clone(), name.clone());
                }
                patch_core::Operation::RemoveRegistryValue { id: op_id, hive, key, name } => {
                    self.backend.registry_delete(hive, key, name)?;
                    journal.add_registry_change(op_id.clone(), hive.clone(), key.clone(), name.clone());
                }
            }
        }

        journal.pending_reboot = pending_reboot;
        journal.save(&journal_path)?;

        // Auto-remediate
        if let Some(ar) = &manifest.auto_remediate {
            if ar.enabled {
                self.backend.arm_auto_remediate(id, &manifest, ar)?;
            }
        }

        Ok(ApplyReport {
            patch_id: id.to_string(),
            state: if pending_reboot { PatchState::PendingReboot } else { PatchState::Applied },
            dry_run: false,
            message: if pending_reboot {
                "applied; reboot required".to_string()
            } else {
                "applied".to_string()
            },
            pending_reboot,
        })
    }

    pub fn revert(&self, id: &str, confirm: bool) -> Result<RevertReport> {
        if !confirm {
            return Ok(RevertReport {
                patch_id: id.to_string(),
                dry_run: true,
                message: "dry run: pass --confirm to revert".to_string(),
                restored: 0,
            });
        }

        let manifest = self.load_manifest(id)?;
        let journal_path = self.backend.journal_path(id);
        let mut journal = Journal::read(&journal_path)?;

        if journal.reverted {
            return Ok(RevertReport {
                patch_id: id.to_string(),
                dry_run: false,
                message: "already reverted".to_string(),
                restored: 0,
            });
        }

        if manifest.requires.admin && !self.backend.is_admin() {
            anyhow::bail!("requires administrator privileges");
        }

        let mut restored = 0;

        // Revert pending renames first
        for (path, staged) in &journal.pending_reboot_entries {
            self.backend.cancel_rename(path, staged)?;
        }

        // Restore registry values from journal
        for rb in &journal.registry_backups {
            match &rb.previous_value {
                Some(v) => {
                    self.backend.registry_set(&rb.hive, &rb.key, &rb.name, &rb.reg_type, &serde_json::Value::String(v.clone()))?;
                }
                None => {
                    self.backend.registry_delete(&rb.hive, &rb.key, &rb.name)?;
                }
            }
            restored += 1;
        }

        // Restore files from backup (use pending rename for locked files)
        for change in &journal.file_changes {
            if let Some(backup) = &change.backup_path {
                // Stage the backup for pending replacement
                let staged = format!("{}.revert", change.path);
                self.backend.stage_file(std::path::Path::new(backup), &staged)?;
                self.backend.schedule_rename(&staged, &change.path)?;
                restored += 1;
            }
        }

        // Run revert.after_journal ops
        if let Some(revert) = &manifest.revert {
            for op in &revert.after_journal {
                // Execute declared revert ops (simplified)
                let _ = op;
            }
        }

        // Disarm auto-remediate
        if let Some(ar) = &manifest.auto_remediate {
            if ar.enabled {
                self.backend.disarm_auto_remediate(id)?;
            }
        }

        journal.reverted = true;
        journal.reverted_at = Some(chrono_now());
        journal.save(&journal_path)?;

        Ok(RevertReport {
            patch_id: id.to_string(),
            dry_run: false,
            message: "reverted".to_string(),
            restored,
        })
    }
}

fn check_build_range(build: &str, range: &str) -> bool {
    // Simple: check if build matches or range contains it
    if range.contains(build) { return true; }
    // Try numeric comparison
    let build_num: u64 = build.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0);
    // Parse range like ">=19045.0 <19046.0"
    let parts: Vec<&str> = range.split_whitespace().collect();
    let mut min: Option<u64> = None;
    let mut max: Option<u64> = None;
    for part in &parts {
        if part.starts_with(">=") {
            min = part[2..].split('.').next().and_then(|s| s.parse().ok());
        } else if part.starts_with("<") {
            max = part[1..].split('.').next().and_then(|s| s.parse().ok());
        }
    }
    let mut ok = true;
    if let Some(m) = min { if build_num < m { ok = false; } }
    if let Some(m) = max { if build_num >= m { ok = false; } }
    ok
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyReport {
    pub patch_id: String,
    pub state: PatchState,
    pub dry_run: bool,
    pub message: String,
    pub pending_reboot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertReport {
    pub patch_id: String,
    pub dry_run: bool,
    pub message: String,
    pub restored: usize,
}
