//! Requirement resolution for devices, drivers, and software.
//!
//! The engine compares a [`CompatibilityEntry`] with the observable state of a
//! guest. It never installs anything and never claims compatibility when a
//! required value is unknown.

use catalog_schema::Architecture;
use compatibility_catalog::{CompatibilityEntry, CompatibilityKind, CompatibilityStatus};
use std::collections::{HashMap, HashSet};

/// Observable state used to resolve a compatibility entry.
#[derive(Debug, Clone, Default)]
pub struct GuestProfile {
    /// Target build identifier such as `win10-22h2-x64`.
    pub build: String,
    /// Guest processor architecture.
    pub architecture: Option<Architecture>,
    /// Compatibility entry identifiers already satisfied on the guest.
    pub applied_entries: HashSet<String>,
    /// Installed software keyed by compatibility entry identifier.
    pub software_versions: HashMap<String, String>,
    /// Hardware identifiers present on the guest.
    pub hardware_ids: HashSet<String>,
    /// Driver versions keyed by normalized hardware identifier.
    pub driver_versions: HashMap<String, String>,
}

impl GuestProfile {
    /// Creates an empty profile for a target build.
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            build: build.into(),
            ..Default::default()
        }
    }

    /// Returns whether a hardware identifier is present, ignoring case.
    pub fn has_hardware_id(&self, value: &str) -> bool {
        self.hardware_ids
            .iter()
            .any(|candidate| normalize_hardware_id(candidate) == normalize_hardware_id(value))
    }

    /// Returns the installed driver version for a hardware identifier.
    pub fn driver_version(&self, value: &str) -> Option<&str> {
        self.driver_versions.iter().find_map(|(key, version)| {
            if normalize_hardware_id(key) == normalize_hardware_id(value) {
                Some(version.as_str())
            } else {
                None
            }
        })
    }
}

/// Status of a single requirement check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Requirement is satisfied.
    Satisfied,
    /// Requirement is definitively not satisfied.
    Unsatisfied,
    /// Requirement cannot be evaluated from the available state.
    Undetermined,
}

/// Kind of requirement evaluated by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementKind {
    /// Entry contract validity.
    Contract,
    /// Lifecycle status.
    Lifecycle,
    /// Target build.
    Build,
    /// Processor architecture.
    Architecture,
    /// Entry dependency.
    Dependency,
    /// Hardware identifier.
    HardwareId,
    /// Driver version.
    DriverVersion,
    /// Software presence.
    Software,
    /// Software version.
    SoftwareVersion,
}

/// Result of one requirement check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementCheck {
    /// Requirement category.
    pub kind: RequirementKind,
    /// Value or identifier being checked.
    pub requirement: String,
    /// Outcome of the check.
    pub status: CheckStatus,
    /// Explanation for the outcome.
    pub detail: String,
}

impl RequirementCheck {
    /// Creates a satisfied check.
    pub fn satisfied(
        kind: RequirementKind,
        requirement: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            requirement: requirement.into(),
            status: CheckStatus::Satisfied,
            detail: detail.into(),
        }
    }

    /// Creates an unsatisfied check.
    pub fn unsatisfied(
        kind: RequirementKind,
        requirement: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            requirement: requirement.into(),
            status: CheckStatus::Unsatisfied,
            detail: detail.into(),
        }
    }

    /// Creates an undetermined check.
    pub fn undetermined(
        kind: RequirementKind,
        requirement: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            requirement: requirement.into(),
            status: CheckStatus::Undetermined,
            detail: detail.into(),
        }
    }
}

/// Aggregate resolution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStatus {
    /// Every requirement is satisfied.
    Compatible,
    /// At least one requirement is definitively not satisfied.
    Incompatible,
    /// At least one requirement cannot be evaluated safely.
    Refused,
}

/// Resolution for a single compatibility entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Compatibility entry identifier.
    pub entry_id: String,
    /// Aggregate status.
    pub status: ResolutionStatus,
    /// Individual requirement checks.
    pub checks: Vec<RequirementCheck>,
    /// Known issues copied from the entry.
    pub known_issues: Vec<String>,
    /// Limitations copied from the entry.
    pub limitations: Vec<String>,
}

impl Resolution {
    /// Returns checks with the given status.
    pub fn checks_with(&self, status: CheckStatus) -> Vec<&RequirementCheck> {
        self.checks
            .iter()
            .filter(|check| check.status == status)
            .collect()
    }
}

/// Requirement resolver.
#[derive(Debug, Default)]
pub struct CompatibilityEngine;

impl CompatibilityEngine {
    /// Creates a resolver.
    pub fn new() -> Self {
        Self
    }

