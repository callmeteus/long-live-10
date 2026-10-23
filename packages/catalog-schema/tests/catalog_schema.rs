use catalog_schema::{
    Architecture, Artifact, ArtifactKind, PatchEntry, PatchStatus, SchemaError, SchemaVersion,
    TargetBuild, Validate, ValidationResult, ValidationStatus,
};
use serde::de::DeserializeOwned;

const VALID_PATCH: &str = include_str!("fixtures/valid/patch.json");
const VALID_ARTIFACT: &str = include_str!("fixtures/valid/artifact.json");
const VALID_TARGET_BUILD: &str = include_str!("fixtures/valid/target-build.json");
const VALID_VALIDATION_RESULT: &str = include_str!("fixtures/valid/validation-result.json");

const INVALID_ARTIFACT_BAD_HASH: &str = include_str!("fixtures/invalid/artifact-bad-hash.json");
const INVALID_ARTIFACT_PATH: &str = include_str!("fixtures/invalid/artifact-path.json");
const INVALID_ARTIFACT_HTTP: &str = include_str!("fixtures/invalid/artifact-http-source.json");
const INVALID_PATCH_EMPTY_CVES: &str = include_str!("fixtures/invalid/patch-empty-cves.json");
const INVALID_PATCH_UNKNOWN_STATUS: &str =
    include_str!("fixtures/invalid/patch-unknown-status.json");
const INVALID_PATCH_MISSING_TITLE: &str = include_str!("fixtures/invalid/patch-missing-title.json");
const INVALID_PATCH_VERSION: &str = include_str!("fixtures/invalid/patch-unsupported-version.json");
const INVALID_TARGET_ARCHITECTURE: &str =
    include_str!("fixtures/invalid/target-build-unsupported-architecture.json");
const INVALID_RESULT_PASSED_WITH_FAILURE: &str =
    include_str!("fixtures/invalid/validation-passed-with-failure.json");
const INVALID_RESULT_FAILED_WITHOUT_FAILURE: &str =
    include_str!("fixtures/invalid/validation-failed-without-failure.json");
const INVALID_RESULT_EMPTY_CHECKS: &str =
    include_str!("fixtures/invalid/validation-empty-checks.json");
const INVALID_RESULT_TIME_RANGE: &str = include_str!("fixtures/invalid/validation-time-range.json");

fn parse<T: DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).expect("fixture parses")
}

#[test]
fn valid_patch_fixture_parses_and_validates() {
    let entry: PatchEntry = parse(VALID_PATCH);

    entry.validate().expect("patch is valid");

    assert_eq!(entry.status, PatchStatus::Experimental);
    assert_eq!(entry.cves, vec!["CVE-2026-0001"]);
}

#[test]
fn valid_artifact_fixture_parses_and_validates() {
    let artifact: Artifact = parse(VALID_ARTIFACT);

    artifact.validate().expect("artifact is valid");

    assert_eq!(artifact.kind, ArtifactKind::Msu);
    assert_eq!(artifact.size_bytes, 1_048_576);
}

#[test]
fn valid_target_build_fixture_parses_and_validates() {
    let build: TargetBuild = parse(VALID_TARGET_BUILD);

    build.validate().expect("build is valid");

    assert_eq!(build.architecture, Architecture::X64);
    assert_eq!(build.build, 19045);
    assert_eq!(build.ubr, 6466);
}

#[test]
fn valid_validation_result_fixture_parses_and_validates() {
    let result: ValidationResult = parse(VALID_VALIDATION_RESULT);

    result.validate().expect("result is valid");

    assert_eq!(result.status, ValidationStatus::Passed);
    assert_eq!(result.checks.len(), 2);
}

#[test]
fn patch_status_serializes_in_lowercase() {
    let encoded = serde_json::to_string(&PatchStatus::Validated).expect("status serializes");

    assert_eq!(encoded, "\"validated\"");
}

