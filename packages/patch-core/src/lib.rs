use anyhow::Result;
use catalog_schema::{Architecture, Artifact, PatchEntry, Validate};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub mod manifest;

pub use manifest::*;

/// Permitted operation types for the patch engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// Capture a backup before making changes.
    Backup,
    /// Validate a precondition before execution.
    ValidatePrecondition,
    /// Replace or update a target file.
    ReplaceFile,
    /// Remove or update a registry value.
    UpdateRegistry,
    /// Apply a package artifact.
    ApplyPackage,
    /// Create a restore point entry for rollback.
    RecordRestore,
    /// Roll back a previously recorded change.
    Rollback,
}

/// A planned operation in execution order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedOperation {
    /// Stable operation identifier within the plan.
    pub id: String,
    /// Operation type.
    pub kind: OperationKind,
    /// Identifier of the patch that produces this operation.
    pub patch_id: String,
    /// Target path or resource affected by the operation.
    pub target: String,
    /// Operations that must run before this one.
    pub depends_on: Vec<String>,
    /// Explanation of why this operation is planned.
    pub reason: String,
}

/// Type of conflict detected during planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Two operations target the same resource in an incompatible way.
    ResourceOverlap,
    /// A required artifact is missing from the request.
    MissingArtifact,
    /// A prerequisite is not satisfied.
    MissingPrerequisite,
    /// The patch is not applicable to the guest.
    NotApplicable,
    /// Applicability cannot be determined safely.
    Refused,
    /// Operations would conflict in ordering or permission.
    OrderConflict,
}

/// A detected conflict that blocks execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict classification.
    pub kind: ConflictKind,
    /// Human readable explanation.
    pub description: String,
    /// Identifiers of involved patches or operations.
    pub involved: Vec<String>,
}

/// Ordered plan for applying one or more patches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchPlan {
    /// Operations in execution order.
    pub operations: Vec<PlannedOperation>,
    /// Conflicts that prevent safe execution.
    pub conflicts: Vec<Conflict>,
    /// Recovery artifacts are planned and ready.
    pub recovery_ready: bool,
}

/// Request to plan patch operations.
#[derive(Debug, Clone)]
pub struct PlanRequest<'a> {
    /// Patch to plan.
    pub patch: &'a PatchEntry,
    /// Artifacts available for the patch.
    pub artifacts: &'a [Artifact],
    /// Current guest state.
    pub state: &'a GuestState,
    /// Additional requirements for applicability.
    pub requirements: &'a PatchRequirements,
}

/// Version information for an installed component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentVersion {
    /// Component version string as reported by the guest.
    pub version: String,
}

/// Component version requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRequirement {
    /// Component identifier that must exist in the guest state.
    pub component: String,
    /// Minimum allowed version string.
    pub minimum_version: Option<String>,
    /// Maximum allowed version string.
    pub maximum_version: Option<String>,
}

/// Requirements that the guest must satisfy for a patch.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PatchRequirements {
    /// Component version constraints.
    pub component_versions: Vec<ComponentRequirement>,
    /// Patch identifiers that must already be applied.
    pub prerequisite_patches: Vec<String>,
}

/// Observable state of a guest system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestState {
    /// Windows build number.
    pub build: u32,
    /// Update build revision, if known.
    pub ubr: Option<u32>,
    /// Guest processor architecture.
    pub architecture: Architecture,
    /// Installed component versions.
    pub components: HashMap<String, ComponentVersion>,
    /// Patch identifiers already applied to the guest.
    pub applied_patches: HashSet<String>,
}

/// Result of applicability evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    /// The patch can be applied given the known state.
    Applicable,
    /// The patch does not apply to this build, architecture, or configuration.
    NotApplicable,
    /// Applicability cannot be determined safely from the available state.
    Refused {
        /// Explanation of why the decision was refused.
        reason: String,
    },
}

/// Core patch engine.
#[derive(Debug)]
pub struct PatchEngine;

impl PatchEngine {
    /// Creates a new patch engine.
    pub fn new() -> Self {
        Self
    }

