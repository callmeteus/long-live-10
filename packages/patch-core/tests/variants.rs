use patch_core::{PatchManifest, VariantSelection};

const STOCK_A: &str = "03b7a32454ae46a26511ad8933ba9026af099976b9e896f5251615e196e83cf8";
const STOCK_B: &str = "936c33a8283196a070aa6f844a65a29a8f99f760eae0016754c1cbf17d62bdb7";
const PATCHED_A: &str = "805c0ddc845c28a20f6b1d053e5260c670c4511d4103070a28fc634660f70a23";
const PATCHED_B: &str = "66049f127566ef448326dbcb52ffb51c25a057637a9c3cb4930b0cd2af9eed09";

fn manifest(build_variants: &str, preconditions: &str, auto: &str) -> PatchManifest {
    let yaml = format!(
        r#"
schema: 1
id: test
title: test
severity: high
target: guest
requires: {{ admin: false, reboot: false }}
docs: {{ readme: README.md }}
applies_to:
  os: windows
  builds: ">=19045.0 <19046.0"
artifacts:
  - id: a
    file: artifacts/a.dll
    sha256: {PATCHED_A}
    target_path: C:\Windows\System32\shell32.dll
  - id: b
    file: artifacts/b.dll
    sha256: {PATCHED_B}
    target_path: C:\Windows\System32\shell32.dll
{build_variants}
{preconditions}
operations:
  - type: stage_file
    id: stage
    artifact: "$selected"
    to: C:\Windows\Temp\staged.dll
checks:
  - type: file_sha256
    id: check
    path: C:\Windows\System32\shell32.dll
    equals: "$selected"
{auto}
"#
    );
    serde_norway::from_str(&yaml).expect("manifest parses")
}

fn variants() -> String {
    format!(
        "build_variants:\n  - stock_sha256: {STOCK_A}\n    artifact: a\n  - stock_sha256: {STOCK_B}\n    artifact: b\n"
    )
}

#[test]
fn each_stock_build_selects_its_own_artifact() {
    let parsed = manifest(&variants(), "", "");
    parsed.validate().expect("manifest is valid");

    assert_eq!(
        parsed.select_variant(STOCK_A),
        VariantSelection::Matched("a".to_string())
    );
    assert_eq!(
        parsed.select_variant(STOCK_B),
        VariantSelection::Matched("b".to_string())
    );
}

#[test]
fn a_patched_file_is_reported_as_already_applied() {
    let parsed = manifest(&variants(), "", "");
    assert_eq!(
        parsed.select_variant(PATCHED_B),
        VariantSelection::AlreadyApplied("b".to_string())
    );
}

#[test]
fn an_unknown_build_is_refused() {
    let parsed = manifest(&variants(), "", "");
    assert_eq!(
        parsed.select_variant("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        VariantSelection::Unsupported
    );
}

#[test]
fn variants_and_preconditions_cannot_be_combined() {
    let preconditions = format!(
        "preconditions:\n  - type: file_sha256\n    id: pre\n    path: C:\\Windows\\System32\\shell32.dll\n    one_of:\n      - {STOCK_A}\n"
    );
    let parsed = manifest(&variants(), &preconditions, "");
    assert!(parsed.validate().is_err(), "combination must be rejected");
}

#[test]
fn variant_must_reference_a_declared_artifact() {
    let bad = format!(
        "build_variants:\n  - stock_sha256: {STOCK_A}\n    artifact: missing\n"
    );
    let parsed = manifest(&bad, "", "");
    assert!(parsed.validate().is_err(), "unknown artifact must be rejected");
}

#[test]
fn auto_remediation_accepts_a_variant_target() {
    // The variants declare target_path, which is a valid watch target.
    let auto = "auto_remediate:\n  enabled: true\n";
    let parsed = manifest(&variants(), "", auto);
    parsed
        .validate()
        .expect("enabled auto-remediation with a variant target is valid");
}

#[test]
fn auto_remediation_without_a_watch_target_is_rejected() {
    // No variants and no watch_paths: nothing to watch.
    let auto = "auto_remediate:\n  enabled: true\n";
    let parsed = manifest("", "", auto);
    assert!(parsed.validate().is_err(), "must require a watch target");
}

#[test]
fn auto_remediation_with_watch_paths_is_valid() {
    let auto = "auto_remediate:\n  enabled: true\n  watch_paths:\n    - C:\\Windows\\System32\\shell32.dll\n";
    let parsed = manifest(&variants(), "", auto);
    parsed.validate().expect("watch_paths is enough");
}
