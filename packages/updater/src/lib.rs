//! Release manifest fetching and Ed25519 signature verification.
//!
//! A manifest is delivered as a signed envelope:
//!
//! ```json
//! {
//!     "key_id": "release-2026-a",
//!     "manifest": { "...": "..." },
//!     "signature": "base64"
//! }
//! ```
//!
//! The signature covers the canonical JSON serialization of `manifest` with the
//! field order used by [`ReleaseManifest`]. The updater verifies the signature
//! before validating expiry, channel, and downgrade rules.

use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use catalog_schema::{SchemaError, SchemaVersion, Validate};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::io::Read;
use thiserror::Error;

/// Release channel used by a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    /// General availability builds.
    Stable,
    /// Preview builds for opt-in testers.
    Beta,
    /// Internal builds that are never offered automatically.
    Nightly,
}

/// Downloadable artifact described by a release manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    /// Stable artifact identifier.
    pub id: String,
    /// File name without directories.
    pub file_name: String,
    /// Lowercase SHA-256 digest.
    pub sha256: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// HTTPS download location.
    pub url: String,
}

impl ReleaseArtifact {
    /// Validates the artifact metadata.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.trim().is_empty() {
            return Err(ManifestError::EmptyField {
                field: "artifacts[].id",
            });
        }

        if self.file_name.trim().is_empty() {
            return Err(ManifestError::EmptyField {
                field: "artifacts[].file_name",
            });
        }

        if self.file_name.contains('/') || self.file_name.contains('\\') {
            return Err(ManifestError::InvalidFileName {
                value: self.file_name.clone(),
            });
        }

        if self.sha256.len() != 64 || !self.sha256.chars().all(|value| value.is_ascii_hexdigit()) {
            return Err(ManifestError::InvalidSha256 {
                value: self.sha256.clone(),
            });
        }

        if self.size_bytes == 0 {
            return Err(ManifestError::EmptyArtifact);
        }

        if !self.url.starts_with("https://") {
            return Err(ManifestError::InsecureUrl {
                value: self.url.clone(),
            });
        }

        Ok(())
    }
}

/// Release manifest signed by a trusted key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    /// Contract version used by the manifest.
    pub schema_version: SchemaVersion,
    /// Semantic version offered by this manifest.
    pub version: String,
    /// Release channel.
    pub channel: ReleaseChannel,
    /// Unix timestamp when the manifest becomes valid.
    pub published_at: u64,
    /// Unix timestamp after which the manifest is no longer valid.
    pub expires_at: u64,
    /// Oldest client version allowed to install this release.
    pub minimum_version: Option<String>,
    /// Artifacts included in the release.
    pub artifacts: Vec<ReleaseArtifact>,
}

impl ReleaseManifest {
    /// Validates the manifest against the clock and the installed client.
    pub fn validate(&self, now: u64, current_version: &str) -> Result<(), ManifestError> {
        self.schema_version.validate()?;

        let offered = Version::parse(&self.version).map_err(|_| ManifestError::InvalidVersion {
            value: self.version.clone(),
        })?;

        let current =
            Version::parse(current_version).map_err(|_| ManifestError::InvalidVersion {
                value: current_version.to_string(),
            })?;

        if self.expires_at <= self.published_at {
            return Err(ManifestError::InvalidValidityWindow {
                published_at: self.published_at,
                expires_at: self.expires_at,
            });
        }

        if now < self.published_at {
            return Err(ManifestError::NotYetValid {
                published_at: self.published_at,
                now,
            });
        }

        if now > self.expires_at {
            return Err(ManifestError::Expired {
                expires_at: self.expires_at,
                now,
            });
        }

        if offered <= current {
            return Err(ManifestError::Downgrade {
                current: current.to_string(),
                offered: offered.to_string(),
            });
        }

        if let Some(minimum) = &self.minimum_version {
            let minimum = Version::parse(minimum).map_err(|_| ManifestError::InvalidVersion {
                value: minimum.clone(),
            })?;

            if current < minimum {
                return Err(ManifestError::UpgradeRequired {
                    minimum: minimum.to_string(),
                    current: current.to_string(),
                });
            }
        }

        if self.artifacts.is_empty() {
            return Err(ManifestError::EmptyField { field: "artifacts" });
        }

        for artifact in &self.artifacts {
            artifact.validate()?;
        }

        Ok(())
    }
}