    /// Plans operations for a patch request with conflict detection.
    pub fn plan(&self, request: &PlanRequest) -> PatchPlan {
        let mut conflicts = Vec::new();
        let mut operations = Vec::new();

        match self.evaluate(request.patch, request.requirements, request.state) {
            Applicability::NotApplicable => {
                conflicts.push(Conflict {
                    kind: ConflictKind::NotApplicable,
                    description: format!(
                        "patch {} is not applicable to this guest",
                        request.patch.id
                    ),
                    involved: vec![request.patch.id.clone()],
                });
            }
            Applicability::Refused { reason } => {
                conflicts.push(Conflict {
                    kind: ConflictKind::Refused,
                    description: reason,
                    involved: vec![request.patch.id.clone()],
                });
            }
            Applicability::Applicable => {}
        }

        if request.artifacts.is_empty() {
            conflicts.push(Conflict {
                kind: ConflictKind::MissingArtifact,
                description: format!("patch {} has no artifacts available", request.patch.id),
                involved: vec![request.patch.id.clone()],
            });
        }

        if request.patch.validate().is_err() {
            conflicts.push(Conflict {
                kind: ConflictKind::Refused,
                description: "patch entry is invalid".to_string(),
                involved: vec![request.patch.id.clone()],
            });
        }

        for artifact in request.artifacts {
            if artifact.validate().is_err() {
                conflicts.push(Conflict {
                    kind: ConflictKind::Refused,
                    description: format!("artifact {} is invalid", artifact.id),
                    involved: vec![request.patch.id.clone(), artifact.id.clone()],
                });
            }

            if artifact.patch_id != request.patch.id {
                conflicts.push(Conflict {
                    kind: ConflictKind::OrderConflict,
                    description: format!(
                        "artifact {} belongs to {} but was provided for {}",
                        artifact.id, artifact.patch_id, request.patch.id
                    ),
                    involved: vec![request.patch.id.clone(), artifact.id.clone()],
                });
            }
        }

        let mut seen_resources: HashMap<String, String> = HashMap::new();
        let mut pending_conflicts: Vec<Conflict> = Vec::new();

        for artifact in request.artifacts {
            let resource = artifact.file_name.clone();
            if let Some(existing) = seen_resources.get(&resource) {
                pending_conflicts.push(Conflict {
                    kind: ConflictKind::ResourceOverlap,
                    description: format!(
                        "artifacts {} and {} target the same file {}",
                        existing, artifact.id, resource
                    ),
                    involved: vec![
                        request.patch.id.clone(),
                        existing.clone(),
                        artifact.id.clone(),
                    ],
                });
                continue;
            }

            seen_resources.insert(resource.clone(), artifact.id.clone());
        }

        if !pending_conflicts.is_empty() {
            conflicts.extend(pending_conflicts);
        }

        if conflicts.is_empty() {
            operations.push(PlannedOperation {
                id: format!("{}-backup", request.patch.id),
                kind: OperationKind::Backup,
                patch_id: request.patch.id.clone(),
                target: "system-state".to_string(),
                depends_on: Vec::new(),
                reason: "capture recovery point before changes".to_string(),
            });

            operations.push(PlannedOperation {
                id: format!("{}-validate-preconditions", request.patch.id),
                kind: OperationKind::ValidatePrecondition,
                patch_id: request.patch.id.clone(),
                target: request.patch.component.clone(),
                depends_on: vec![format!("{}-backup", request.patch.id)],
                reason: "verify preconditions remain satisfied".to_string(),
            });

            for artifact in request.artifacts {
                let resource = artifact.file_name.clone();

                let kind = match artifact.kind {
                    catalog_schema::ArtifactKind::Msu
                    | catalog_schema::ArtifactKind::Msi
                    | catalog_schema::ArtifactKind::Cab => OperationKind::ApplyPackage,
                    catalog_schema::ArtifactKind::Dll => OperationKind::ReplaceFile,
                    catalog_schema::ArtifactKind::Driver => OperationKind::ReplaceFile,
                    catalog_schema::ArtifactKind::Other => OperationKind::ReplaceFile,
                };

                operations.push(PlannedOperation {
                    id: format!("{}-{}", request.patch.id, artifact.id),
                    kind,
                    patch_id: request.patch.id.clone(),
                    target: resource,
                    depends_on: vec![format!("{}-validate-preconditions", request.patch.id)],
                    reason: format!("apply {} artifact", artifact.file_name),
                });
            }

            operations.push(PlannedOperation {
                id: format!("{}-record-restore", request.patch.id),
                kind: OperationKind::RecordRestore,
                patch_id: request.patch.id.clone(),
                target: "rollback-log".to_string(),
                depends_on: operations
                    .iter()
                    .filter(|op| {
                        op.patch_id == request.patch.id
                            && op.kind != OperationKind::Backup
                            && op.kind != OperationKind::RecordRestore
                    })
                    .map(|op| op.id.clone())
                    .collect(),
                reason: "record manifest for reversible rollback".to_string(),
            });
        }

        operations.sort_by_key(operation_order);

        let recovery_ready = conflicts.is_empty();

        PatchPlan {
            operations,
            conflicts,
            recovery_ready,
        }
    }

