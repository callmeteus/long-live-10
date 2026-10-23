//! Bounded file-level executor used by the agent command line.
//!
//! The executor only applies `ReplaceFile` operations from a verified plan.
//! Package, registry, and other system-level operations are rejected before any
//! change is made. Every replaced file is copied into a journal first, so a
//! revert can restore the previous state.

use crate::request::AgentRequest;
use patch_core::{OperationKind, PatchPlan, PlannedOperation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Journal file written into the journal directory.
pub const JOURNAL_FILE: &str = "journal.json";

/// Journal format version understood by this executor.
pub const JOURNAL_VERSION: u32 = 1;

/// Recovery record for a single replaced file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalChange {
    /// Identifier of the plan operation that made the change.
    pub operation_id: String,
    /// Absolute target path.
    pub target: PathBuf,
    /// Backup path relative to the journal directory, when the target existed.
    pub backup: Option<PathBuf>,
}

/// Recovery record written by `apply` and consumed by `revert`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// Journal format version.
    pub version: u32,
    /// Patch that produced the changes.
    pub patch_id: String,
    /// Unix timestamp when the journal was created.
    pub created_at: u64,
    /// Unix timestamp when the changes were reverted.
    pub reverted_at: Option<u64>,
    /// Changes in execution order.
    pub changes: Vec<JournalChange>,
}

impl Journal {
    /// Reads a journal from a journal directory.
    pub fn read(journal_dir: &Path) -> Result<Self, ExecutorError> {
        let path = journal_dir.join(JOURNAL_FILE);
        let text = fs::read_to_string(&path).map_err(|source| ExecutorError::ReadJournal {
            path: path.clone(),
            source,
        })?;

        let journal: Journal = serde_json::from_str(&text)?;

        if journal.version != JOURNAL_VERSION {
            return Err(ExecutorError::UnsupportedJournal {
                version: journal.version,
            });
        }

        Ok(journal)
    }

    /// Writes the journal into a journal directory.
    pub fn write(&self, journal_dir: &Path) -> Result<(), ExecutorError> {
        let path = journal_dir.join(JOURNAL_FILE);
        let bytes = serde_json::to_vec_pretty(self)?;

        fs::write(&path, bytes).map_err(|source| ExecutorError::WriteJournal { path, source })?;

        Ok(())
    }
}