/// Public key trusted to sign release manifests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedKey {
    /// Stable key identifier written into the envelope.
    pub id: String,
    /// Raw Ed25519 public key bytes.
    pub public_key: [u8; 32],
}

impl TrustedKey {
    /// Creates a trusted key from raw bytes.
    pub fn new(id: impl Into<String>, public_key: [u8; 32]) -> Self {
        Self {
            id: id.into(),
            public_key,
        }
    }

    /// Creates a trusted key from a base64-encoded public key.
    pub fn from_base64(id: impl Into<String>, value: &str) -> Result<Self, ManifestError> {
        let bytes = Base64
            .decode(value)
            .map_err(|_| ManifestError::InvalidPublicKey)?;

        let public_key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ManifestError::InvalidPublicKey)?;

        Ok(Self {
            id: id.into(),
            public_key,
        })
    }
}

/// Set of trusted release keys.
#[derive(Debug, Clone, Default)]
pub struct KeyRing {
    keys: Vec<TrustedKey>,
}

impl KeyRing {
    /// Creates a key ring from the given keys.
    pub fn new(keys: Vec<TrustedKey>) -> Self {
        Self { keys }
    }

    /// Adds a key to the ring.
    pub fn add(&mut self, key: TrustedKey) {
        self.keys.push(key);
    }

    /// Finds a key by identifier.
    pub fn find(&self, id: &str) -> Option<&TrustedKey> {
        self.keys.iter().find(|key| key.id == id)
    }
}

/// Signed manifest envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedManifest {
    /// Identifier of the key that produced the signature.
    pub key_id: String,
    /// Manifest covered by the signature.
    pub manifest: ReleaseManifest,
    /// Base64-encoded Ed25519 signature over the canonical manifest JSON.
    #[serde(default)]
    pub signature: Option<String>,
}

impl SignedManifest {
    /// Verifies the signature and validates the manifest.
    pub fn verify(
        &self,
        keyring: &KeyRing,
        now: u64,
        current_version: &str,
    ) -> Result<(), ManifestError> {
        let encoded = self
            .signature
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(ManifestError::MissingSignature)?;

        let key = keyring
            .find(&self.key_id)
            .ok_or_else(|| ManifestError::UnknownKey {
                key_id: self.key_id.clone(),
            })?;

        let signature_bytes = Base64
            .decode(encoded)
            .map_err(|_| ManifestError::InvalidSignatureEncoding)?;

        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| ManifestError::InvalidSignature)?;

        let verifying_key = VerifyingKey::from_bytes(&key.public_key)
            .map_err(|_| ManifestError::InvalidPublicKey)?;

        let message = manifest_bytes(&self.manifest)?;

        verifying_key
            .verify(&message, &signature)
            .map_err(|_| ManifestError::InvalidSignature)?;

        self.manifest.validate(now, current_version)
    }
}

/// Produces the canonical bytes covered by a manifest signature.
pub fn manifest_bytes(manifest: &ReleaseManifest) -> Result<Vec<u8>, ManifestError> {
    serde_json::to_vec(manifest).map_err(|error| ManifestError::ManifestEncoding {
        message: error.to_string(),
    })
}

/// Creates a signed envelope for the given manifest and key.
pub fn sign_manifest(
    manifest: ReleaseManifest,
    key_id: impl Into<String>,
    signing_key: &SigningKey,
) -> Result<SignedManifest, ManifestError> {
    let message = manifest_bytes(&manifest)?;
    let signature = signing_key.sign(&message);

    Ok(SignedManifest {
        key_id: key_id.into(),
        manifest,
        signature: Some(Base64.encode(signature.to_bytes())),
    })
}

/// Fetches raw manifest bytes from a location.
pub trait ManifestTransport {
    /// Fetches the envelope bytes for a URL.
    fn fetch(&self, url: &str) -> Result<Vec<u8>, ManifestError>;
}

/// Fetches manifests over HTTPS.
#[derive(Debug, Clone, Copy, Default)]
pub struct HttpTransport;

