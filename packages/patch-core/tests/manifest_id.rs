use std::io::Write;

use patch_core::PatchManifest;

fn manifest_with_id(id: &str) -> String {
    format!(
        r#"
schema: 1
id: '{id}'
title: test
severity: high
target: guest
requires:
  admin: true
docs:
  readme: README.md
applies_to:
  os: windows
  arch:
    - x86_64
  builds: ">=19045.0 <19046.0"
"#
    )
}

fn is_valid(manifest: &str) -> bool {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("patch.yml");
    let mut file = std::fs::File::create(&path).expect("manifest file");
    file.write_all(manifest.as_bytes()).expect("write manifest");
    PatchManifest::load_from_path(&path).is_ok()
}

#[test]
fn accepts_an_alphanumeric_id() {
    assert!(is_valid(&manifest_with_id("cve-2026-20816")));
}

#[test]
fn rejects_shell_metacharacters_in_the_id() {
    // The id is interpolated into the auto-remediation command line. A '&'
    // used to separate commands must never be accepted.
    for id in [
        "poc & calc & rem",
        "poc;calc",
        "poc|calc",
        "poc>out",
        "poc<in",
    ] {
        assert!(!is_valid(&manifest_with_id(id)), "id {id:?} should be rejected");
    }
}
