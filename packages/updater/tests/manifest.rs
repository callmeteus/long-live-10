use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use catalog_schema::SchemaVersion;
use ed25519_dalek::SigningKey;
use std::sync::Mutex;
use updater::{
    sign_manifest, HttpTransport, KeyRing, ManifestError, ManifestFetcher, ManifestTransport,
    ReleaseArtifact, ReleaseChannel, ReleaseManifest, SignedManifest, TrustedKey,
};

const NOW: u64 = 1_800_000_000;
const CHANNEL: ReleaseChannel = ReleaseChannel::Stable;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn keyring() -> KeyRing {
    let key = TrustedKey::new("test-key", signing_key().verifying_key().to_bytes());

    KeyRing::new(vec![key])
}

fn manifest(version: &str) -> ReleaseManifest {
    ReleaseManifest {
        schema_version: SchemaVersion::current(),
        version: version.to_string(),
        channel: CHANNEL,
        published_at: NOW - 60,
        expires_at: NOW + 86_400,
        minimum_version: None,
        artifacts: vec![ReleaseArtifact {
            id: "artifact-1".to_string(),
            file_name: "long-live-10.msi".to_string(),
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            size_bytes: 1024,
            url: "https://example.invalid/long-live-10.msi".to_string(),
        }],
    }
}

fn signed(version: &str) -> SignedManifest {
    sign_manifest(manifest(version), "test-key", &signing_key()).expect("manifest signs")
}

#[test]
fn accepts_a_valid_signed_manifest() {
    let signed = signed("1.1.0");

    signed
        .verify(&keyring(), NOW, "1.0.0")
        .expect("manifest is valid");
}

#[test]
fn rejects_an_unsigned_manifest() {
    let mut signed = signed("1.1.0");
    signed.signature = None;

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::MissingSignature)
    );
}

#[test]
fn rejects_an_empty_signature() {
    let mut signed = signed("1.1.0");
    signed.signature = Some("   ".to_string());

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::MissingSignature)
    );
}

#[test]
fn rejects_a_tampered_manifest() {
    let mut signed = signed("1.1.0");
    signed.manifest.version = "9.9.9".to_string();

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InvalidSignature)
    );
}

#[test]
fn rejects_a_manifest_signed_by_an_unknown_key() {
    let mut signed = signed("1.1.0");
    signed.key_id = "other-key".to_string();

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::UnknownKey {
            key_id: "other-key".to_string()
        })
    );
}

#[test]
fn rejects_an_expired_manifest() {
    let mut signed = signed("1.1.0");
    signed.manifest.expires_at = NOW - 1;
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::Expired { .. })
    ));
}

#[test]
fn rejects_a_manifest_that_is_not_yet_valid() {
    let mut signed = signed("1.1.0");
    signed.manifest.published_at = NOW + 1;
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::NotYetValid { .. })
    ));
}

#[test]
fn rejects_a_downgraded_version() {
    let signed = signed("1.0.0");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "2.0.0"),
        Err(ManifestError::Downgrade { .. })
    ));
}

#[test]
fn rejects_the_same_version() {
    let signed = signed("1.0.0");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::Downgrade { .. })
    ));
}

#[test]
fn rejects_a_client_below_the_minimum_version() {
    let mut signed = signed("1.1.0");
    signed.manifest.minimum_version = Some("1.5.0".to_string());
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::UpgradeRequired { .. })
    ));
}

#[test]
fn rejects_an_invalid_version() {
    let mut signed = signed("1.1.0");
    signed.manifest.version = "not-a-version".to_string();
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InvalidVersion { .. })
    ));
}

#[test]
fn rejects_an_invalid_validity_window() {
    let mut signed = signed("1.1.0");
    signed.manifest.published_at = 200;
    signed.manifest.expires_at = 100;
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InvalidValidityWindow { .. })
    ));
}

#[test]
fn rejects_a_manifest_without_artifacts() {
    let mut signed = signed("1.1.0");
    signed.manifest.artifacts.clear();
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::EmptyField { field: "artifacts" })
    );
}

