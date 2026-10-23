//! Release packaging, manifest generation, and pre-release validation.
//!
//! A release spec describes the version, channel, validity window, notes,
//! limitations, rollback steps, and artifact files. The builder hashes every
//! artifact, validates the resulting manifest, and stages a signed envelope that
//! the updater can verify.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use catalog_schema::SchemaVersion;
use ed25519_dalek::SigningKey;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use updater::{sign_manifest, ReleaseArtifact, ReleaseChannel, ReleaseManifest, SignedManifest};

/// File name of the signed manifest written by the packager.
pub const MANIFEST_FILE: &str = "release-manifest.json";

/// Directory that receives the packaged artifacts.
pub const ARTIFACTS_DIR: &str = "artifacts";

/// Description of a release before hashes are computed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSpec {
    /// Semantic version offered by this release.
    pub version: String,
    /// Release channel.
    pub channel: ReleaseChannel,
    /// Unix timestamp when the manifest becomes valid.
    pub published_at: u64,
    /// Unix timestamp after which the manifest is no longer valid.
    pub expires_at: u64,
    /// Oldest client version allowed to install this release.
    #[serde(default)]
    pub minimum_version: Option<String>,
    /// Base HTTPS URL where the staged artifacts will be published.
    pub base_url: String,
    /// Identifier of the signing key written into the envelope.
    pub key_id: String,
    /// User-facing release notes.
    pub notes: Vec<String>,
    /// Known limitations of the release.
    #[serde(default)]
    pub limitations: Vec<String>,
    /// Steps required to return to the previous version.
    pub rollback_steps: Vec<String>,
    /// Artifact files included in the release.
    pub artifacts: Vec<ArtifactSpec>,
}

/// Source file for one release artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSpec {
    /// Stable artifact identifier written into the manifest.
    pub id: String,
    /// Local path to the artifact file.
    pub path: PathBuf,
}

/// Artifact copied and hashed by the packager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedArtifact {
    /// Stable artifact identifier.
    pub id: String,
    /// Source path on the build machine.
    pub source: PathBuf,
    /// File name used in the release directory and download URL.
    pub file_name: String,
    /// Lowercase SHA-256 digest of the source file.
    pub sha256: String,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Manifest and artifacts ready to be signed and published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCandidate {
    /// Identifier of the key that should sign the manifest.
    pub key_id: String,
    /// Manifest generated from the release spec.
    pub manifest: ReleaseManifest,
    /// Artifacts that back the manifest.
    pub artifacts: Vec<PackagedArtifact>,
}

impl ReleaseCandidate {
    /// Builds a release candidate from a spec.
    ///
    /// The builder reads every artifact from disk, computes its SHA-256 digest
    /// and size, and validates the resulting manifest before returning it. Notes,
    /// rollback steps, and at least one artifact are required so a published
    /// release never lacks the information users need.
    pub fn build(spec: ReleaseSpec) -> Result<Self> {
        let version =
            Version::parse(&spec.version).context("release version is not valid semver")?;

        if !spec.base_url.starts_with("https://") {
            bail!("base URL must use HTTPS");
        }

        if spec.expires_at <= spec.published_at {
            bail!("release validity window must end after it starts");
        }

        let notes = require_lines(&spec.notes, "release notes")?;
        let limitations = clean_lines(&spec.limitations, "limitations")?;
        let rollback_steps = require_lines(&spec.rollback_steps, "rollback steps")?;

        if let Some(minimum) = &spec.minimum_version {
            let minimum = Version::parse(minimum).context("minimum version is not valid semver")?;

            if minimum > version {
                bail!("minimum version {minimum} is newer than the offered version {version}");
            }
        }

        if spec.artifacts.is_empty() {
            bail!("release must include at least one artifact");
        }

        let base_url = spec.base_url.trim_end_matches('/');
        let mut seen_names = Vec::new();
        let mut artifacts = Vec::new();

        for artifact in &spec.artifacts {
            if artifact.id.trim().is_empty() {
                bail!("artifact id must not be empty");
            }

            let file_name = artifact
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .with_context(|| {
                    format!(
                        "artifact {} has no usable file name: {}",
                        artifact.id,
                        artifact.path.display()
                    )
                })?
                .to_string();

            if seen_names.contains(&file_name) {
                bail!("artifact file name is duplicated: {file_name}");
            }

            let (sha256, size_bytes) = hash_file(&artifact.path)?;

            if size_bytes == 0 {
                bail!(
                    "artifact {} is empty: {}",
                    artifact.id,
                    artifact.path.display()
                );
            }

            let url = format!("{base_url}/{file_name}");
            let manifest_artifact = ReleaseArtifact {
                id: artifact.id.clone(),
                file_name: file_name.clone(),
                sha256: sha256.clone(),
                size_bytes,
                url,
            };

            manifest_artifact
                .validate()
                .with_context(|| format!("artifact {} is invalid", artifact.id))?;

            seen_names.push(file_name.clone());

            artifacts.push(PackagedArtifact {
                id: artifact.id.clone(),
                source: artifact.path.clone(),
                file_name,
                sha256,
                size_bytes,
            });
        }

        let manifest = ReleaseManifest {
            schema_version: SchemaVersion::current(),
            version: version.to_string(),
            channel: spec.channel,
            published_at: spec.published_at,
            expires_at: spec.expires_at,
            minimum_version: spec.minimum_version,
            notes,
            limitations,
            rollback_steps,
            artifacts: artifacts
                .iter()
                .map(|artifact| ReleaseArtifact {
                    id: artifact.id.clone(),
                    file_name: artifact.file_name.clone(),
                    sha256: artifact.sha256.clone(),
                    size_bytes: artifact.size_bytes,
                    url: format!("{base_url}/{}", artifact.file_name),
                })
                .collect(),
        };

        Ok(Self {
            key_id: spec.key_id,
            manifest,
            artifacts,
        })
    }

