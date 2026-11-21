//! The Start-menu search repair is a declarative AppX re-registration, not a
//! file replacement. These tests pin down the runtime behaviour: detect a
//! degraded registration, repair it once, and leave a healthy one untouched.

use patch_runtime::{MemoryBackend, PatchRuntime, PatchState};
use std::path::PathBuf;

const PATCH_ID: &str = "start-menu-search-repair";
const PACKAGE: &str = "Microsoft.Windows.Search";

struct Fixture {
    dir: PathBuf,
    runtime: PatchRuntime<MemoryBackend>,
}

impl Fixture {
    fn new(patches_dir: &PathBuf) -> Self {
        let mut backend = MemoryBackend::new();
        backend.journal_dir = patches_dir.join("journal");
        let runtime = PatchRuntime::new(backend, patches_dir.clone());
        Self {
            dir: patches_dir.clone(),
            runtime,
        }
    }

    fn degrade(&self) {
        self.runtime
            .backend
            .appx_registered
            .lock()
            .unwrap()
            .clear();
    }
}

fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!(
        "ll10-appx-repair-{}-{}",
        PATCH_ID,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let patch_dir = dir.join(PATCH_ID);
    std::fs::create_dir_all(&patch_dir).unwrap();
    std::fs::write(
        patch_dir.join("patch.yml"),
        format!(
            r#"schema: 1
id: {id}
title: Start menu search repair
severity: medium
target: guest
requires:
  admin: true
  reboot: false
docs:
  readme: README.md
applies_to:
  os: windows
  arch: [x86_64]
  builds: ">=19045.0 <19046.0"
checks:
  - type: appx_package_registration
    id: search-appx
    package_family_name: "{package}"
    expect_registered: true
operations:
  - type: re_register_appx
    id: repair-search-appx
    package_family_name: "{package}"
    manifest_path: "C:\\Windows\\SystemApps\\Microsoft.Windows.Search_cw5n1h2txyewy\\AppxManifest.xml"
"#,
            id = PATCH_ID,
            package = PACKAGE
        ),
    )
    .unwrap();
    Fixture::new(&dir)
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn degraded_registration_is_detected_and_repaired() {
    let f = fixture();
    f.degrade();

    // No journal + failing check: the repair has not been applied yet.
    assert_eq!(f.runtime.verify(PATCH_ID).unwrap(), PatchState::NotApplied);

    let report = f.runtime.apply(PATCH_ID, true).unwrap();
    assert_eq!(report.state, PatchState::Applied);
    assert_eq!(
        f.runtime.verify(PATCH_ID).unwrap(),
        PatchState::Applied,
        "after re-registration the check must pass"
    );
}

#[test]
fn applying_a_healthy_registration_is_a_no_op() {
    let f = fixture();
    // Apply once to create the journal and repair the registration.
    let first = f.runtime.apply(PATCH_ID, true).unwrap();
    assert_eq!(first.state, PatchState::Applied);

    // Re-applying must not change a healthy registration and must stay applied.
    let second = f.runtime.apply(PATCH_ID, true).unwrap();
    assert_eq!(second.state, PatchState::Applied);
    assert_eq!(
        f.runtime.verify(PATCH_ID).unwrap(),
        PatchState::Applied
    );
}

#[test]
fn registration_state_is_keyed_by_package_name() {
    use patch_runtime::SystemBackend;

    let f = fixture();
    f.runtime.apply(PATCH_ID, true).unwrap();

    // O pacote declarado está registrado depois do reparo.
    assert!(f
        .runtime
        .backend
        .appx_package_registered(PACKAGE)
        .unwrap());

    // Outro pacote não pode herdar o estado: o backend real faria essa
    // distinção, e um nome errado no manifesto deve falhar no teste.
    assert!(!f
        .runtime
        .backend
        .appx_package_registered("Microsoft.Windows.NotSearch")
        .unwrap());
}
