use catalog_schema::SchemaVersion;
use serde_json::json;
use ui_contract::{
    CatalogPayload, ContractError, DiagnosticsPayload, WindowsVersionPayload,
    CURRENT_CONTRACT_MAJOR,
};

const DIAGNOSTICS: &str = include_str!("../fixtures/diagnostics.json");
const CATALOG: &str = include_str!("../fixtures/catalog.json");

#[test]
fn diagnostics_fixture_parses_and_validates() {
    let payload = DiagnosticsPayload::from_json(DIAGNOSTICS).expect("diagnostics fixture parses");

    payload.validate().expect("diagnostics fixture is valid");

    assert_eq!(payload.contract_version.major, CURRENT_CONTRACT_MAJOR);
    assert_eq!(payload.agent_version, "0.1.0");
    assert_eq!(payload.os, "windows");
    assert_eq!(payload.architecture, "x86_64");
    assert_eq!(
        payload.windows,
        Some(WindowsVersionPayload {
            product_name: "Windows 10 Pro".to_string(),
            display_version: "22H2".to_string(),
            build: "19045".to_string(),
            update_build_revision: Some(6466),
        })
    );
}

#[test]
fn diagnostics_fixture_round_trips() {
    let payload = DiagnosticsPayload::from_json(DIAGNOSTICS).expect("diagnostics fixture parses");
    let encoded = payload.to_json().expect("diagnostics serialize");
    let decoded = DiagnosticsPayload::from_json(&encoded).expect("diagnostics parse");

    assert_eq!(decoded, payload);
}

#[test]
fn diagnostics_rejects_an_unsupported_contract_version() {
    let mut payload =
        DiagnosticsPayload::from_json(DIAGNOSTICS).expect("diagnostics fixture parses");
    payload.contract_version = SchemaVersion {
        major: CURRENT_CONTRACT_MAJOR + 1,
        minor: 0,
        patch: 0,
    };

    assert!(matches!(
        payload.validate(),
        Err(ContractError::Schema(
            catalog_schema::SchemaError::UnsupportedVersion { .. }
        ))
    ));
}

#[test]
fn diagnostics_rejects_empty_required_fields() {
    let payload = DiagnosticsPayload::new("", "windows", "x86_64", None);

    assert!(matches!(
        payload.validate(),
        Err(ContractError::EmptyField {
            field: "agent_version"
        })
    ));
}

#[test]
fn diagnostics_rejects_empty_windows_fields() {
    let payload = DiagnosticsPayload::new(
        "0.1.0",
        "windows",
        "x86_64",
        Some(WindowsVersionPayload {
            product_name: "Windows 10 Pro".to_string(),
            display_version: String::new(),
            build: "19045".to_string(),
            update_build_revision: None,
        }),
    );

    assert!(matches!(
        payload.validate(),
        Err(ContractError::EmptyField {
            field: "windows.display_version"
        })
    ));
}

#[test]
fn catalog_fixture_parses_and_validates() {
    let payload = CatalogPayload::from_json(CATALOG).expect("catalog fixture parses");

    payload.validate().expect("catalog fixture is valid");

    assert_eq!(payload.contract_version.major, CURRENT_CONTRACT_MAJOR);
    assert_eq!(payload.target_builds.len(), 1);
    assert_eq!(payload.patches.len(), 1);
    assert_eq!(payload.compatibility.len(), 2);
    assert_eq!(payload.target_builds[0].id, "win10-22h2-x64");
    assert_eq!(payload.patches[0].id, "kb-test-0001");
    assert_eq!(
        payload.compatibility[0].id,
        "software-dotnet-framework-4-8-1"
    );
}

#[test]
fn catalog_fixture_round_trips() {
    let payload = CatalogPayload::from_json(CATALOG).expect("catalog fixture parses");
    let encoded = payload.to_json().expect("catalog serialize");
    let decoded = CatalogPayload::from_json(&encoded).expect("catalog parses");

    assert_eq!(decoded, payload);
}

#[test]
fn catalog_rejects_an_unsupported_contract_version() {
    let mut payload = CatalogPayload::from_json(CATALOG).expect("catalog fixture parses");
    payload.contract_version.major = CURRENT_CONTRACT_MAJOR + 1;

    assert!(matches!(
        payload.validate(),
        Err(ContractError::Schema(
            catalog_schema::SchemaError::UnsupportedVersion { .. }
        ))
    ));
}

#[test]
fn catalog_rejects_invalid_entries() {
    let mut payload = CatalogPayload::from_json(CATALOG).expect("catalog fixture parses");
    payload.patches[0].cves.clear();

    assert!(matches!(
        payload.validate(),
        Err(ContractError::Schema(
            catalog_schema::SchemaError::EmptyField { field: "cves" }
        ))
    ));
}

#[test]
fn catalog_rejects_invalid_compatibility_entries() {
    let mut payload = CatalogPayload::from_json(CATALOG).expect("catalog fixture parses");
    payload.compatibility[0].sources.clear();

    assert!(matches!(
        payload.validate(),
        Err(ContractError::Compatibility(_))
    ));
}

#[test]
fn malformed_json_is_reported_as_an_encoding_error() {
    let error = DiagnosticsPayload::from_json("{").expect_err("payload is malformed");

    assert!(matches!(error, ContractError::Encoding { .. }));
}

#[test]
fn catalog_payload_can_be_built_from_entries() {
    let payload = CatalogPayload::new(Vec::new(), Vec::new(), Vec::new());
    let encoded = payload.to_json().expect("empty catalog serializes");
    let decoded = CatalogPayload::from_json(&encoded).expect("empty catalog parses");

    assert_eq!(decoded, payload);
}

#[test]
fn empty_catalog_serializes_an_empty_contract_version() {
    let payload = CatalogPayload::new(Vec::new(), Vec::new(), Vec::new());
    let value: serde_json::Value =
        serde_json::from_str(&payload.to_json().expect("catalog serializes"))
            .expect("catalog JSON parses");

    assert_eq!(
        value["contract_version"],
        json!({
            "major": 1,
            "minor": 0,
            "patch": 0
        })
    );
}
