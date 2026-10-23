use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Major schema version supported by this crate.
pub const CURRENT_SCHEMA_MAJOR: u32 = 1;

/// Version attached to every catalog contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Breaking schema generation.
    pub major: u32,
    /// Backward compatible additions.
    pub minor: u32,
    /// Corrections that do not change the shape.
    pub patch: u32,
}

impl SchemaVersion {
    /// Returns the version written by current producers.
    pub const fn current() -> Self {
        Self {
            major: CURRENT_SCHEMA_MAJOR,
            minor: 0,
            patch: 0,
        }
    }
}

/// Semantic validation for catalog contracts.
pub trait Validate {
    /// Returns the first problem found in the value.
    fn validate(&self) -> Result<(), SchemaError>;
}

impl Validate for SchemaVersion {
    fn validate(&self) -> Result<(), SchemaError> {
        if self.major != CURRENT_SCHEMA_MAJOR {
            return Err(SchemaError::UnsupportedVersion {
                major: self.major,
                expected: CURRENT_SCHEMA_MAJOR,
            });
        }

        Ok(())
    }
}

/// Lifecycle state of a patch entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchStatus {
    /// Catalogued and available for testing only.
    Experimental,
    /// Passed the validation tier for its target builds.
    Validated,
    /// Replaced or withdrawn.
    Discontinued,
}

/// Patch catalog entry shown to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchEntry {
    /// Stable patch identifier.
    pub id: String,
    /// Contract version used by this entry.
    pub schema_version: SchemaVersion,
    /// CVE identifiers addressed by the patch.
    pub cves: Vec<String>,
    /// Windows component affected by the patch.
    pub component: String,
    /// Target build identifiers where the patch applies.
    pub builds: Vec<String>,
    /// Architectures supported by the patch.
    pub architectures: Vec<String>,
    /// Short display title.
    pub title: String,
    /// Longer explanation of the fix.
    pub description: String,
    /// Current lifecycle state.
    pub status: PatchStatus,
}

impl Validate for PatchEntry {
    fn validate(&self) -> Result<(), SchemaError> {
        self.schema_version.validate()?;
        require_text(&self.id, "id")?;
        require_text(&self.component, "component")?;
        require_text(&self.title, "title")?;
        require_text(&self.description, "description")?;

        if self.cves.is_empty() {
            return Err(SchemaError::EmptyField { field: "cves" });
        }

        for cve in &self.cves {
            if !is_cve(cve) {
                return Err(SchemaError::InvalidCve { value: cve.clone() });
            }
        }

        require_list(&self.builds, "builds")?;
        require_list(&self.architectures, "architectures")?;

        Ok(())
    }
}

/// Kind of downloadable patch artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    /// Windows update package.
    Msu,
    /// Cabinet archive.
    Cab,
    /// Windows installer package.
    Msi,
    /// Standalone library.
    Dll,
    /// Driver package.
    Driver,
    /// Any other reviewed format.
    Other,
}

/// Downloadable file referenced by a patch entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Stable artifact identifier.
    pub id: String,
    /// Patch that owns the artifact.
    pub patch_id: String,
    /// Contract version used by this artifact.
    pub schema_version: SchemaVersion,
    /// File format.
    pub kind: ArtifactKind,
    /// File name without directories.
    pub file_name: String,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Optional HTTPS location.
    pub source_url: Option<String>,
}

impl Validate for Artifact {
    fn validate(&self) -> Result<(), SchemaError> {
        self.schema_version.validate()?;
        require_text(&self.id, "id")?;
        require_text(&self.patch_id, "patch_id")?;
        require_text(&self.file_name, "file_name")?;

        if self.file_name.contains('/') || self.file_name.contains('\\') {
            return Err(SchemaError::InvalidFileName {
                value: self.file_name.clone(),
            });
        }

        if self.file_name == "." || self.file_name == ".." {
            return Err(SchemaError::InvalidFileName {
                value: self.file_name.clone(),
            });
        }

        if !is_sha256(&self.sha256) {
            return Err(SchemaError::InvalidSha256 {
                value: self.sha256.clone(),
            });
        }

        if self.size_bytes == 0 {
            return Err(SchemaError::EmptyArtifact);
        }

        if let Some(url) = &self.source_url {
            if !url.starts_with("https://") {
                return Err(SchemaError::InsecureSource { value: url.clone() });
            }
        }

        Ok(())
    }
}

/// Processor architecture supported by a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    /// 32-bit x86.
    X86,
    /// 64-bit x86.
    X64,
    /// 64-bit ARM.
    Arm64,
}

/// Windows build targeted by patches and validation runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetBuild {
    /// Stable build identifier used by references.
    pub id: String,
    /// Contract version used by this build.
    pub schema_version: SchemaVersion,
    /// Marketing product name.
    pub product_name: String,
    /// Marketing version such as `22H2`.
    pub display_version: String,
    /// Kernel version such as `10.0`.
    pub os_version: String,
    /// Windows build number.
    pub build: u32,
    /// Update build revision.
    pub ubr: u32,
    /// Target processor architecture.
    pub architecture: Architecture,
}

impl Validate for TargetBuild {
    fn validate(&self) -> Result<(), SchemaError> {
        self.schema_version.validate()?;
        require_text(&self.id, "id")?;
        require_text(&self.product_name, "product_name")?;
        require_text(&self.display_version, "display_version")?;
        require_text(&self.os_version, "os_version")?;

        if self.build < 10240 {
            return Err(SchemaError::InvalidBuild { value: self.build });
        }

        Ok(())
    }
}