/// Problems found while applying or reverting a plan.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The plan contains conflicts and cannot be applied.
    #[error("plan has conflicts; refusing to apply")]
    PlanConflicted,
    /// The plan is not recovery ready.
    #[error("plan is not recovery ready; refusing to apply")]
    RecoveryNotReady,
    /// A journal already exists in the target directory.
    #[error("a journal already exists at {0}")]
    JournalExists(PathBuf),
    /// Operation type is outside the bounded file executor.
    #[error("operation {operation} uses unsupported kind {kind}")]
    UnsupportedOperation {
        /// Operation identifier.
        operation: String,
        /// Operation kind.
        kind: String,
    },
    /// Plan references an artifact that is not in the request.
    #[error("plan operation {operation} has no matching artifact {file_name}")]
    MissingArtifact {
        /// Operation identifier.
        operation: String,
        /// Artifact file name.
        file_name: String,
    },
    /// Artifact file is missing.
    #[error("artifact file is missing: {0}")]
    ArtifactMissing(PathBuf),
    /// Artifact size differs from the catalog.
    #[error("artifact {artifact} size is {actual} bytes, expected {expected}")]
    ArtifactSize {
        /// Artifact identifier.
        artifact: String,
        /// Recorded size.
        expected: u64,
        /// Actual size.
        actual: u64,
    },
    /// Artifact digest differs from the catalog.
    #[error("artifact {0} does not match the catalog SHA-256")]
    ArtifactDigest(String),
    /// Journal could not be read.
    #[error("failed to read journal {path}")]
    ReadJournal {
        /// Journal path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// Journal could not be written.
    #[error("failed to write journal {path}")]
    WriteJournal {
        /// Journal path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// Journal version is not supported.
    #[error("unsupported journal version {version}")]
    UnsupportedJournal {
        /// Version found on disk.
        version: u32,
    },
    /// Journal has already been reverted.
    #[error("journal has already been reverted")]
    AlreadyReverted,
    /// Backed-up file is missing.
    #[error("journal backup is missing: {0}")]
    BackupMissing(PathBuf),
    /// File system or JSON failure.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Journal JSON cannot be decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Applies a plan with the bounded file executor.
///
/// Only `Backup`, `ValidatePrecondition`, `ReplaceFile`, and `RecordRestore`
/// operations are accepted. Unsupported operations abort before any file is
/// changed. Each replaced file is backed up into `journal_dir/backups` before
/// the artifact is copied into `target_dir`.
pub fn apply(
    request: &AgentRequest,
    plan: &PatchPlan,
    artifact_dir: &Path,
    target_dir: &Path,
    journal_dir: &Path,
) -> Result<Journal, ExecutorError> {
    if !plan.conflicts.is_empty() {
        return Err(ExecutorError::PlanConflicted);
    }

    if !plan.recovery_ready {
        return Err(ExecutorError::RecoveryNotReady);
    }

    preflight(plan)?;

    let target_dir = absolute_dir(target_dir)?;
    let journal_dir = absolute_dir(journal_dir)?;
    let backups_dir = journal_dir.join("backups");

    let journal_path = journal_dir.join(JOURNAL_FILE);

    if journal_path.exists() {
        return Err(ExecutorError::JournalExists(journal_path));
    }

    fs::create_dir_all(&backups_dir)?;

    let mut journal = Journal {
        version: JOURNAL_VERSION,
        patch_id: request.patch.id.clone(),
        created_at: unix_now(),
        reverted_at: None,
        changes: Vec::new(),
    };

    journal.write(&journal_dir)?;

    for operation in &plan.operations {
        match operation.kind {
            OperationKind::Backup
            | OperationKind::ValidatePrecondition
            | OperationKind::RecordRestore => {}
            OperationKind::ReplaceFile => {
                let change = replace_file(
                    request,
                    operation,
                    artifact_dir,
                    &target_dir,
                    &journal_dir,
                    journal.changes.len(),
                )?;

                journal.changes.push(change);
                journal.write(&journal_dir)?;
            }
            OperationKind::ApplyPackage
            | OperationKind::UpdateRegistry
            | OperationKind::Rollback => {
                return Err(unsupported(operation));
            }
        }
    }

    Ok(journal)
}

/// Reverts every change in a journal in reverse order.
pub fn revert(journal_dir: &Path) -> Result<usize, ExecutorError> {
    let journal_dir = absolute_dir(journal_dir)?;
    let mut journal = Journal::read(&journal_dir)?;

    if journal.reverted_at.is_some() {
        return Err(ExecutorError::AlreadyReverted);
    }

    for change in journal.changes.iter().rev() {
        match &change.backup {
            Some(relative) => {
                let backup = journal_dir.join(relative);

                if !backup.is_file() {
                    return Err(ExecutorError::BackupMissing(backup));
                }

                if let Some(parent) = change.target.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(&backup, &change.target)?;
            }
            None => {
                if change.target.exists() {
                    fs::remove_file(&change.target)?;
                }
            }
        }
    }

    journal.reverted_at = Some(unix_now());
    journal.write(&journal_dir)?;

    Ok(journal.changes.len())
}

fn preflight(plan: &PatchPlan) -> Result<(), ExecutorError> {
    for operation in &plan.operations {
        match operation.kind {
            OperationKind::Backup
            | OperationKind::ValidatePrecondition
            | OperationKind::ReplaceFile
            | OperationKind::RecordRestore => {}
            OperationKind::ApplyPackage
            | OperationKind::UpdateRegistry
            | OperationKind::Rollback => return Err(unsupported(operation)),
        }
    }

    Ok(())
}

fn replace_file(
    request: &AgentRequest,
    operation: &PlannedOperation,
    artifact_dir: &Path,
    target_dir: &Path,
    journal_dir: &Path,
    index: usize,
) -> Result<JournalChange, ExecutorError> {
    let artifact = request
        .artifacts
        .iter()
        .find(|artifact| artifact.file_name == operation.target)
        .ok_or_else(|| ExecutorError::MissingArtifact {
            operation: operation.id.clone(),
            file_name: operation.target.clone(),
        })?;

    let source = artifact_dir.join(&artifact.file_name);

    verify_artifact(&source, &artifact.id, artifact.size_bytes, &artifact.sha256)?;

    let target = target_dir.join(&artifact.file_name);
    let backup = if target.is_file() {
        let relative = PathBuf::from("backups").join(format!("{index}-{}", artifact.file_name));
        let backup = journal_dir.join(&relative);

        fs::copy(&target, &backup)?;

        Some(relative)
    } else {
        None
    };

    fs::copy(&source, &target)?;

    Ok(JournalChange {
        operation_id: operation.id.clone(),
        target,
        backup,
    })
}

fn verify_artifact(
    path: &Path,
    artifact_id: &str,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), ExecutorError> {
    let metadata = fs::metadata(path).map_err(|_| ExecutorError::ArtifactMissing(path.into()))?;

    if metadata.len() != expected_size {
        return Err(ExecutorError::ArtifactSize {
            artifact: artifact_id.to_string(),
            expected: expected_size,
            actual: metadata.len(),
        });
    }

    let mut file = File::open(path).map_err(|_| ExecutorError::ArtifactMissing(path.into()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let digest = format!("{:x}", hasher.finalize());

    if digest != expected_sha256 {
        return Err(ExecutorError::ArtifactDigest(artifact_id.to_string()));
    }

    Ok(())
}

fn absolute_dir(path: &Path) -> Result<PathBuf, ExecutorError> {
    fs::create_dir_all(path)?;

    Ok(fs::canonicalize(path)?)
}

fn unsupported(operation: &PlannedOperation) -> ExecutorError {
    ExecutorError::UnsupportedOperation {
        operation: operation.id.clone(),
        kind: format!("{:?}", operation.kind),
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::AgentRequest;
    use catalog_schema::{
        Architecture, Artifact, ArtifactKind, PatchEntry, PatchStatus, SchemaVersion,
    };
    use patch_core::{GuestState, PatchEngine, PatchRequirements};
    use std::collections::{HashMap, HashSet};
    use tempfile::tempdir;

    fn patch() -> PatchEntry {
        PatchEntry {
            id: "KB-TEST".to_string(),
            schema_version: SchemaVersion::current(),
            cves: vec!["CVE-2026-0001".to_string()],
            component: "example".to_string(),
            builds: vec!["19045".to_string()],
            architectures: vec!["x64".to_string()],
            title: "Test patch".to_string(),
            description: "Test patch.".to_string(),
            status: PatchStatus::Experimental,
        }
    }

    fn artifact(sha256: String, size_bytes: u64) -> Artifact {
        Artifact {
            id: "fix".to_string(),
            patch_id: "KB-TEST".to_string(),
            schema_version: SchemaVersion::current(),
            kind: ArtifactKind::Dll,
            file_name: "fix.dll".to_string(),
            sha256,
            size_bytes,
            source_url: None,
        }
    }

    fn request(sha256: String, size_bytes: u64) -> AgentRequest {
        AgentRequest {
            patch: patch(),
            artifacts: vec![artifact(sha256, size_bytes)],
            state: GuestState {
                build: 19045,
                ubr: Some(6466),
                architecture: Architecture::X64,
                components: HashMap::new(),
                applied_patches: HashSet::new(),
            },
            requirements: PatchRequirements::default(),
        }
    }

    fn plan(request: &AgentRequest) -> PatchPlan {
        PatchEngine::new().plan(&request.plan_request())
    }

    #[test]
    fn applies_and_reverts_a_file_replacement() {
        let dir = tempdir().expect("temp dir");
        let artifact_dir = dir.path().join("artifacts");
        let target_dir = dir.path().join("target");
        let journal_dir = dir.path().join("journal");
        fs::create_dir_all(&artifact_dir).expect("artifact dir");

        let bytes = b"new file";
        fs::write(artifact_dir.join("fix.dll"), bytes).expect("artifact writes");
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let request = request(sha256, bytes.len() as u64);
        let plan = plan(&request);

        let journal =
            apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir).expect("plan applies");

        assert_eq!(journal.changes.len(), 1);
        assert_eq!(
            fs::read(target_dir.join("fix.dll")).expect("target exists"),
            bytes
        );

        revert(&journal_dir).expect("plan reverts");

        assert!(!target_dir.join("fix.dll").exists());
        assert!(Journal::read(&journal_dir)
            .expect("journal reads")
            .reverted_at
            .is_some());
    }

    #[test]
    fn restores_an_existing_file_on_revert() {
        let dir = tempdir().expect("temp dir");
        let artifact_dir = dir.path().join("artifacts");
        let target_dir = dir.path().join("target");
        let journal_dir = dir.path().join("journal");
        fs::create_dir_all(&artifact_dir).expect("artifact dir");
        fs::create_dir_all(&target_dir).expect("target dir");
        fs::write(target_dir.join("fix.dll"), b"old file").expect("old file writes");

        let bytes = b"new file";
        fs::write(artifact_dir.join("fix.dll"), bytes).expect("artifact writes");
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let request = request(sha256, bytes.len() as u64);
        let plan = plan(&request);

        apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir).expect("plan applies");
        revert(&journal_dir).expect("plan reverts");

        assert_eq!(
            fs::read(target_dir.join("fix.dll")).expect("target exists"),
            b"old file"
        );
    }

    #[test]
    fn refuses_an_artifact_with_the_wrong_digest_before_writing() {
        let dir = tempdir().expect("temp dir");
        let artifact_dir = dir.path().join("artifacts");
        let target_dir = dir.path().join("target");
        let journal_dir = dir.path().join("journal");
        fs::create_dir_all(&artifact_dir).expect("artifact dir");
        fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");

        let request = request("0".repeat(64), 8);
        let plan = plan(&request);

        assert!(matches!(
            apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir),
            Err(ExecutorError::ArtifactDigest(_))
        ));
        assert!(!target_dir.join("fix.dll").exists());
    }

    #[test]
    fn refuses_unsupported_operations_before_writing() {
        let dir = tempdir().expect("temp dir");
        let artifact_dir = dir.path().join("artifacts");
        let target_dir = dir.path().join("target");
        let journal_dir = dir.path().join("journal");
        fs::create_dir_all(&artifact_dir).expect("artifact dir");

        let bytes = b"new file";
        fs::write(artifact_dir.join("fix.dll"), bytes).expect("artifact writes");
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let mut request = request(sha256, bytes.len() as u64);
        request.artifacts[0].kind = ArtifactKind::Msu;
        let plan = plan(&request);

        assert!(matches!(
            apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir),
            Err(ExecutorError::UnsupportedOperation { .. })
        ));
        assert!(!target_dir.join("fix.dll").exists());
    }

    #[test]
    fn refuses_a_second_apply_into_the_same_journal() {
        let dir = tempdir().expect("temp dir");
        let artifact_dir = dir.path().join("artifacts");
        let target_dir = dir.path().join("target");
        let journal_dir = dir.path().join("journal");
        fs::create_dir_all(&artifact_dir).expect("artifact dir");

        let bytes = b"new file";
        fs::write(artifact_dir.join("fix.dll"), bytes).expect("artifact writes");
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let request = request(sha256, bytes.len() as u64);
        let plan = plan(&request);

        apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir).expect("plan applies");

        assert!(matches!(
            apply(&request, &plan, &artifact_dir, &target_dir, &journal_dir),
            Err(ExecutorError::JournalExists(_))
        ));
    }

    #[test]
    fn refuses_to_revert_twice() {
        let dir = tempdir().expect("temp dir");
        let journal_dir = dir.path().join("journal");
        let journal = Journal {
            version: JOURNAL_VERSION,
            patch_id: "KB-TEST".to_string(),
            created_at: 1,
            reverted_at: Some(2),
            changes: Vec::new(),
        };

        fs::create_dir_all(&journal_dir).expect("journal dir");
        journal.write(&journal_dir).expect("journal writes");

        assert!(matches!(
            revert(&journal_dir),
            Err(ExecutorError::AlreadyReverted)
        ));
    }
}