    /// Resolves every entry against the same guest profile.
    pub fn resolve_catalog(
        &self,
        entries: &[CompatibilityEntry],
        profile: &GuestProfile,
    ) -> Vec<Resolution> {
        entries
            .iter()
            .map(|entry| self.resolve(entry, profile))
            .collect()
    }

    /// Resolves one entry against a guest profile.
    pub fn resolve(&self, entry: &CompatibilityEntry, profile: &GuestProfile) -> Resolution {
        let mut checks = Vec::new();

        if let Err(error) = entry.validate() {
            checks.push(RequirementCheck::unsatisfied(
                RequirementKind::Contract,
                "entry",
                error.to_string(),
            ));

            return resolution(entry, checks);
        }

        checks.push(RequirementCheck::satisfied(
            RequirementKind::Contract,
            "entry",
            "entry contract is valid",
        ));

        match entry.status {
            CompatibilityStatus::Validated => checks.push(RequirementCheck::satisfied(
                RequirementKind::Lifecycle,
                "status",
                "entry is validated",
            )),
            CompatibilityStatus::Experimental => checks.push(RequirementCheck::satisfied(
                RequirementKind::Lifecycle,
                "status",
                "entry is experimental and has not passed the validation tier",
            )),
            CompatibilityStatus::Discontinued => checks.push(RequirementCheck::unsatisfied(
                RequirementKind::Lifecycle,
                "status",
                "entry is discontinued",
            )),
        }

        if entry
            .tested_builds
            .iter()
            .any(|build| build.eq_ignore_ascii_case(&profile.build))
        {
            checks.push(RequirementCheck::satisfied(
                RequirementKind::Build,
                profile.build.clone(),
                "guest build is listed by the entry",
            ));
        } else {
            checks.push(RequirementCheck::unsatisfied(
                RequirementKind::Build,
                profile.build.clone(),
                format!("guest build {} is not listed by the entry", profile.build),
            ));
        }

        match profile.architecture {
            Some(architecture) if entry.architectures.contains(&architecture) => {
                checks.push(RequirementCheck::satisfied(
                    RequirementKind::Architecture,
                    format!("{architecture:?}"),
                    "guest architecture is supported by the entry",
                ));
            }
            Some(architecture) => checks.push(RequirementCheck::unsatisfied(
                RequirementKind::Architecture,
                format!("{architecture:?}"),
                "guest architecture is not supported by the entry",
            )),
            None => checks.push(RequirementCheck::undetermined(
                RequirementKind::Architecture,
                "architecture",
                "guest architecture is unknown",
            )),
        }

        for dependency in &entry.dependencies {
            if profile.applied_entries.contains(dependency) {
                checks.push(RequirementCheck::satisfied(
                    RequirementKind::Dependency,
                    dependency,
                    "dependency is present",
                ));
            } else {
                checks.push(RequirementCheck::unsatisfied(
                    RequirementKind::Dependency,
                    dependency,
                    "dependency is missing",
                ));
            }
        }

        match entry.kind {
            CompatibilityKind::Driver => {
                resolve_hardware(entry, profile, &mut checks);
                resolve_driver_version(entry, profile, &mut checks);
            }
            CompatibilityKind::Device => {
                resolve_hardware(entry, profile, &mut checks);
            }
            CompatibilityKind::Software => {
                resolve_software(entry, profile, &mut checks);
            }
        }

        resolution(entry, checks)
    }
}

fn resolve_hardware(
    entry: &CompatibilityEntry,
    profile: &GuestProfile,
    checks: &mut Vec<RequirementCheck>,
) {
    let matched: Vec<&String> = entry
        .hardware_ids
        .iter()
        .filter(|hardware_id| profile.has_hardware_id(hardware_id))
        .collect();

    if matched.is_empty() {
        checks.push(RequirementCheck::unsatisfied(
            RequirementKind::HardwareId,
            entry.hardware_ids.join(", "),
            "no listed hardware identifier is present on the guest",
        ));
        return;
    }

    for hardware_id in matched {
        checks.push(RequirementCheck::satisfied(
            RequirementKind::HardwareId,
            hardware_id.as_str(),
            "hardware identifier is present on the guest",
        ));
    }
}

