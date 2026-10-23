use anyhow::Result;
use catalog_schema::{Architecture, PatchEntry, Validate};
use std::collections::{HashMap, HashSet};

/// Version information for an installed component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentVersion {
    /// Component version string as reported by the guest.
    pub version: String,
}

/// Component version requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRequirement {
    /// Component identifier that must exist in the guest state.
    pub component: String,
    /// Minimum allowed version string.
    pub minimum_version: Option<String>,
    /// Maximum allowed version string.
    pub maximum_version: Option<String>,
}

/// Requirements that the guest must satisfy for a patch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatchRequirements {
    /// Component version constraints.
    pub component_versions: Vec<ComponentRequirement>,
    /// Patch identifiers that must already be applied.
    pub prerequisite_patches: Vec<String>,
}

/// Observable state of a guest system.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