    /// Copies the artifacts into `out_dir/artifacts` and returns that directory.
    pub fn stage_artifacts(&self, out_dir: &Path) -> Result<PathBuf> {
        let artifacts_dir = out_dir.join(ARTIFACTS_DIR);
        fs::create_dir_all(&artifacts_dir)
            .with_context(|| format!("failed to create {}", artifacts_dir.display()))?;

        for artifact in &self.artifacts {
            let destination = artifacts_dir.join(&artifact.file_name);

            if same_file(&artifact.source, &destination) {
                continue;
            }

            fs::copy(&artifact.source, &destination).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    artifact.source.display(),
                    destination.display()
                )
            })?;
        }

        Ok(artifacts_dir)
    }

    /// Signs the manifest with the given key.
    pub fn sign(&self, signing_key: &SigningKey) -> Result<SignedManifest> {
        sign_manifest(self.manifest.clone(), self.key_id.clone(), signing_key)
            .context("failed to sign release manifest")
    }

    /// Writes the signed manifest into `out_dir` and returns its path.
    pub fn write_manifest(&self, out_dir: &Path, signed: &SignedManifest) -> Result<PathBuf> {
        fs::create_dir_all(out_dir)
            .with_context(|| format!("failed to create {}", out_dir.display()))?;

        let path = out_dir.join(MANIFEST_FILE);
        let bytes = serde_json::to_vec_pretty(signed).context("failed to encode manifest")?;
        fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;

        Ok(path)
    }

    /// Stages artifacts and writes a signed manifest into `out_dir`.
    pub fn package(&self, out_dir: &Path, signing_key: &SigningKey) -> Result<PathBuf> {
        self.stage_artifacts(out_dir)?;

        let signed = self.sign(signing_key)?;

        self.write_manifest(out_dir, &signed)
    }
}

/// Decodes a base64 Ed25519 signing key seed.
pub fn signing_key_from_base64(value: &str) -> Result<SigningKey> {
    let bytes = Base64
        .decode(value)
        .context("signing key is not valid base64")?;

    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("signing key must contain exactly 32 bytes"))?;

    Ok(SigningKey::from_bytes(&seed))
}

/// Returns the base64 public key for a signing key.
pub fn public_key_base64(signing_key: &SigningKey) -> String {
    Base64.encode(signing_key.verifying_key().to_bytes())
}

fn hash_file(path: &Path) -> Result<(String, u64)> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let size = io::copy(&mut file, &mut hasher)
        .with_context(|| format!("failed to read {}", path.display()))?;

    Ok((format!("{:x}", hasher.finalize()), size))
}

fn require_lines(values: &[String], field: &str) -> Result<Vec<String>> {
    let lines = clean_lines(values, field)?;

    if lines.is_empty() {
        bail!("{field} must contain at least one entry");
    }

    Ok(lines)
}

fn clean_lines(values: &[String], field: &str) -> Result<Vec<String>> {
    values
        .iter()
        .map(|value| {
            let trimmed = value.trim();

            if trimmed.is_empty() {
                bail!("{field} must not contain empty entries");
            }

            Ok(trimmed.to_string())
        })
        .collect()
}

fn same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
