use catalog_schema::{Architecture, PatchEntry, PatchStatus, Validate};
use patch_core::{
    Applicability, ComponentRequirement, ComponentVersion, GuestState, PatchEngine,
    PatchRequirements,
};
use std::collections::{HashMap, HashSet};

fn patch_fixture() -> PatchEntry {
    serde_json::from_str(include_str!(
        "../../catalog-schema/tests/fixtures/valid/patch.json"
    ))
    .expect("patch fixture parses")
}

fn target_fixture() -> catalog_schema::TargetBuild {
    serde_json::from_str(include_str!(
        "../../catalog-schema/tests/fixtures/valid/target-build.json"
    ))
    .expect("target fixture parses")
}

fn guest_state() -> GuestState {
    GuestState {
        build: 19045,
        ubr: Some(6466),
        architecture: Architecture::X64,
        components: HashMap::new(),
        applied_patches: HashSet::new(),
    }
}

#[test]
fn matches_patch_to_fixture_target() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements::default();
    let state = guest_state();

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Applicable
    );
}

#[test]
fn refuses_when_patch_entry_is_invalid() {
    let engine = PatchEngine::new();
    let mut patch = patch_fixture();
    patch.cves.clear();
    let requirements = PatchRequirements::default();
    let state = guest_state();

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn refuses_when_build_does_not_match() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements::default();
    let mut state = guest_state();
    state.build = 19044;

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::NotApplicable
    );
}

#[test]
fn refuses_when_ubr_below_requirement() {
    let engine = PatchEngine::new();
    let mut patch = patch_fixture();
    patch.builds = vec!["19045.6466".to_string()];
    let requirements = PatchRequirements::default();
    let mut state = guest_state();
    state.ubr = Some(5000);

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::NotApplicable
    );
}

#[test]
fn applicable_when_ubr_meets_requirement() {
    let engine = PatchEngine::new();
    let mut patch = patch_fixture();
    patch.builds = vec!["19045.6466".to_string()];
    let requirements = PatchRequirements::default();
    let mut state = guest_state();
    state.ubr = Some(6466);

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Applicable
    );
}

#[test]
fn not_applicable_when_architecture_differs() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements::default();
    let mut state = guest_state();
    state.architecture = Architecture::X86;

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::NotApplicable
    );
}

#[test]
fn refuses_when_prerequisite_not_applied() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        prerequisite_patches: vec!["kb-prereq-0001".to_string()],
        ..Default::default()
    };
    let state = guest_state();

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn applicable_when_prerequisite_applied() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        prerequisite_patches: vec!["kb-prereq-0001".to_string()],
        ..Default::default()
    };
    let mut state = guest_state();
    state.applied_patches.insert("kb-prereq-0001".to_string());

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Applicable
    );
}

#[test]
fn refuses_when_component_version_unknown() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        component_versions: vec![ComponentRequirement {
            component: "ntoskrnl".to_string(),
            minimum_version: Some("10.0.19045.1".to_string()),
            maximum_version: None,
        }],
        ..Default::default()
    };
    let state = guest_state();

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn refuses_when_component_below_minimum() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        component_versions: vec![ComponentRequirement {
            component: "ntoskrnl".to_string(),
            minimum_version: Some("10.0.19045.5000".to_string()),
            maximum_version: None,
        }],
        ..Default::default()
    };
    let mut state = guest_state();
    state.components.insert(
        "ntoskrnl".to_string(),
        ComponentVersion {
            version: "10.0.19045.1000".to_string(),
        },
    );

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn applicable_when_component_meets_minimum() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        component_versions: vec![ComponentRequirement {
            component: "ntoskrnl".to_string(),
            minimum_version: Some("10.0.19045.1000".to_string()),
            maximum_version: None,
        }],
        ..Default::default()
    };
    let mut state = guest_state();
    state.components.insert(
        "ntoskrnl".to_string(),
        ComponentVersion {
            version: "10.0.19045.6466".to_string(),
        },
    );

    assert_eq!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Applicable
    );
}

#[test]
fn refuses_when_component_above_maximum() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements {
        component_versions: vec![ComponentRequirement {
            component: "ntoskrnl".to_string(),
            minimum_version: None,
            maximum_version: Some("10.0.19045.5000".to_string()),
        }],
        ..Default::default()
    };
    let mut state = guest_state();
    state.components.insert(
        "ntoskrnl".to_string(),
        ComponentVersion {
            version: "10.0.19045.6466".to_string(),
        },
    );

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn refuses_when_build_list_empty() {
    let engine = PatchEngine::new();
    let mut patch = patch_fixture();
    patch.builds.clear();
    let requirements = PatchRequirements::default();
    let state = guest_state();

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn refuses_when_architecture_list_empty() {
    let engine = PatchEngine::new();
    let mut patch = patch_fixture();
    patch.architectures.clear();
    let requirements = PatchRequirements::default();
    let state = guest_state();

    assert!(matches!(
        engine.evaluate(&patch, &requirements, &state),
        Applicability::Refused { .. }
    ));
}

#[test]
fn target_build_fixture_is_valid() {
    let build = target_fixture();
    assert!(build.validate().is_ok());
}

#[test]
fn patch_and_target_use_same_windows_10_build() {
    let patch = patch_fixture();
    let build = target_fixture();
    assert_eq!(patch.builds, vec!["19045.6466".to_string()]);
    assert_eq!(build.build, 19045);
    assert_eq!(build.architecture, Architecture::X64);
    assert_eq!(patch.status, PatchStatus::Experimental);
    assert_eq!(patch.schema_version.major, 1);
}
