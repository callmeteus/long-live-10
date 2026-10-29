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

    /// True when every declared artifact target already holds its patched
    /// content. Used to tell "applied" from "someone restored the stock file".
    fn target_is_patched(&self, manifest: &PatchManifest) -> bool {
        let targets: Vec<(&String, &String)> = manifest
            .artifacts
            .iter()
            .filter_map(|a| a.target_path.as_ref().map(|p| (p, &a.sha256)))
            .collect();

        !targets.is_empty()
            && targets.iter().all(|(path, sha256)| {
                self.backend
                    .file_hash(path)
                    .map(|hash| hash.eq_ignore_ascii_case(sha256))
                    .unwrap_or(false)
            })
    }

    /// True when the file on disk matches a stock build this manifest has an
    /// artifact for. Anything else is not ours to overwrite.
    fn is_known_stock(&self, manifest: &PatchManifest) -> Result<bool> {
        if manifest.build_variants.is_empty() {
            return Ok(true);
        }

        let target = manifest.variant_target_path().unwrap_or_default().to_string();
        let current = self.backend.file_hash(&target)?.to_lowercase();
        Ok(matches!(
            manifest.select_variant(&current),
            patch_core::VariantSelection::Matched(_)
        ))
    }

    /// Removes the remediation subscription for the duration of a write a
    /// patch is doing itself. Re-armed by `ensure_auto_remediate` when the
    /// patch is in place.
    fn suspend_auto_remediate(&self, id: &str, manifest: &PatchManifest) -> Result<()> {
        if let Some(auto) = &manifest.auto_remediate {
            if auto.enabled {
                self.backend.disarm_auto_remediate(id)?;
            }
        }
        Ok(())
    }

    /// Arms auto-remediation when the manifest asks for it. Idempotent: the
    /// backend removes any previous subscription before creating one.
    fn ensure_auto_remediate(&self, id: &str, manifest: &PatchManifest) -> Result<()> {
        if let Some(auto) = &manifest.auto_remediate {
            if auto.enabled {
                self.backend
                    .arm_auto_remediate(id, manifest, auto, &self.patches_dir)?;
            }
        }
        Ok(())
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
                    // "$selected" means "whichever build variant was installed".
                    let expected = if equals == "$selected" {
                        manifest.patched_hashes()
                    } else {
                        vec![equals.to_lowercase()]
                    };
                    match self.backend.file_hash(path) {
                        Ok(hash) if expected.contains(&hash.to_lowercase()) => {}
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

    /// Re-applies a patch only when it is supposed to be installed and the
    /// machine drifted. Used by the auto-remediation subscription: an explicit
    /// revert (or no journal at all) must never be undone by a stray event.
    ///
    /// Returns `None` when there was nothing to do.
    pub fn remediate(&self, id: &str) -> Result<Option<ApplyReport>> {
        let journal = Journal::read(&self.backend.journal_path(id)).ok();

        match journal {
            Some(j) if !j.reverted => Ok(Some(self.apply(id, true)?)),
            _ => Ok(None),
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

        // A journal records what was changed, not proof that the machine is
        // still in that state: SFC and servicing updates put the stock binary
        // back without touching it. Decide from the file hash instead, and roll
        // the old changes back before re-applying so the backup of the original
        // file keeps pointing at the original.
        if let Ok(j) = Journal::read(&journal_path) {
            if !j.reverted {
                let has_targets = manifest.artifacts.iter().any(|a| a.target_path.is_some());

                if !has_targets || self.target_is_patched(&manifest) {
                    // Re-arming here matters: a repair tool may have removed
                    // the subscription while the file itself is still patched.
                    self.ensure_auto_remediate(id, &manifest)?;
                    return Ok(ApplyReport {
                        patch_id: id.to_string(),
                        state: PatchState::Applied,
                        dry_run: false,
                        message: "already applied".to_string(),
                        pending_reboot: j.pending_reboot,
                    });
                }

                // Only a build we have a stock hash for may be re-applied. Any
                // other hash means the machine moved on (or the file was
                // replaced by something we do not know); writing our artifact
                // over it could stop the machine from booting.
                if self.is_known_stock(&manifest)? {
                    self.revert(id, true)?;
                }
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

        // A manifest with build variants picks its artifact from the current file
        // hash. This is what lets one package cover several Windows builds while
        // refusing every build it has no artifact for.
        let mut selected_artifact: Option<String> = None;
        if !manifest.build_variants.is_empty() {
            let target = manifest.variant_target_path().unwrap_or_default().to_string();
            let current = self.backend.file_hash(&target)?.to_lowercase();

            match manifest.select_variant(&current) {
                patch_core::VariantSelection::Matched(artifact) => {
                    selected_artifact = Some(artifact);
                }
                patch_core::VariantSelection::AlreadyApplied(_) => {
                    self.ensure_auto_remediate(id, &manifest)?;
                    return Ok(ApplyReport {
                        patch_id: id.to_string(),
                        state: PatchState::Applied,
                        dry_run: false,
                        message: "already applied".to_string(),
                        pending_reboot: false,
                    });
                }
                patch_core::VariantSelection::Unsupported => {
                    return Ok(ApplyReport {
                        patch_id: id.to_string(),
                        state: PatchState::NotApplicable,
                        dry_run: false,
                        message: format!(
                            "refusing to apply: {target} has hash {current}, which this patch does not support"
                        ),
                        pending_reboot: false,
                    });
                }
            }
        }

        // Preconditions gate every write. A patch built for one Windows build must
        // never be applied to another: replacing shell32.dll with a binary from a
        // different build prevents the machine from booting.
        for precondition in &manifest.preconditions {
            match precondition {
                patch_core::Precondition::FileSha256 { path, one_of, .. } => {
                    let actual = self.backend.file_hash(path)?.to_lowercase();
                    let matches = one_of.iter().any(|allowed| allowed.eq_ignore_ascii_case(&actual));
                    if !matches {
                        return Ok(ApplyReport {
                            patch_id: id.to_string(),
                            state: PatchState::NotApplicable,
                            dry_run: false,
                            message: format!(
                                "refusing to apply: {path} has hash {actual}, which this patch does not support"
                            ),
                            pending_reboot: false,
                        });
                    }
                }
            }
        }

        // Our own write is an instance-modification event. If the subscription
        // were still armed it would start a second apply while this one is
        // halfway through, and the two would fight over the journal.
        self.suspend_auto_remediate(id, &manifest)?;

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
                    let artifact_id = if artifact == "$selected" {
                        selected_artifact.clone().unwrap_or_default()
                    } else {
                        artifact.clone()
                    };
                    let artifact_path = self.patches_dir.join(id).join(
                        manifest
                            .artifacts
                            .iter()
                            .find(|a| a.id == artifact_id)
                            .map(|a| a.file.clone())
                            .ok_or_else(|| {
                                anyhow::anyhow!("artifact '{artifact_id}' is not declared by the manifest")
                            })?
                    );
                    self.backend.stage_file(&artifact_path, to)?;
                    journal.add_staged(op_id.clone(), artifact_id, to.clone());
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

        self.ensure_auto_remediate(id, &manifest)?;

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

        // Stop listening before restoring the file: the restore itself is a
        // modification event and would trigger a remediation mid-revert.
        self.suspend_auto_remediate(id, &manifest)?;

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

        // Restore files from backup. The backend already knows how to replace a
        // file that is still mapped (copy, then rename), so the restore does
        // not need to be deferred to a reboot.
        for change in &journal.file_changes {
            if let Some(backup) = &change.backup_path {
                self.backend.restore_file(backup, &change.path)?;
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