#[test]
fn architecture_and_artifact_kind_serialize_in_lowercase() {
    assert_eq!(
        serde_json::to_string(&Architecture::X64).expect("architecture serializes"),
        "\"x64\""
    );
    assert_eq!(
        serde_json::to_string(&ArtifactKind::Msu).expect("artifact kind serializes"),
        "\"msu\""
    );
}

#[test]
fn patch_entry_round_trips_through_json() {
    let entry: PatchEntry = parse(VALID_PATCH);
    let encoded = serde_json::to_string(&entry).expect("entry serializes");
    let decoded: PatchEntry = serde_json::from_str(&encoded).expect("entry parses");

    assert_eq!(decoded, entry);
}

#[test]
fn unknown_status_is_rejected() {
    let result = serde_json::from_str::<PatchEntry>(INVALID_PATCH_UNKNOWN_STATUS);

    assert!(result.is_err());
}

#[test]
fn missing_required_field_is_rejected() {
    let result = serde_json::from_str::<PatchEntry>(INVALID_PATCH_MISSING_TITLE);

    assert!(result.is_err());
}

#[test]
fn unsupported_architecture_is_rejected() {
    let result = serde_json::from_str::<TargetBuild>(INVALID_TARGET_ARCHITECTURE);

    assert!(result.is_err());
}

#[test]
fn empty_cves_are_rejected() {
    let entry: PatchEntry = parse(INVALID_PATCH_EMPTY_CVES);
    let error = entry.validate().expect_err("empty CVEs must fail");

    assert_eq!(error, SchemaError::EmptyField { field: "cves" });
}

#[test]
fn unsupported_schema_version_is_rejected() {
    let entry: PatchEntry = parse(INVALID_PATCH_VERSION);
    let error = entry.validate().expect_err("version must fail");

    assert_eq!(
        error,
        SchemaError::UnsupportedVersion {
            major: 2,
            expected: 1
        }
    );
}

#[test]
fn invalid_sha256_is_rejected() {
    let artifact: Artifact = parse(INVALID_ARTIFACT_BAD_HASH);
    let error = artifact.validate().expect_err("hash must fail");

    assert!(matches!(error, SchemaError::InvalidSha256 { .. }));
}

#[test]
fn artifact_path_is_rejected() {
    let artifact: Artifact = parse(INVALID_ARTIFACT_PATH);
    let error = artifact.validate().expect_err("path must fail");

    assert!(matches!(error, SchemaError::InvalidFileName { .. }));
}

#[test]
fn insecure_artifact_source_is_rejected() {
    let artifact: Artifact = parse(INVALID_ARTIFACT_HTTP);
    let error = artifact.validate().expect_err("http must fail");

    assert!(matches!(error, SchemaError::InsecureSource { .. }));
}

#[test]
fn passed_result_with_failed_check_is_rejected() {
    let result: ValidationResult = parse(INVALID_RESULT_PASSED_WITH_FAILURE);
    let error = result
        .validate()
        .expect_err("inconsistent result must fail");

    assert_eq!(
        error,
        SchemaError::PassedWithFailedCheck {
            check: "install".to_string()
        }
    );
}

#[test]
fn failed_result_without_failed_check_is_rejected() {
    let result: ValidationResult = parse(INVALID_RESULT_FAILED_WITHOUT_FAILURE);
    let error = result
        .validate()
        .expect_err("inconsistent result must fail");

    assert_eq!(error, SchemaError::FailedWithoutFailedCheck);
}

#[test]
fn result_without_checks_is_rejected() {
    let result: ValidationResult = parse(INVALID_RESULT_EMPTY_CHECKS);
    let error = result.validate().expect_err("empty checks must fail");

    assert_eq!(error, SchemaError::EmptyChecks);
}

#[test]
fn result_with_reversed_timestamps_is_rejected() {
    let result: ValidationResult = parse(INVALID_RESULT_TIME_RANGE);
    let error = result.validate().expect_err("time range must fail");

    assert_eq!(error, SchemaError::InvalidTimeRange);
}

#[test]
fn current_schema_version_is_valid() {
    SchemaVersion::current()
        .validate()
        .expect("current version is valid");
}
