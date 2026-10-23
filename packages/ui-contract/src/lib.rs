//! Versioned JSON payloads exchanged between the Rust core and the Flutter UI.
//!
//! Every payload starts with `contract_version`. The Dart models understand the
//! current major version and reject anything else instead of guessing at the
//! shape of unknown fields.
//!
//! The fixtures under `fixtures/` are shared by Rust contract tests and Dart
//! parser tests, so both sides are checked against the same bytes.

use catalog_schema::{PatchEntry, SchemaError, SchemaVersion, TargetBuild, Validate};
use compatibility_catalog::{CompatibilityEntry, CompatibilityError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Contract major version understood by the current producers and consumers.
pub const CURRENT_CONTRACT_MAJOR: u32 = 1;

/// Diagnostics payload returned by `ll10_diagnostics_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsPayload {
    /// Version of the UI contract.
    pub contract_version: SchemaVersion,
    /// Version of the running agent build.
    pub agent_version: String,
    /// Operating system identifier reported by the Rust standard library.
    pub os: String,
    /// CPU architecture identifier reported by the Rust standard library.
    pub architecture: String,
    /// Windows version data, present only when running on Windows.
    pub windows: Option<WindowsVersionPayload>,
}

impl DiagnosticsPayload {
    /// Creates a diagnostics payload with the current contract version.
    pub fn new(
        agent_version: impl Into<String>,
        os: impl Into<String>,
        architecture: impl Into<String>,
        windows: Option<WindowsVersionPayload>,
    ) -> Self {
        Self {
            contract_version: SchemaVersion::current(),
            agent_version: agent_version.into(),
            os: os.into(),
            architecture: architecture.into(),
            windows,
        }
    }

    /// Validates the contract version and required fields.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.contract_version.validate()?;
        require_text(&self.agent_version, "agent_version")?;
        require_text(&self.os, "os")?;
        require_text(&self.architecture, "architecture")?;

        if let Some(windows) = &self.windows {
            windows.validate()?;
        }

        Ok(())
    }

    /// Serializes the payload as compact JSON for the FFI boundary.
    pub fn to_json(&self) -> Result<String, ContractError> {
        serde_json::to_string(self).map_err(ContractError::from_encoding)
    }

    /// Parses a payload received from the FFI boundary.
    pub fn from_json(value: &str) -> Result<Self, ContractError> {
        serde_json::from_str(value).map_err(ContractError::from_encoding)
    }
}

/// Windows version block returned in the diagnostics payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsVersionPayload {
    /// Marketing product name, for example `Windows 10 Pro`.
    pub product_name: String,
    /// Feature update version, for example `22H2`.
    pub display_version: String,
    /// Build number, for example `19045`.
    pub build: String,
    /// Update Build Revision (UBR), for example `3803`.
    pub update_build_revision: Option<u32>,
}

impl WindowsVersionPayload {
    /// Validates required Windows version fields.
    pub fn validate(&self) -> Result<(), ContractError> {
        require_text(&self.product_name, "windows.product_name")?;
        require_text(&self.display_version, "windows.display_version")?;
        require_text(&self.build, "windows.build")?;

        Ok(())
    }
}

/// Catalog payload used by catalog and status screens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogPayload {
    /// Version of the UI contract.
    pub contract_version: SchemaVersion,
    /// Windows builds referenced by catalog entries.
    pub target_builds: Vec<TargetBuild>,
    /// Security patches available for the supported builds.
    pub patches: Vec<PatchEntry>,
    /// Devices, drivers, and software assessed for the supported builds.
    pub compatibility: Vec<CompatibilityEntry>,
}

impl CatalogPayload {
    /// Creates a catalog payload with the current contract version.
    pub fn new(
        target_builds: Vec<TargetBuild>,
        patches: Vec<PatchEntry>,
        compatibility: Vec<CompatibilityEntry>,
    ) -> Self {
        Self {
            contract_version: SchemaVersion::current(),
            target_builds,
            patches,
            compatibility,
        }
    }

    /// Validates the contract version and every catalog entry.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.contract_version.validate()?;

        for target_build in &self.target_builds {
            target_build.validate()?;
        }

        for patch in &self.patches {
            patch.validate()?;
        }

        for entry in &self.compatibility {
            entry.validate()?;
        }

        Ok(())
    }

    /// Serializes the payload as compact JSON.
    pub fn to_json(&self) -> Result<String, ContractError> {
        serde_json::to_string(self).map_err(ContractError::from_encoding)
    }

    /// Parses a catalog payload.
    pub fn from_json(value: &str) -> Result<Self, ContractError> {
        serde_json::from_str(value).map_err(ContractError::from_encoding)
    }
}

/// Problems found while decoding or validating a UI payload.
#[derive(Debug, Error)]
pub enum ContractError {
    /// Shared catalog schema problem.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// Compatibility catalog problem.
    #[error(transparent)]
    Compatibility(#[from] CompatibilityError),
    /// JSON encoding or decoding problem.
    #[error("contract JSON error: {message}")]
    Encoding {
        /// Underlying serializer message.
        message: String,
    },
    /// Required text field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },
}

impl ContractError {
    fn from_encoding(error: serde_json::Error) -> Self {
        Self::Encoding {
            message: error.to_string(),
        }
    }
}

fn require_text(value: &str, field: &'static str) -> Result<(), ContractError> {
    if value.trim().is_empty() {
        return Err(ContractError::EmptyField { field });
    }

    Ok(())
}