fn resolve_driver_version(
    entry: &CompatibilityEntry,
    profile: &GuestProfile,
    checks: &mut Vec<RequirementCheck>,
) {
    let Some(tested) = entry.driver_version.as_deref() else {
        checks.push(RequirementCheck::undetermined(
            RequirementKind::DriverVersion,
            "driver",
            "entry has no assessed driver version",
        ));
        return;
    };

    let matched: Vec<&String> = entry
        .hardware_ids
        .iter()
        .filter(|hardware_id| profile.has_hardware_id(hardware_id))
        .collect();

    if matched.is_empty() {
        return;
    }

    for hardware_id in matched {
        match profile.driver_version(hardware_id) {
            Some(installed) if versions_match(installed, tested) => {
                checks.push(RequirementCheck::satisfied(
                    RequirementKind::DriverVersion,
                    hardware_id.as_str(),
                    format!("installed driver version {installed} matches {tested}"),
                ));
            }
            Some(installed) => checks.push(RequirementCheck::unsatisfied(
                RequirementKind::DriverVersion,
                hardware_id.as_str(),
                format!("installed driver version {installed} does not match tested {tested}"),
            )),
            None => checks.push(RequirementCheck::undetermined(
                RequirementKind::DriverVersion,
                hardware_id.as_str(),
                "installed driver version is unknown",
            )),
        }
    }
}

fn resolve_software(
    entry: &CompatibilityEntry,
    profile: &GuestProfile,
    checks: &mut Vec<RequirementCheck>,
) {
    let installed = profile.software_versions.get(&entry.id);

    match installed {
        Some(version) => {
            checks.push(RequirementCheck::satisfied(
                RequirementKind::Software,
                entry.id.as_str(),
                "software is installed",
            ));

            if let Some(tested) = entry.version.as_deref() {
                if versions_match(version, tested) {
                    checks.push(RequirementCheck::satisfied(
                        RequirementKind::SoftwareVersion,
                        entry.id.as_str(),
                        format!("installed version {version} matches tested {tested}"),
                    ));
                } else {
                    checks.push(RequirementCheck::unsatisfied(
                        RequirementKind::SoftwareVersion,
                        entry.id.as_str(),
                        format!("installed version {version} does not match tested {tested}"),
                    ));
                }
            }
        }
        None => checks.push(RequirementCheck::unsatisfied(
            RequirementKind::Software,
            entry.id.as_str(),
            "software is not installed",
        )),
    }
}

fn resolution(entry: &CompatibilityEntry, checks: Vec<RequirementCheck>) -> Resolution {
    let status = if checks
        .iter()
        .any(|check| check.status == CheckStatus::Unsatisfied)
    {
        ResolutionStatus::Incompatible
    } else if checks
        .iter()
        .any(|check| check.status == CheckStatus::Undetermined)
    {
        ResolutionStatus::Refused
    } else {
        ResolutionStatus::Compatible
    };

    Resolution {
        entry_id: entry.id.clone(),
        status,
        checks,
        known_issues: entry.known_issues.clone(),
        limitations: entry.limitations.clone(),
    }
}

/// Normalizes a Windows hardware identifier for comparison.
pub fn normalize_hardware_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

/// Compares an installed version with a tested version.
///
/// Numeric segments are compared with zero padding. A tested value may use `x`
/// or `*` as a wildcard segment, for example `14.x` or `10.0.*`.
pub fn versions_match(installed: &str, tested: &str) -> bool {
    let installed = installed.trim();
    let tested = tested.trim();

    if installed.is_empty() || tested.is_empty() {
        return false;
    }

    if tested.contains('x') || tested.contains('X') || tested.contains('*') {
        return wildcard_match(installed, tested);
    }

    let installed_tokens = numeric_tokens(installed);
    let tested_tokens = numeric_tokens(tested);

    if installed_tokens.is_empty() || tested_tokens.is_empty() {
        return installed.eq_ignore_ascii_case(tested);
    }

    let length = installed_tokens.len().max(tested_tokens.len());

    for index in 0..length {
        let installed_value = installed_tokens.get(index).copied().unwrap_or(0);
        let tested_value = tested_tokens.get(index).copied().unwrap_or(0);

        if installed_value != tested_value {
            return false;
        }
    }

    true
}

fn wildcard_match(installed: &str, tested: &str) -> bool {
    let installed_segments: Vec<&str> = installed.split('.').collect();
    let tested_segments: Vec<&str> = tested.split('.').collect();

    if tested_segments.len() > installed_segments.len() && !tested_segments.contains(&"x") {
        return false;
    }

    for (index, tested_segment) in tested_segments.iter().enumerate() {
        if *tested_segment == "x"
            || *tested_segment == "X"
            || *tested_segment == "*"
            || tested_segment.is_empty()
        {
            continue;
        }

        let Some(installed_segment) = installed_segments.get(index) else {
            return false;
        };

        if normalize_segment(installed_segment) != normalize_segment(tested_segment) {
            return false;
        }
    }

    true
}

fn normalize_segment(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn numeric_tokens(value: &str) -> Vec<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<u64>().ok())
        .collect()
}
