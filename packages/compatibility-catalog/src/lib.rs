//! Versioned compatibility catalog data and fixtures.
//!
//! Entries describe devices, drivers, and software that have been assessed
//! against a specific Windows target build. Every entry carries external
//! source references so a reviewer can reproduce the assessment.

use catalog_schema::{Architecture, SchemaError, SchemaVersion, Validate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Kind of compatibility entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompatibilityKind {
    /// Kernel or user-mode driver package.
    Driver,
    /// Physical or virtual device.
    Device,
    /// Application, runtime, or redistributable package.
    Software,
}

/// Lifecycle state of a compatibility entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompatibilityStatus {
    /// Catalogued and available for testing only.
    Experimental,
    /// Passed the validation tier for its tested builds.
    Validated,
    /// Replaced or withdrawn.
    Discontinued,
}

/// External evidence used to support an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReference {
    /// Human readable source title.
    pub title: String,
    /// HTTPS location of the source.
    pub url: String,
    /// Publisher or maintainer of the source.
    pub publisher: String,
    /// Retrieval date in `YYYY-MM-DD` form.
    pub retrieved_at: String,
}

impl SourceReference {
    /// Validates the source reference.
    pub fn validate(&self) -> Result<(), CompatibilityError> {
        require_text(&self.title, "sources[].title")?;
        require_text(&self.url, "sources[].url")?;
        require_text(&self.publisher, "sources[].publisher")?;
        require_text(&self.retrieved_at, "sources[].retrieved_at")?;

        if !self.url.starts_with("https://") {
            return Err(CompatibilityError::InsecureSource {
                value: self.url.clone(),
            });
        }

        if !is_date(&self.retrieved_at) {
            return Err(CompatibilityError::InvalidDate {
                value: self.retrieved_at.clone(),
            });
        }

        Ok(())
    }
}

/// Compatibility record for a device, driver, or software package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    /// Stable entry identifier used by dependencies.
    pub id: String,
    /// Contract version used by this entry.
    pub schema_version: SchemaVersion,
    /// Entry kind.
    pub kind: CompatibilityKind,
    /// Display name.
    pub name: String,
    /// Hardware or software vendor.
    pub vendor: String,
    /// What the entry covers and why it matters on Windows 10.
    pub description: String,
    /// Target build identifiers used during assessment.
    pub tested_builds: Vec<String>,
    /// Architectures covered by the assessment.
    pub architectures: Vec<Architecture>,
    /// Device hardware identifiers, required for drivers and devices.
    #[serde(default)]
    pub hardware_ids: Vec<String>,
    /// Driver version under assessment.
    #[serde(default)]
    pub driver_version: Option<String>,
    /// Software version under assessment.
    #[serde(default)]
    pub version: Option<String>,
    /// Entry identifiers that must be present first.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Known defects or regressions.
    #[serde(default)]
    pub known_issues: Vec<String>,
    /// Explicit limits of the assessment.
    #[serde(default)]
    pub limitations: Vec<String>,
    /// External evidence for the entry.
    pub sources: Vec<SourceReference>,
    /// Current lifecycle state.
    pub status: CompatibilityStatus,
}

impl CompatibilityEntry {
    /// Validates the entry and every source reference.
    pub fn validate(&self) -> Result<(), CompatibilityError> {
        self.schema_version.validate()?;
        require_text(&self.id, "id")?;
        require_text(&self.name, "name")?;
        require_text(&self.vendor, "vendor")?;
        require_text(&self.description, "description")?;

        if self.tested_builds.is_empty() {
            return Err(CompatibilityError::EmptyField {
                field: "tested_builds",
            });
        }

        if self.architectures.is_empty() {
            return Err(CompatibilityError::EmptyField {
                field: "architectures",
            });
        }

        for build in &self.tested_builds {
            require_text(build, "tested_builds[]")?;
        }

        match self.kind {
            CompatibilityKind::Driver | CompatibilityKind::Device => {
                if self.hardware_ids.is_empty() {
                    return Err(CompatibilityError::MissingHardwareIds);
                }
            }
            CompatibilityKind::Software => {}
        }

        if self.kind == CompatibilityKind::Driver
            && self
                .driver_version
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(CompatibilityError::MissingDriverVersion);
        }

        for hardware_id in &self.hardware_ids {
            require_text(hardware_id, "hardware_ids[]")?;
        }

        if let Some(version) = &self.driver_version {
            require_text(version, "driver_version")?;
        }

        if let Some(version) = &self.version {
            require_text(version, "version")?;
        }

        for dependency in &self.dependencies {
            require_text(dependency, "dependencies[]")?;
        }

        for issue in &self.known_issues {
            require_text(issue, "known_issues[]")?;
        }

        for limitation in &self.limitations {
            require_text(limitation, "limitations[]")?;
        }

        if self.sources.is_empty() {
            return Err(CompatibilityError::EmptyField { field: "sources" });
        }

        for source in &self.sources {
            source.validate()?;
        }

        Ok(())
    }
}

/// Problems found while validating a compatibility entry.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CompatibilityError {
    /// Shared schema version problem.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// Required text field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },
    /// Driver or device entry has no hardware identifier.
    #[error("driver and device entries need at least one hardware ID")]
    MissingHardwareIds,
    /// Driver entry has no driver version.
    #[error("driver entries need a driver version")]
    MissingDriverVersion,
    /// Source URL is not HTTPS.
    #[error("source URL must use HTTPS: {value}")]
    InsecureSource {
        /// Value found in the document.
        value: String,
    },
    /// Retrieval date is not in `YYYY-MM-DD` form.
    #[error("source retrieved_at must use YYYY-MM-DD: {value}")]
    InvalidDate {
        /// Value found in the document.
        value: String,
    },
}

/// Verifies that a required string field is not blank.
fn require_text(value: &str, field: &'static str) -> Result<(), CompatibilityError> {
    if value.trim().is_empty() {
        return Err(CompatibilityError::EmptyField { field });
    }

    Ok(())
}

/// Checks the `YYYY-MM-DD` date shape without calendar validation.
fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();

    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}
