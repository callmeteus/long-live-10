use agent::request::AgentRequest;
use agent::Diagnostics;
use catalog_schema::{
    Architecture, Artifact, ArtifactKind, PatchEntry, PatchStatus, SchemaVersion,
};
use patch_core::{GuestState, PatchPlan, PatchRequirements};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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

fn artifact(kind: ArtifactKind, bytes: &[u8]) -> Artifact {
    Artifact {
        id: "fix".to_string(),
        patch_id: "KB-TEST".to_string(),
        schema_version: SchemaVersion::current(),
        kind,
        file_name: "fix.dll".to_string(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        source_url: None,
    }
}

fn request(kind: ArtifactKind, bytes: &[u8]) -> AgentRequest {
    AgentRequest {
        patch: patch(),
        artifacts: vec![artifact(kind, bytes)],
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

fn write_request(dir: &Path, request: &AgentRequest) -> PathBuf {
    let path = dir.join("request.json");

    fs::write(
        &path,
        serde_json::to_vec_pretty(request).expect("request serializes"),
    )
    .expect("request writes");

    path
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ll10-agent"))
        .args(arguments)
        .output()
        .expect("agent runs")
}

#[test]
fn diagnostics_json_is_a_valid_payload() {
    let output = run(&["diagnostics", "--json"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Diagnostics =
        serde_json::from_slice(&output.stdout).expect("diagnostics JSON parses");

    payload.validate().expect("diagnostics are valid");
    assert_eq!(payload.agent_version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn plan_prints_a_conflict_free_plan() {
    let dir = tempdir().expect("temp dir");
    let request = request(ArtifactKind::Dll, b"new file");
    let request_path = write_request(dir.path(), &request);

    let output = run(&["plan", "--request", request_path.to_str().unwrap()]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let plan: PatchPlan = serde_json::from_slice(&output.stdout).expect("plan JSON parses");

    assert!(plan.conflicts.is_empty());
    assert!(plan.recovery_ready);
    assert!(plan
        .operations
        .iter()
        .any(|operation| matches!(operation.kind, patch_core::OperationKind::ReplaceFile)));
}

#[test]
fn apply_without_confirm_is_a_dry_run() {
    let dir = tempdir().expect("temp dir");
    let request = request(ArtifactKind::Dll, b"new file");
    let request_path = write_request(dir.path(), &request);
    let artifact_dir = dir.path().join("artifacts");
    let target_dir = dir.path().join("target");
    let journal_dir = dir.path().join("journal");
    fs::create_dir_all(&artifact_dir).expect("artifact dir");
    fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");

    let output = run(&[
        "apply",
        "--request",
        request_path.to_str().unwrap(),
        "--artifact-dir",
        artifact_dir.to_str().unwrap(),
        "--target-dir",
        target_dir.to_str().unwrap(),
        "--journal",
        journal_dir.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!target_dir.join("fix.dll").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("dry run"));
}

#[test]
fn apply_and_revert_round_trip() {
    let dir = tempdir().expect("temp dir");
    let request = request(ArtifactKind::Dll, b"new file");
    let request_path = write_request(dir.path(), &request);
    let artifact_dir = dir.path().join("artifacts");
    let target_dir = dir.path().join("target");
    let journal_dir = dir.path().join("journal");
    fs::create_dir_all(&artifact_dir).expect("artifact dir");
    fs::create_dir_all(&target_dir).expect("target dir");
    fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");
    fs::write(target_dir.join("fix.dll"), b"old file").expect("old file writes");

    let apply = run(&[
        "apply",
        "--request",
        request_path.to_str().unwrap(),
        "--artifact-dir",
        artifact_dir.to_str().unwrap(),
        "--target-dir",
        target_dir.to_str().unwrap(),
        "--journal",
        journal_dir.to_str().unwrap(),
        "--confirm",
    ]);

    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert_eq!(
        fs::read(target_dir.join("fix.dll")).expect("target reads"),
        b"new file"
    );
    assert!(journal_dir.join(agent::executor::JOURNAL_FILE).is_file());

    let revert = run(&[
        "revert",
        "--journal",
        journal_dir.to_str().unwrap(),
        "--confirm",
    ]);

    assert!(
        revert.status.success(),
        "{}",
        String::from_utf8_lossy(&revert.stderr)
    );
    assert_eq!(
        fs::read(target_dir.join("fix.dll")).expect("target reads"),
        b"old file"
    );
}

#[test]
fn revert_without_confirm_keeps_the_new_file() {
    let dir = tempdir().expect("temp dir");
    let request = request(ArtifactKind::Dll, b"new file");
    let request_path = write_request(dir.path(), &request);
    let artifact_dir = dir.path().join("artifacts");
    let target_dir = dir.path().join("target");
    let journal_dir = dir.path().join("journal");
    fs::create_dir_all(&artifact_dir).expect("artifact dir");
    fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");

    let apply = run(&[
        "apply",
        "--request",
        request_path.to_str().unwrap(),
        "--artifact-dir",
        artifact_dir.to_str().unwrap(),
        "--target-dir",
        target_dir.to_str().unwrap(),
        "--journal",
        journal_dir.to_str().unwrap(),
        "--confirm",
    ]);

    assert!(apply.status.success());

    let revert = run(&["revert", "--journal", journal_dir.to_str().unwrap()]);

    assert!(
        revert.status.success(),
        "{}",
        String::from_utf8_lossy(&revert.stderr)
    );
    assert!(target_dir.join("fix.dll").is_file());
}

#[test]
fn apply_rejects_a_digest_mismatch() {
    let dir = tempdir().expect("temp dir");
    let mut request = request(ArtifactKind::Dll, b"new file");
    request.artifacts[0].sha256 = "0".repeat(64);
    let request_path = write_request(dir.path(), &request);
    let artifact_dir = dir.path().join("artifacts");
    let target_dir = dir.path().join("target");
    let journal_dir = dir.path().join("journal");
    fs::create_dir_all(&artifact_dir).expect("artifact dir");
    fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");

    let output = run(&[
        "apply",
        "--request",
        request_path.to_str().unwrap(),
        "--artifact-dir",
        artifact_dir.to_str().unwrap(),
        "--target-dir",
        target_dir.to_str().unwrap(),
        "--journal",
        journal_dir.to_str().unwrap(),
        "--confirm",
    ]);

    assert!(!output.status.success());
    assert!(!target_dir.join("fix.dll").exists());
}

#[test]
fn apply_rejects_a_package_artifact() {
    let dir = tempdir().expect("temp dir");
    let request = request(ArtifactKind::Msu, b"new file");
    let request_path = write_request(dir.path(), &request);
    let artifact_dir = dir.path().join("artifacts");
    let target_dir = dir.path().join("target");
    let journal_dir = dir.path().join("journal");
    fs::create_dir_all(&artifact_dir).expect("artifact dir");
    fs::write(artifact_dir.join("fix.dll"), b"new file").expect("artifact writes");

    let output = run(&[
        "apply",
        "--request",
        request_path.to_str().unwrap(),
        "--artifact-dir",
        artifact_dir.to_str().unwrap(),
        "--target-dir",
        target_dir.to_str().unwrap(),
        "--journal",
        journal_dir.to_str().unwrap(),
        "--confirm",
    ]);

    assert!(!output.status.success());
    assert!(!target_dir.join("fix.dll").exists());
}
