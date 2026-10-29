//! A journal records what a patch changed; it is not proof that the machine is
//! still in that state. SFC and servicing updates restore the stock file
//! without touching the journal, so `apply` has to decide from the file hash.
//!
//! These tests reproduce that drift and pin down the two behaviours that keep
//! auto-remediation safe: repairable drift is re-applied, unknown content is
//! never overwritten.

use patch_runtime::backend::MemoryBackend;
use patch_runtime::{PatchRuntime, PatchState};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const TARGET: &str = r"C:\Windows\System32\drift.dll";
const STOCK: &[u8] = b"stock shell32 bytes";
const PATCHED: &[u8] = b"patched shell32 bytes";
const UNKNOWN: &[u8] = b"shell32 from a build we know nothing about";

struct Fixture {
    dir: PathBuf,
    runtime: PatchRuntime<MemoryBackend>,
}

impl Fixture {
    fn new(patch_id: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ll10-drift-{}-{}", patch_id, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let patch_dir = dir.join(patch_id);
        std::fs::create_dir_all(patch_dir.join("artifacts")).unwrap();

        std::fs::write(patch_dir.join("artifacts/patched.bin"), PATCHED).unwrap();
        std::fs::write(patch_dir.join("README.md"), "test fixture").unwrap();
        std::fs::write(patch_dir.join("patch.yml"), manifest(patch_id)).unwrap();

        let mut backend = MemoryBackend::new();
        backend.build = "19045.2965".to_string();
        backend.journal_dir = dir.join("journal");
        backend
            .files
            .lock()
            .unwrap()
            .insert(TARGET.to_string(), STOCK.to_vec());

        let runtime = PatchRuntime::new(backend, dir.clone());
        Self { dir, runtime }
    }

    fn content(&self) -> Vec<u8> {
        self.runtime
            .backend
            .files
            .lock()
            .unwrap()
            .get(TARGET)
            .cloned()
            .unwrap_or_default()
    }

    /// Puts whatever bytes we want on disk, the way SFC or a servicing update
    /// would, without touching the journal.
    fn overwrite(&self, bytes: &[u8]) {
        self.runtime
            .backend
            .files
            .lock()
            .unwrap()
            .insert(TARGET.to_string(), bytes.to_vec());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest(id: &str) -> String {
    format!(
        r#"schema: 1
id: {id}
title: Drift test
severity: low
target: guest
requires:
  admin: true
  reboot: false
docs:
  readme: README.md
applies_to:
  os: windows
  builds: ">=19045.0 <19046.0"
artifacts:
  - id: patched
    file: artifacts/patched.bin
    sha256: {patched}
    target_path: "C:\\Windows\\System32\\drift.dll"
build_variants:
  - stock_sha256: {stock}
    artifact: patched
operations:
  - type: backup_file
    id: backup
    path: "C:\\Windows\\System32\\drift.dll"
  - type: stage_file
    id: stage
    artifact: "$selected"
    to: "C:\\ProgramData\\LongLive10\\staging\\drift.dll"
  - type: replace_file
    id: replace
    from: stage
    path: "C:\\Windows\\System32\\drift.dll"
    mode: direct
checks:
  - type: file_sha256
    id: verify
    path: "C:\\Windows\\System32\\drift.dll"
    equals: "$selected"
"#,
        id = id,
        patched = sha(PATCHED),
        stock = sha(STOCK),
    )
}

#[test]
fn a_restored_stock_file_is_patched_again() {
    let f = Fixture::new("drift-reapplied");

    let first = f.runtime.apply("drift-reapplied", true).unwrap();
    assert_eq!(first.state, PatchState::Applied);
    assert_eq!(f.content(), PATCHED);

    // SFC repairs the file from the component manifest, which describes the
    // stock binary. The journal is untouched.
    f.overwrite(STOCK);
    assert_eq!(
        f.runtime.verify("drift-reapplied").unwrap(),
        PatchState::Drifted,
        "the journal must not hide drift"
    );

    let second = f.runtime.apply("drift-reapplied", true).unwrap();
    assert_eq!(second.state, PatchState::Applied, "drift must be re-applied");
    assert_eq!(f.content(), PATCHED);
    assert_eq!(
        f.runtime.verify("drift-reapplied").unwrap(),
        PatchState::Applied
    );
}

#[test]
fn reverting_after_drift_restores_the_original_file() {
    let f = Fixture::new("drift-revert");

    f.runtime.apply("drift-revert", true).unwrap();
    f.overwrite(STOCK);
    f.runtime.apply("drift-revert", true).unwrap();

    f.runtime.revert("drift-revert", true).unwrap();
    assert_eq!(
        f.content(),
        STOCK,
        "revert after a repair must still put the original back"
    );
}

#[test]
fn an_unknown_file_is_never_overwritten() {
    let f = Fixture::new("drift-unknown");

    f.runtime.apply("drift-unknown", true).unwrap();

    // A Windows update moved the machine to a build this patch has no artifact
    // for. Writing the old binary over it would stop the machine booting.
    f.overwrite(UNKNOWN);

    let report = f.runtime.apply("drift-unknown", true).unwrap();
    assert_eq!(report.state, PatchState::NotApplicable);
    assert_eq!(
        f.content(),
        UNKNOWN,
        "an unsupported file must be left exactly as it was"
    );
}

#[test]
fn applying_twice_does_not_stack_changes() {
    let f = Fixture::new("drift-twice");

    f.runtime.apply("drift-twice", true).unwrap();
    let second = f.runtime.apply("drift-twice", true).unwrap();

    assert_eq!(second.state, PatchState::Applied);
    assert_eq!(second.message, "already applied");
    assert_eq!(f.content(), PATCHED);
}

#[test]
fn remediation_repairs_drift() {
    let f = Fixture::new("drift-remediate");

    f.runtime.apply("drift-remediate", true).unwrap();
    f.overwrite(STOCK);

    let report = f.runtime.remediate("drift-remediate").unwrap();
    assert_eq!(
        report.expect("a patched journal must be repaired").state,
        PatchState::Applied
    );
    assert_eq!(f.content(), PATCHED);
}

#[test]
fn remediation_refuses_to_undo_a_revert() {
    let f = Fixture::new("drift-reverted");

    f.runtime.apply("drift-reverted", true).unwrap();
    f.runtime.revert("drift-reverted", true).unwrap();

    assert!(
        f.runtime.remediate("drift-reverted").unwrap().is_none(),
        "an explicit revert must not be undone by a stray event"
    );
    assert_eq!(f.content(), STOCK);
}

#[test]
fn remediation_does_nothing_without_a_journal() {
    let f = Fixture::new("drift-no-journal");

    assert!(
        f.runtime.remediate("drift-no-journal").unwrap().is_none(),
        "nothing is installed, so there is nothing to repair"
    );
    assert_eq!(f.content(), STOCK);
}
