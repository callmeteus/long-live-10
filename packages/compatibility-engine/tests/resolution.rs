use catalog_schema::Architecture;
use compatibility_catalog::{
    CompatibilityEntry, CompatibilityKind, CompatibilityStatus, SourceReference,
};
use compatibility_engine::{
    versions_match, CheckStatus, CompatibilityEngine, GuestProfile, RequirementKind,
    ResolutionStatus,
};
use std::collections::HashMap;

fn software_entry() -> CompatibilityEntry {
    serde_json::from_str(include_str!(
        "../../compatibility-catalog/entries/software/dotnet-framework-4-8-1.json"
    ))
    .expect("software entry parses")
}

fn driver_entry() -> CompatibilityEntry {
    serde_json::from_str(include_str!(
        "../../compatibility-catalog/tests/fixtures/valid/driver-example-network.json"
    ))
    .expect("driver entry parses")
}

fn profile() -> GuestProfile {
    let mut profile = GuestProfile::new("win10-22h2-x64");
    profile.architecture = Some(Architecture::X64);

    profile
}

#[test]
fn resolves_installed_software_with_matching_version() {
    let mut profile = profile();
    profile
        .software_versions
        .insert(software_entry().id, "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
    assert!(resolution
        .checks
        .iter()
        .any(|check| check.kind == RequirementKind::SoftwareVersion
            && check.status == CheckStatus::Satisfied));
}

#[test]
fn rejects_missing_software() {
    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile());

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Software && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn rejects_software_version_mismatch() {
    let mut profile = profile();
    profile
        .software_versions
        .insert(software_entry().id, "4.7.2".to_string());

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::SoftwareVersion && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn resolves_wildcard_software_version() {
    let mut entry = software_entry();
    entry.version = Some("4.8.x".to_string());

    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}

#[test]
fn resolves_driver_with_matching_hardware_and_version() {
    let entry = driver_entry();
    let hardware_id = entry.hardware_ids[0].clone();
    let mut profile = profile();
    profile.hardware_ids.insert(hardware_id.clone());
    profile
        .driver_versions
        .insert(hardware_id, "1.0.0.0".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}

#[test]
fn rejects_driver_when_hardware_is_missing() {
    let resolution = CompatibilityEngine::new().resolve(&driver_entry(), &profile());

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::HardwareId && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn rejects_driver_version_mismatch() {
    let entry = driver_entry();
    let hardware_id = entry.hardware_ids[0].clone();
    let mut profile = profile();
    profile.hardware_ids.insert(hardware_id.clone());
    profile
        .driver_versions
        .insert(hardware_id, "2.0.0.0".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::DriverVersion && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn refuses_driver_when_version_is_unknown() {
    let entry = driver_entry();
    let hardware_id = entry.hardware_ids[0].clone();
    let mut profile = profile();
    profile.hardware_ids.insert(hardware_id);

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Refused);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::DriverVersion && check.status == CheckStatus::Undetermined
    }));
}

#[test]
fn resolves_device_with_matching_hardware() {
    let mut entry = driver_entry();
    entry.kind = CompatibilityKind::Device;
    entry.driver_version = None;
    let hardware_id = entry.hardware_ids[0].clone();
    let mut profile = profile();
    profile.hardware_ids.insert(hardware_id);

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}

#[test]
fn rejects_build_mismatch() {
    let mut profile = profile();
    profile.build = "win10-21h2-x64".to_string();

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Build && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn rejects_architecture_mismatch() {
    let mut profile = profile();
    profile.architecture = Some(Architecture::X86);

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Architecture && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn refuses_when_architecture_is_unknown() {
    let mut profile = profile();
    profile.architecture = None;
    profile
        .software_versions
        .insert(software_entry().id, "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Refused);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Architecture && check.status == CheckStatus::Undetermined
    }));
}

#[test]
fn rejects_missing_dependency() {
    let mut entry = software_entry();
    entry.dependencies = vec!["software-runtime".to_string()];

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile());

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Dependency && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn resolves_present_dependency() {
    let mut entry = software_entry();
    entry.dependencies = vec!["software-runtime".to_string()];
    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "4.8.1".to_string());
    profile
        .applied_entries
        .insert("software-runtime".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}

#[test]
fn refuses_an_invalid_entry() {
    let mut entry = software_entry();
    entry.sources = Vec::<SourceReference>::new();

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile());

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Contract && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn rejects_a_discontinued_entry() {
    let mut entry = software_entry();
    entry.status = CompatibilityStatus::Discontinued;
    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Incompatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Lifecycle && check.status == CheckStatus::Unsatisfied
    }));
}

#[test]
fn marks_an_experimental_entry_as_compatible_but_not_validated() {
    let mut entry = software_entry();
    entry.status = CompatibilityStatus::Experimental;
    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
    assert!(resolution.checks.iter().any(|check| {
        check.kind == RequirementKind::Lifecycle
            && check.status == CheckStatus::Satisfied
            && check.detail.contains("experimental")
    }));
}

#[test]
fn resolves_the_whole_catalog() {
    let entries = vec![software_entry(), driver_entry()];

    let resolutions = CompatibilityEngine::new().resolve_catalog(&entries, &profile());

    assert_eq!(resolutions.len(), 2);
    assert_eq!(resolutions[0].entry_id, "software-dotnet-framework-4-8-1");
    assert_eq!(resolutions[1].entry_id, "driver-example-network");
}

#[test]
fn hardware_ids_match_ignoring_case() {
    let mut profile = GuestProfile::new("win10-22h2-x64");
    profile
        .hardware_ids
        .insert("pci\\ven_1234&dev_5678".to_string());

    assert!(profile.has_hardware_id("PCI\\VEN_1234&DEV_5678"));
}

#[test]
fn versions_match_numeric_padding() {
    assert!(versions_match("1.0.0.0", "1.0.0"));
    assert!(!versions_match("1.0.1.0", "1.0.0"));
}

#[test]
fn versions_match_wildcards() {
    assert!(versions_match("14.44.35211", "14.x"));
    assert!(versions_match("14.44.35211", "14.*"));
    assert!(!versions_match("13.0.0", "14.x"));
}

#[test]
fn versions_match_exact_strings_when_not_numeric() {
    assert!(versions_match("en-US", "en-US"));
    assert!(!versions_match("en-US", "pt-BR"));
}

#[test]
fn resolution_copies_issues_and_limitations() {
    let entry = software_entry();
    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "4.8.1".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.known_issues, entry.known_issues);
    assert_eq!(resolution.limitations, entry.limitations);
}

#[test]
fn software_presence_without_version_is_enough() {
    let mut entry = software_entry();
    entry.version = None;
    let mut profile = profile();
    profile
        .software_versions
        .insert(entry.id.clone(), "unknown".to_string());

    let resolution = CompatibilityEngine::new().resolve(&entry, &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}

#[test]
fn software_version_map_can_include_other_entries() {
    let mut profile = profile();
    profile.software_versions = HashMap::from([
        (
            "software-dotnet-framework-4-8-1".to_string(),
            "4.8.1".to_string(),
        ),
        (
            "software-visual-cpp-redistributable-2015-2022".to_string(),
            "14.44.35211".to_string(),
        ),
    ]);

    let resolution = CompatibilityEngine::new().resolve(&software_entry(), &profile);

    assert_eq!(resolution.status, ResolutionStatus::Compatible);
}