impl ManifestTransport for HttpTransport {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, ManifestError> {
        if !url.starts_with("https://") {
            return Err(ManifestError::InsecureUrl {
                value: url.to_string(),
            });
        }

        let response = ureq::get(url)
            .call()
            .map_err(|error| ManifestError::FetchFailed {
                message: error.to_string(),
            })?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|error| ManifestError::FetchFailed {
                message: error.to_string(),
            })?;

        Ok(bytes)
    }
}

/// Fetch and verify a manifest with a transport.
pub struct ManifestFetcher<T> {
    transport: T,
}

impl<T: ManifestTransport> ManifestFetcher<T> {
    /// Creates a fetcher over a transport.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Fetches, parses, and verifies a manifest envelope.
    pub fn fetch_and_verify(
        &self,
        url: &str,
        keyring: &KeyRing,
        now: u64,
        current_version: &str,
    ) -> Result<ReleaseManifest, ManifestError> {
        let bytes = self.transport.fetch(url)?;

        let signed: SignedManifest =
            serde_json::from_slice(&bytes).map_err(|error| ManifestError::ManifestEncoding {
                message: error.to_string(),
            })?;

        signed.verify(keyring, now, current_version)?;

        Ok(signed.manifest)
    }
}

/// Problems found while fetching or verifying a release manifest.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ManifestError {
    /// Shared catalog schema problem.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// Required text field is empty.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },
    /// Artifact file name contains a path separator.
    #[error("artifact file name must not contain path separators: {value}")]
    InvalidFileName {
        /// Value found in the manifest.
        value: String,
    },
    /// Artifact digest is not a lowercase SHA-256 value.
    #[error("invalid SHA-256 value: {value}")]
    InvalidSha256 {
        /// Value found in the manifest.
        value: String,
    },
    /// Artifact has no content.
    #[error("artifact size must be greater than zero")]
    EmptyArtifact,
    /// URL is not HTTPS.
    #[error("URL must use HTTPS: {value}")]
    InsecureUrl {
        /// Value found in the manifest or request.
        value: String,
    },
    /// Manifest has no signature.
    #[error("manifest is not signed")]
    MissingSignature,
    /// Signature is not valid base64.
    #[error("manifest signature is not valid base64")]
    InvalidSignatureEncoding,
    /// Signature does not match the manifest.
    #[error("manifest signature does not match")]
    InvalidSignature,
    /// Key identifier is not trusted.
    #[error("manifest was signed by an unknown key: {key_id}")]
    UnknownKey {
        /// Identifier written into the envelope.
        key_id: String,
    },
    /// Public key is malformed.
    #[error("trusted public key is malformed")]
    InvalidPublicKey,
    /// Manifest JSON cannot be serialized for verification.
    #[error("manifest encoding failed: {message}")]
    ManifestEncoding {
        /// Serialization problem.
        message: String,
    },
    /// Version string is not valid semver.
    #[error("invalid semantic version: {value}")]
    InvalidVersion {
        /// Value found in the manifest.
        value: String,
    },
    /// Manifest validity window is reversed or empty.
    #[error("manifest validity window is invalid: {published_at} to {expires_at}")]
    InvalidValidityWindow {
        /// Start of the window.
        published_at: u64,
        /// End of the window.
        expires_at: u64,
    },
    /// Manifest is not valid yet.
    #[error("manifest is not valid before {published_at}, current time is {now}")]
    NotYetValid {
        /// Start of the window.
        published_at: u64,
        /// Current time.
        now: u64,
    },
    /// Manifest has expired.
    #[error("manifest expired at {expires_at}, current time is {now}")]
    Expired {
        /// End of the window.
        expires_at: u64,
        /// Current time.
        now: u64,
    },
    /// Offered version is not newer than the installed version.
    #[error("manifest version {offered} is not newer than installed version {current}")]
    Downgrade {
        /// Installed version.
        current: String,
        /// Offered version.
        offered: String,
    },
    /// Client is older than the minimum supported version.
    #[error("client version {current} is below the minimum {minimum}")]
    UpgradeRequired {
        /// Minimum supported version.
        minimum: String,
        /// Installed version.
        current: String,
    },
    /// Manifest transport failed.
    #[error("manifest fetch failed: {message}")]
    FetchFailed {
        /// Transport error.
        message: String,
    },
}
