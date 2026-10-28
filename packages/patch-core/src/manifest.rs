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
                if equals.len() != 64 || !equals.chars().all(|c| c.is_ascii_hexdigit()) {
                    anyhow::bail!("invalid sha256 in check");
                }
            }
        }
        Ok(())
    }
}
