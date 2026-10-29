use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MANIFEST_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Guest,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    X86,
    Arm64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requires {
    pub admin: bool,
    #[serde(default)]
    pub reboot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliesTo {
    pub os: String,
    #[serde(default)]
    pub arch: Vec<Arch>,
    pub builds: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Docs {
    pub readme: String,
    #[serde(default)]
    pub recovery: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub file: String,
    pub sha256: String,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub arch: Option<Arch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Precondition {
    FileSha256 { id: String, path: String, one_of: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryRef {
    pub hive: String,
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    BackupFile { id: String, path: String },
    StageFile { id: String, artifact: String, to: String },
    ReplaceFile { id: String, from: String, path: String, mode: String },
    BackupRegistry { id: String, values: Vec<RegistryRef> },
    SetRegistry { id: String, hive: String, key: String, name: String, reg_type: String, data: serde_json::Value },
    RemoveRegistryValue { id: String, hive: String, key: String, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupFileOp {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageFileOp {
    pub id: String,
    pub artifact: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaceFileOp {
    pub id: String,
    pub from: String,
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRegistryOp {
    pub id: String,
    pub values: Vec<RegistryRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRegistryOp {
    pub hive: String,
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub reg_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveRegistryValueOp {
    pub hive: String,
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Check {
    FileSha256 { id: String, path: String, equals: String },
    RegistryValue { id: String, hive: String, key: String, name: String, reg_type: String, data: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoRemediate {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub scheduled_task_fallback: bool,
    #[serde(default)]
    pub debounce_seconds: u32,
    #[serde(default)]
    pub respect_updates: bool,
    #[serde(default)]
    pub watch_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revert {
    #[serde(default)]
    pub after_journal: Vec<Operation>,
}

/// One supported source build: the stock hash of the target file and the
/// artifact that replaces it.
///
/// A binary patch of a system DLL is build-specific. Listing the stock hash here
/// lets one manifest carry several builds while still refusing any build it has
/// no artifact for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildVariant {
    /// SHA-256 of the untouched target file on this build.
    pub stock_sha256: String,
    /// Artifact id (from `artifacts`) that replaces it.
    pub artifact: String,
}

/// Result of matching the current file against the declared build variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantSelection {
    /// The file matches a stock hash and this artifact should be installed.
    Matched(String),
    /// The file already carries one of the patched artifacts.
    AlreadyApplied(String),
    /// No variant covers this file.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchManifest {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub severity: String,
    #[serde(default)]
    pub cve: Vec<String>,
    pub target: Target,
    pub requires: Requires,
    pub docs: Docs,
    pub applies_to: AppliesTo,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub preconditions: Vec<Precondition>,
    #[serde(default)]
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub checks: Vec<Check>,
    /// Supported source builds. When present, the engine selects the artifact
    /// from the current file hash and `preconditions` must be empty.
    #[serde(default)]
    pub build_variants: Vec<BuildVariant>,
    #[serde(default)]
    pub auto_remediate: Option<AutoRemediate>,
    #[serde(default)]
    pub revert: Option<Revert>,
}

impl PatchManifest {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest: {}", path.display()))?;
        let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
        let manifest: PatchManifest = serde_norway::from_str(content)
            .with_context(|| format!("failed to parse manifest: {}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_variants()?;
        if self.schema != MANIFEST_SCHEMA {
            anyhow::bail!("unsupported schema: {}", self.schema);
        }
        if self.id.is_empty() {
            anyhow::bail!("id cannot be empty");
        }
        for artifact in &self.artifacts {
            if artifact.sha256.len() != 64 || !artifact.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                anyhow::bail!("invalid sha256 for artifact {}", artifact.id);
            }
        }
        for pre in &self.preconditions {
            if let Precondition::FileSha256 { one_of, .. } = pre {
                for h in one_of {
                    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
                        anyhow::bail!("invalid sha256 in precondition");
                    }
                }
            }
        }
        for check in &self.checks {
            if let Check::FileSha256 { equals, .. } = check {
                // "$selected" is resolved at runtime from the matched build variant.
                if equals != "$selected"
                    && (equals.len() != 64 || !equals.chars().all(|c| c.is_ascii_hexdigit()))
                {
                    anyhow::bail!("invalid sha256 in check");
                }
            }
        }

        // Auto-remediation is reserved in the schema but has no validated
        // implementation yet. Refusing here keeps a manifest from advertising a
        // capability the engine cannot deliver.
        if let Some(auto) = &self.auto_remediate {
            if auto.enabled && auto.watch_paths.is_empty() && self.variant_target_path().is_none() {
                anyhow::bail!(
                    "auto_remediate.enabled needs watch_paths, or build_variants whose artifact declares target_path"
                );
            }
        }
        Ok(())
    }
}

impl PatchManifest {
    /// Picks the variant that matches the current file hash.
    pub fn select_variant(&self, current_sha256: &str) -> VariantSelection {
        if self.build_variants.is_empty() {
            return VariantSelection::Unsupported;
        }

        let current = current_sha256.to_lowercase();

        for variant in &self.build_variants {
            if variant.stock_sha256.to_lowercase() == current {
                return VariantSelection::Matched(variant.artifact.clone());
            }
        }

        for artifact in &self.artifacts {
            if artifact.sha256.to_lowercase() == current {
                return VariantSelection::AlreadyApplied(artifact.id.clone());
            }
        }

        VariantSelection::Unsupported
    }

    /// The file these variants replace, if the manifest declares any.
    pub fn variant_target_path(&self) -> Option<&str> {
        self.build_variants.iter().find_map(|variant| {
            self.artifacts
                .iter()
                .find(|artifact| artifact.id == variant.artifact)
                .and_then(|artifact| artifact.target_path.as_deref())
        })
    }

    /// Every patched hash, used to verify without knowing which build was patched.
    pub fn patched_hashes(&self) -> Vec<String> {
        self.artifacts
            .iter()
            .map(|artifact| artifact.sha256.to_lowercase())
            .collect()
    }

    fn validate_variants(&self) -> Result<()> {
        if self.build_variants.is_empty() {
            for operation in &self.operations {
                if let Operation::StageFile { artifact, .. } = operation {
                    if artifact == "$selected" {
                        anyhow::bail!("stage_file.artifact $selected requires build_variants");
                    }
                }
            }
            for check in &self.checks {
                if let Check::FileSha256 { equals, .. } = check {
                    if equals == "$selected" {
                        anyhow::bail!("checks.equals $selected requires build_variants");
                    }
                }
            }
            return Ok(());
        }

        if !self.preconditions.is_empty() {
            anyhow::bail!(
                "build_variants and preconditions are mutually exclusive: the variant list already decides which builds are supported"
            );
        }

        let mut seen_stock = std::collections::HashSet::new();
        let mut target_paths = std::collections::HashSet::new();

        for variant in &self.build_variants {
            if variant.stock_sha256.len() != 64
                || !variant.stock_sha256.chars().all(|c| c.is_ascii_hexdigit())
            {
                anyhow::bail!("invalid stock_sha256 in build_variants");
            }
            if !seen_stock.insert(variant.stock_sha256.to_lowercase()) {
                anyhow::bail!("duplicate stock_sha256 in build_variants");
            }

            let artifact = self
                .artifacts
                .iter()
                .find(|artifact| artifact.id == variant.artifact)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "build_variants references unknown artifact '{}'",
                        variant.artifact
                    )
                })?;

            let path = artifact.target_path.clone().unwrap_or_default();
            if path.is_empty() {
                anyhow::bail!(
                    "artifact '{}' must declare target_path to be used in build_variants",
                    artifact.id
                );
            }
            target_paths.insert(path);
        }

        if target_paths.len() != 1 {
            anyhow::bail!("all build_variants must target the same file");
        }

        Ok(())
    }
}