#[test]
fn rejects_an_insecure_artifact_url() {
    let mut signed = signed("1.1.0");
    signed.manifest.artifacts[0].url = "http://example.invalid/file".to_string();
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InsecureUrl { .. })
    ));
}

#[test]
fn rejects_an_invalid_artifact_digest() {
    let mut signed = signed("1.1.0");
    signed.manifest.artifacts[0].sha256 = "abc".to_string();
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert!(matches!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InvalidSha256 { .. })
    ));
}

#[test]
fn rejects_an_artifact_with_zero_size() {
    let mut signed = signed("1.1.0");
    signed.manifest.artifacts[0].size_bytes = 0;
    let signed =
        sign_manifest(signed.manifest, "test-key", &signing_key()).expect("manifest signs");

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::EmptyArtifact)
    );
}

#[test]
fn rejects_a_bad_signature_encoding() {
    let mut signed = signed("1.1.0");
    signed.signature = Some("not-base64".to_string());

    assert_eq!(
        signed.verify(&keyring(), NOW, "1.0.0"),
        Err(ManifestError::InvalidSignatureEncoding)
    );
}

#[test]
fn accepts_a_key_from_base64() {
    let encoded = Base64.encode(signing_key().verifying_key().to_bytes());
    let key = TrustedKey::from_base64("test-key", &encoded).expect("key decodes");
    let keyring = KeyRing::new(vec![key]);

    signed("1.1.0")
        .verify(&keyring, NOW, "1.0.0")
        .expect("manifest is valid");
}

#[test]
fn rejects_a_malformed_public_key() {
    assert_eq!(
        TrustedKey::from_base64("test-key", "not-base64"),
        Err(ManifestError::InvalidPublicKey)
    );
}

#[test]
fn rejects_a_short_public_key() {
    assert_eq!(
        TrustedKey::from_base64("test-key", &Base64.encode([1u8; 31])),
        Err(ManifestError::InvalidPublicKey)
    );
}

struct StaticTransport {
    response: Mutex<Option<Result<Vec<u8>, ManifestError>>>,
}

impl StaticTransport {
    fn new(response: Result<Vec<u8>, ManifestError>) -> Self {
        Self {
            response: Mutex::new(Some(response)),
        }
    }
}

impl ManifestTransport for StaticTransport {
    fn fetch(&self, _url: &str) -> Result<Vec<u8>, ManifestError> {
        self.response
            .lock()
            .expect("transport lock")
            .take()
            .expect("transport response")
    }
}

#[test]
fn fetches_and_verifies_a_signed_manifest() {
    let signed = signed("1.1.0");
    let bytes = serde_json::to_vec(&signed).expect("envelope serializes");
    let transport = StaticTransport::new(Ok(bytes));
    let fetcher = ManifestFetcher::new(transport);

    let manifest = fetcher
        .fetch_and_verify(
            "https://example.invalid/manifest.json",
            &keyring(),
            NOW,
            "1.0.0",
        )
        .expect("manifest is valid");

    assert_eq!(manifest.version, "1.1.0");
}

#[test]
fn fetch_propagates_transport_errors() {
    let transport = StaticTransport::new(Err(ManifestError::FetchFailed {
        message: "offline".to_string(),
    }));
    let fetcher = ManifestFetcher::new(transport);

    assert_eq!(
        fetcher.fetch_and_verify(
            "https://example.invalid/manifest.json",
            &keyring(),
            NOW,
            "1.0.0"
        ),
        Err(ManifestError::FetchFailed {
            message: "offline".to_string()
        })
    );
}

#[test]
fn fetch_rejects_malformed_envelopes() {
    let transport = StaticTransport::new(Ok(b"{".to_vec()));
    let fetcher = ManifestFetcher::new(transport);

    assert!(matches!(
        fetcher.fetch_and_verify(
            "https://example.invalid/manifest.json",
            &keyring(),
            NOW,
            "1.0.0"
        ),
        Err(ManifestError::ManifestEncoding { .. })
    ));
}

#[test]
fn http_transport_rejects_plain_http() {
    let transport = HttpTransport;

    assert!(matches!(
        transport.fetch("http://example.invalid/manifest.json"),
        Err(ManifestError::InsecureUrl { .. })
    ));
}