/// Outcome of a validation run or one of its checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    /// Completed without failures.
    Passed,
    /// Completed with at least one failure.
    Failed,
    /// Not executed.
    Skipped,
    /// Harness could not determine an outcome.
    Error,
}

/// One check executed during validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Check name such as `install` or `boot`.
    pub name: String,
    /// Outcome of this check.
    pub status: ValidationStatus,
    /// Human readable evidence.
    pub details: String,
}

/// Result produced by a VM or static validation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Stable result identifier.
    pub id: String,
    /// Patch under test.
    pub patch_id: String,
    /// Target build used by the run.
    pub target_build_id: String,
    /// Contract version used by this result.
    pub schema_version: SchemaVersion,
    /// Overall outcome.
    pub status: ValidationStatus,
    /// RFC 3339 UTC start time.
    pub started_at: String,
    /// RFC 3339 UTC finish time.
    pub finished_at: String,
    /// Individual checks.
    pub checks: Vec<ValidationCheck>,
    /// Optional log path relative to the run directory.
    pub log_path: Option<String>,
}

impl Validate for ValidationResult {
    fn validate(&self) -> Result<(), SchemaError> {
        self.schema_version.validate()?;
        require_text(&self.id, "id")?;
        require_text(&self.patch_id, "patch_id")?;
        require_text(&self.target_build_id, "target_build_id")?;
        require_text(&self.started_at, "started_at")?;
        require_text(&self.finished_at, "finished_at")?;

        if self.finished_at < self.started_at {
            return Err(SchemaError::InvalidTimeRange);
        }

        if self.checks.is_empty() {
            return Err(SchemaError::EmptyChecks);
        }

        for check in &self.checks {
            require_text(&check.name, "checks[].name")?;
        }

        let failed = self
            .checks
            .iter()
            .find(|check| check.status == ValidationStatus::Failed);
        let errored = self
            .checks
            .iter()
            .find(|check| check.status == ValidationStatus::Error);

        match self.status {
            ValidationStatus::Passed if failed.is_some() || errored.is_some() => {
                let check = failed.or(errored).map(|value| value.name.clone());
                Err(SchemaError::PassedWithFailedCheck {
                    check: check.unwrap_or_default(),
                })
            }
            ValidationStatus::Failed if failed.is_none() => {
                Err(SchemaError::FailedWithoutFailedCheck)
            }
            ValidationStatus::Error if errored.is_none() => {
                Err(SchemaError::ErrorWithoutErroredCheck)
            }
            _ => Ok(()),
        }
    }
}

/// Problems found while validating a catalog contract.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SchemaError {
    /// Schema generation is not understood by this crate.
    #[error("unsupported schema major version {major}; expected {expected}")]
    UnsupportedVersion {
        /// Version found in the document.
        major: u32,
        /// Version supported by this crate.
        expected: u32,
    },
    /// Required text field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },
    /// CVE identifier does not match the expected shape.
    #[error("invalid CVE identifier: {value}")]
    InvalidCve {
        /// Value found in the document.
        value: String,
    },
    /// Digest is not a SHA-256 hex string.
    #[error("invalid SHA-256 hash: {value}")]
    InvalidSha256 {
        /// Value found in the document.
        value: String,
    },
    /// File name contains path segments.
    #[error("artifact file name must not contain a path: {value}")]
    InvalidFileName {
        /// Value found in the document.
        value: String,
    },
    /// Artifact has no content.
    #[error("artifact size must be greater than zero")]
    EmptyArtifact,
    /// Download URL is not HTTPS.
    #[error("artifact source must use HTTPS: {value}")]
    InsecureSource {
        /// Value found in the document.
        value: String,
    },
    /// Build number is older than Windows 10.
    #[error("unsupported Windows build number: {value}")]
    InvalidBuild {
        /// Value found in the document.
        value: u32,
    },
    /// Finish time precedes start time.
    #[error("validation result timestamps are out of order")]
    InvalidTimeRange,
    /// Validation result has no checks.
    #[error("validation result must contain at least one check")]
    EmptyChecks,
    /// Passed result contains a failed or errored check.
    #[error("passed validation result contains a failed check: {check}")]
    PassedWithFailedCheck {
        /// Name of the offending check.
        check: String,
    },
    /// Failed result has no failed check.
    #[error("failed validation result has no failed check")]
    FailedWithoutFailedCheck,
    /// Errored result has no errored check.
    #[error("errored validation result has no errored check")]
    ErrorWithoutErroredCheck,
}

/// Verifies that a required string field is not blank.
fn require_text(value: &str, field: &'static str) -> Result<(), SchemaError> {
    if value.trim().is_empty() {
        return Err(SchemaError::EmptyField { field });
    }

    Ok(())
}

/// Verifies that a required list has no empty values.
fn require_list(values: &[String], field: &'static str) -> Result<(), SchemaError> {
    if values.is_empty() {
        return Err(SchemaError::EmptyField { field });
    }

    for value in values {
        require_text(value, field)?;
    }

    Ok(())
}

/// Checks the `CVE-YYYY-NNNN` identifier shape.
fn is_cve(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("CVE-") else {
        return false;
    };

    let Some((year, id)) = rest.split_once('-') else {
        return false;
    };

    year.len() == 4
        && year.chars().all(|character| character.is_ascii_digit())
        && id.len() >= 4
        && id.chars().all(|character| character.is_ascii_digit())
}

/// Checks for a 64 character hexadecimal digest.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}
