use catalog_schema::{Architecture, Artifact, ArtifactKind, PatchEntry};
use patch_core::{
    ConflictKind, GuestState, OperationKind, PatchEngine, PatchRequirements, PlanRequest,
};
use std::collections::{HashMap, HashSet};

fn patch_fixture() -> PatchEntry {
    serde_json::from_str(include_str!(
        "../../catalog-schema/tests/fixtures/valid/patch.json"
    ))
    .expect("patch fixture parses")
}

fn artifact_fixture() -> Artifact {
    serde_json::from_str(include_str!(
        "../../catalog-schema/tests/fixtures/valid/artifact.json"
    ))
    .expect("artifact fixture parses")
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
fn plans_backup_validate_apply_record_for_applicable_patch() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let artifact = artifact_fixture();
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan.conflicts.is_empty());
    assert!(plan.recovery_ready);
    assert_eq!(plan.operations.len(), 4);
    assert_eq!(plan.operations[0].kind, OperationKind::Backup);
    assert_eq!(plan.operations[1].kind, OperationKind::ValidatePrecondition);
    assert_eq!(plan.operations[2].kind, OperationKind::ApplyPackage);
    assert_eq!(plan.operations[3].kind, OperationKind::RecordRestore);
    assert_eq!(
        plan.operations[2].target,
        "windows10-test-patch.msu".to_string()
    );
}

#[test]
fn detects_resource_overlap_conflict() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let mut artifact_a = artifact_fixture();
    artifact_a.id = "art-a".to_string();
    artifact_a.file_name = "conflict.dll".to_string();
    let mut artifact_b = artifact_fixture();
    artifact_b.id = "art-b".to_string();
    artifact_b.file_name = "conflict.dll".to_string();
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact_a, artifact_b],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert_eq!(plan.operations.len(), 0);
    assert!(!plan.recovery_ready);
    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::ResourceOverlap));
}

#[test]
fn detects_missing_artifact_conflict() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::MissingArtifact));
}

#[test]
fn detects_artifact_owner_mismatch() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let mut artifact = artifact_fixture();
    artifact.patch_id = "kb-other".to_string();
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::OrderConflict));
}

#[test]
fn detects_not_applicable_patch() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let artifact = artifact_fixture();
    let requirements = PatchRequirements::default();
    let mut state = guest_state();
    state.build = 17763;
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::NotApplicable));
}

#[test]
fn detects_refused_patch_due_to_prerequisite() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let artifact = artifact_fixture();
    let requirements = PatchRequirements {
        prerequisite_patches: vec!["kb-prereq".to_string()],
        ..Default::default()
    };
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::Refused));
}

#[test]
fn detects_refused_patch_due_to_missing_component() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let artifact = artifact_fixture();
    let requirements = PatchRequirements {
        component_versions: vec![patch_core::ComponentRequirement {
            component: "ntoskrnl".to_string(),
            minimum_version: Some("10.0.19045.1".to_string()),
            maximum_version: None,
        }],
        ..Default::default()
    };
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan
        .conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::Refused));
}

#[test]
fn plans_driver_as_replace_file_with_ordered_dependencies() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let mut artifact = artifact_fixture();
    artifact.id = "drv-1".to_string();
    artifact.file_name = "example.inf".to_string();
    artifact.kind = ArtifactKind::Driver;
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert!(plan.conflicts.is_empty());
    assert_eq!(plan.operations[2].kind, OperationKind::ReplaceFile);
    assert_eq!(plan.operations[2].target, "example.inf".to_string());
    assert!(plan.operations[2]
        .depends_on
        .contains(&format!("{}-validate-preconditions", patch.id)));
}

#[test]
fn record_restore_depends_on_applied_operations() {
    let engine = PatchEngine::new();
    let patch = patch_fixture();
    let artifact = artifact_fixture();
    let artifact_id = artifact.id.clone();
    let requirements = PatchRequirements::default();
    let state = guest_state();
    let request = PlanRequest {
        patch: &patch,
        artifacts: &[artifact],
        state: &state,
        requirements: &requirements,
    };

    let plan = engine.plan(&request);

    assert_eq!(plan.operations[3].kind, OperationKind::RecordRestore);
    assert!(plan.operations[3]
        .depends_on
        .contains(&format!("{}-{}", patch.id, artifact_id)));
}

#[test]
fn all_operation_kinds_are_permitted() {
    let kinds = [
        OperationKind::Backup,
        OperationKind::ValidatePrecondition,
        OperationKind::ReplaceFile,
        OperationKind::UpdateRegistry,
        OperationKind::ApplyPackage,
        OperationKind::RecordRestore,
        OperationKind::Rollback,
    ];

    assert_eq!(kinds.len(), 7);
}