    /// Evaluates whether a patch applies to a guest state.
    ///
    /// The engine refuses when required information is missing or ambiguous.
    pub fn evaluate(
        &self,
        patch: &PatchEntry,
        requirements: &PatchRequirements,
        state: &GuestState,
    ) -> Applicability {
        if patch.validate().is_err() {
            return Applicability::Refused {
                reason: "patch entry is invalid".to_string(),
            };
        }

        if patch.builds.is_empty() {
            return Applicability::Refused {
                reason: "patch has no target builds".to_string(),
            };
        }

        if patch.architectures.is_empty() {
            return Applicability::Refused {
                reason: "patch has no target architectures".to_string(),
            };
        }

        if !matches_architecture(&patch.architectures, state.architecture) {
            return Applicability::NotApplicable;
        }

        if !matches_build(&patch.builds, state) {
            return Applicability::NotApplicable;
        }

        for prerequisite in &requirements.prerequisite_patches {
            if prerequisite.trim().is_empty() {
                return Applicability::Refused {
                    reason: "prerequisite patch identifier is empty".to_string(),
                };
            }

            if !state.applied_patches.contains(prerequisite) {
                return Applicability::Refused {
                    reason: format!("prerequisite patch {prerequisite} is not applied"),
                };
            }
        }

        for requirement in &requirements.component_versions {
            if requirement.component.trim().is_empty() {
                return Applicability::Refused {
                    reason: "component requirement has an empty identifier".to_string(),
                };
            }

            let Some(installed) = state.components.get(&requirement.component) else {
                return Applicability::Refused {
                    reason: format!("component {} version is unknown", requirement.component),
                };
            };

            if installed.version.trim().is_empty() {
                return Applicability::Refused {
                    reason: format!(
                        "component {} version is undetermined",
                        requirement.component
                    ),
                };
            }

            if let Some(minimum) = &requirement.minimum_version {
                if minimum.trim().is_empty() {
                    return Applicability::Refused {
                        reason: format!(
                            "component {} minimum version is undetermined",
                            requirement.component
                        ),
                    };
                }

                if !version_ge(&installed.version, minimum) {
                    return Applicability::Refused {
                        reason: format!(
                            "component {} version is below the minimum required",
                            requirement.component
                        ),
                    };
                }
            }

            if let Some(maximum) = &requirement.maximum_version {
                if maximum.trim().is_empty() {
                    return Applicability::Refused {
                        reason: format!(
                            "component {} maximum version is undetermined",
                            requirement.component
                        ),
                    };
                }

                if !version_le(&installed.version, maximum) {
                    return Applicability::Refused {
                        reason: format!(
                            "component {} version is above the maximum required",
                            requirement.component
                        ),
                    };
                }
            }
        }

        Applicability::Applicable
    }

    /// Legacy applicability check by patch identifier.
    pub fn check_applicability(&self, _patch_id: &str) -> Result<bool> {
        Ok(false)
    }

    /// Plans an apply operation for the given patch identifier.
    pub fn apply(&self, _patch_id: &str) -> Result<()> {
        Ok(())
    }

    /// Plans a rollback operation for the given patch identifier.
    pub fn rollback(&self, _patch_id: &str) -> Result<()> {
        Ok(())
    }
}

impl Default for PatchEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn matches_architecture(architectures: &[String], architecture: Architecture) -> bool {
    architectures.iter().any(|value| {
        let normalized = value.trim().to_ascii_lowercase();

        matches!(
            (normalized.as_str(), architecture),
            ("x64" | "amd64", Architecture::X64)
                | ("x86" | "i386" | "i686", Architecture::X86)
                | ("arm64" | "aarch64", Architecture::Arm64)
        )
    })
}

fn matches_build(builds: &[String], state: &GuestState) -> bool {
    builds.iter().any(|value| {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return false;
        }

        if trimmed.contains('.') {
            let parts: Vec<&str> = trimmed.split('.').collect();

            if parts.is_empty() {
                return false;
            }

            let Ok(build) = parts[0].parse::<u32>() else {
                return false;
            };

            if build != state.build {
                return false;
            }

            if parts.len() < 2 {
                return true;
            }

            let Some(ubr) = state.ubr else {
                return false;
            };

            let Ok(required_ubr) = parts[1].parse::<u32>() else {
                return false;
            };

            ubr >= required_ubr
        } else {
            if let Ok(build) = trimmed.parse::<u32>() {
                return build == state.build;
            }

            false
        }
    })
}

fn version_ge(installed: &str, minimum: &str) -> bool {
    let installed = normalize_version(installed);
    let minimum = normalize_version(minimum);

    if installed.is_empty() || minimum.is_empty() {
        return false;
    }

    compare_version_tokens(&installed, &minimum) >= 0
}

fn version_le(installed: &str, maximum: &str) -> bool {
    let installed = normalize_version(installed);
    let maximum = normalize_version(maximum);

    if installed.is_empty() || maximum.is_empty() {
        return false;
    }

    compare_version_tokens(&installed, &maximum) <= 0
}

fn operation_order(operation: &PlannedOperation) -> u8 {
    match operation.kind {
        OperationKind::Backup => 0,
        OperationKind::ValidatePrecondition => 1,
        OperationKind::ReplaceFile => 2,
        OperationKind::UpdateRegistry => 3,
        OperationKind::ApplyPackage => 4,
        OperationKind::RecordRestore => 5,
        OperationKind::Rollback => 6,
    }
}

fn normalize_version(value: &str) -> Vec<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<u64>().ok())
        .collect()
}

fn compare_version_tokens(left: &[u64], right: &[u64]) -> i32 {
    let length = left.len().max(right.len());

    for index in 0..length {
        let left_value = left.get(index).copied().unwrap_or(0);
        let right_value = right.get(index).copied().unwrap_or(0);

        if left_value > right_value {
            return 1;
        }

        if left_value < right_value {
            return -1;
        }
    }

    0
}
