use compatibility_catalog::{
    CompatibilityEntry, CompatibilityError, CompatibilityKind, CompatibilityStatus,
};
use serde::de::DeserializeOwned;

const VALID_SOFTWARE: &str = include_str!("fixtures/valid/software-dotnet-framework-4-8-1.json");
const VALID_DRIVER: &str = include_str!("fixtures/valid/driver-example-network.json");

const DRIVER_WITHOUT_HARDWARE: &str =
    include_str!("fixtures/invalid/driver-without-hardware-ids.json");
const DRIVER_WITHOUT_VERSION: &str = include_str!("fixtures/invalid/driver-without-version.json");
const EMPTY_TESTED_BUILDS: &str = include_str!("fixtures/invalid/empty-tested-builds.json");
const INSECURE_SOURCE: &str = include_str!("fixtures/invalid/insecure-source.json");
const NO_SOURCES: &str = include_str!("fixtures/invalid/no-sources.json");
const INVALID_DATE: &str = include_str!("fixtures/invalid/invalid-date.json");
const UNKNOWN_KIND: &str = include_str!("fixtures/invalid/unknown-kind.json");
const INVALID_ARCHITECTURE: &str = include_str!("fixtures/invalid/invalid-architecture.json");
const EMPTY_NAME: &str = include_str!("fixtures/invalid/empty-name.json");
const UNSUPPORTED_VERSION: &str = include_str!("fixtures/invalid/unsupported-version.json");

fn parse<T: DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).expect("fixture parses")
}

#[test]
fn valid_software_fixture_parses_and_validates() {
    let entry: CompatibilityEntry = parse(VALID_SOFTWARE);

    entry.validate().expect("software entry is valid");

    assert_eq!(entry.kind, CompatibilityKind::Software);
    assert_eq!(entry.status, CompatibilityStatus::Experimental);
    assert_eq!(entry.sources.len(), 2);
    assert_eq!(entry.version.as_deref(), Some("4.8.1"));
}

#[test]
fn valid_driver_fixture_parses_and_validates() {
    let entry: CompatibilityEntry = parse(VALID_DRIVER);

    entry.validate().expect("driver entry is valid");

    assert_eq!(entry.kind, CompatibilityKind::Driver);
    assert_eq!(entry.driver_version.as_deref(), Some("1.0.0.0"));
    assert_eq!(entry.hardware_ids.len(), 1);
}

#[test]
fn compatibility_kind_and_status_serialize_in_lowercase() {
    assert_eq!(
        serde_json::to_string(&CompatibilityKind::Driver).expect("kind serializes"),
        "\"driver\""
    );
    assert_eq!(
        serde_json::to_string(&CompatibilityStatus::Validated).expect("status serializes"),
        "\"validated\""
    );
}

#[test]
fn entry_round_trips_through_json() {
    let entry: CompatibilityEntry = parse(VALID_SOFTWARE);
    let encoded = serde_json::to_string(&entry).expect("entry serializes");
    let decoded: CompatibilityEntry = serde_json::from_str(&encoded).expect("entry parses");

    assert_eq!(decoded, entry);
}

#[test]
fn driver_without_hardware_ids_is_rejected() {
    let entry: CompatibilityEntry = parse(DRIVER_WITHOUT_HARDWARE);
    let error = entry.validate().expect_err("hardware IDs are required");

    assert_eq!(error, CompatibilityError::MissingHardwareIds);
}

#[test]
fn driver_without_version_is_rejected() {
    let entry: CompatibilityEntry = parse(DRIVER_WITHOUT_VERSION);
    let error = entry.validate().expect_err("driver version is required");

    assert_eq!(error, CompatibilityError::MissingDriverVersion);
}

#[test]
fn empty_tested_builds_are_rejected() {
    let entry: CompatibilityEntry = parse(EMPTY_TESTED_BUILDS);
    let error = entry.validate().expect_err("tested builds are required");

    assert_eq!(
        error,
        CompatibilityError::EmptyField {
            field: "tested_builds"
        }
    );
}

#[test]
fn insecure_source_is_rejected() {
    let entry: CompatibilityEntry = parse(INSECURE_SOURCE);
    let error = entry.validate().expect_err("HTTP source must fail");

    assert!(matches!(error, CompatibilityError::InsecureSource { .. }));
}

#[test]
fn entry_without_sources_is_rejected() {
    let entry: CompatibilityEntry = parse(NO_SOURCES);
    let error = entry.validate().expect_err("sources are required");

    assert_eq!(error, CompatibilityError::EmptyField { field: "sources" });
}

#[test]
fn invalid_source_date_is_rejected() {
    let entry: CompatibilityEntry = parse(INVALID_DATE);
    let error = entry.validate().expect_err("invalid date must fail");

    assert!(matches!(error, CompatibilityError::InvalidDate { .. }));
}

#[test]
fn unknown_kind_is_rejected() {
    let result = serde_json::from_str::<CompatibilityEntry>(UNKNOWN_KIND);

    assert!(result.is_err());
}

#[test]
fn unsupported_architecture_is_rejected() {
    let result = serde_json::from_str::<CompatibilityEntry>(INVALID_ARCHITECTURE);

    assert!(result.is_err());
}

#[test]
fn empty_name_is_rejected() {
    let entry: CompatibilityEntry = parse(EMPTY_NAME);
    let error = entry.validate().expect_err("name is required");

    assert_eq!(error, CompatibilityError::EmptyField { field: "name" });
}

#[test]
fn unsupported_schema_version_is_rejected() {
    let entry: CompatibilityEntry = parse(UNSUPPORTED_VERSION);
    let error = entry.validate().expect_err("version must fail");

    assert!(matches!(error, CompatibilityError::Schema(_)));
}
