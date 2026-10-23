use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use ed25519_dalek::SigningKey;
use release_tools::{
    public_key_base64, signing_key_from_base64, ArtifactSpec, ReleaseCandidate, ReleaseSpec,
    ARTIFACTS_DIR, MANIFEST_FILE,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use updater::{KeyRing, ReleaseChannel, SignedManifest, TrustedKey};

const NOW: u64 = 1_800_000_000;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[9u8; 32])
}

fn write_artifact(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("artifact writes");

    path
}

fn spec(dir: &Path) -> ReleaseSpec {
    ReleaseSpec {
        version: "1.1.0".to_string(),
        channel: ReleaseChannel::Stable,
        published_at: NOW,
        expires_at: NOW + 86_400,
        minimum_version: None,
        base_url: "https://example.invalid/releases/1.1.0/".to_string(),
        key_id: "release-test".to_string(),
        notes: vec![" Test release. ".to_string()],
        limitations: vec!["Virtual machine only.".to_string()],
        rollback_steps: vec!["Reinstall the previous version.".to_string()],
        artifacts: vec![
            ArtifactSpec {
                id: "installer".to_string(),
                path: write_artifact(dir, "long-live-10.msi", b"abc"),
            },
            ArtifactSpec {
                id: "symbols".to_string(),
                path: write_artifact(dir, "symbols.zip", b"beta"),
            },
        ],
    }
}

#[test]
fn builds_a_manifest_with_hashes_and_metadata() {
    let dir = tempdir().expect("temp dir");
    let candidate = ReleaseCandidate::build(spec(dir.path())).expect("release builds");

    assert_eq!(candidate.key_id, "release-test");
    assert_eq!(candidate.manifest.version, "1.1.0");
    assert_eq!(candidate.manifest.notes, vec!["Test release."]);
    assert_eq!(
        candidate.manifest.limitations,
        vec!["Virtual machine only."]
    );
    assert_eq!(
        candidate.manifest.rollback_steps,
        vec!["Reinstall the previous version."]
    );
    assert_eq!(candidate.artifacts.len(), 2);
    assert_eq!(
        candidate.manifest.artifacts[0].sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(candidate.manifest.artifacts[0].size_bytes, 3);
    assert_eq!(
        candidate.manifest.artifacts[0].url,
        "https://example.invalid/releases/1.1.0/long-live-10.msi"
    );
    assert_eq!(candidate.manifest.artifacts[1].id, "symbols");
}

#[test]
fn stages_and_signs_a_verifiable_release() {
    let dir = tempdir().expect("temp dir");
    let out_dir = dir.path().join("out");
    let candidate = ReleaseCandidate::build(spec(dir.path())).expect("release builds");
    let signing_key = signing_key();

    let manifest_path = candidate
        .package(&out_dir, &signing_key)
        .expect("release packages");

    assert_eq!(manifest_path, out_dir.join(MANIFEST_FILE));
    assert_eq!(
        fs::read(out_dir.join(ARTIFACTS_DIR).join("long-live-10.msi")).expect("artifact is staged"),
        b"abc"
    );

    let signed: SignedManifest =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest is written"))
            .expect("manifest parses");
    let keyring = KeyRing::new(vec![TrustedKey::new(
        "release-test",
        signing_key.verifying_key().to_bytes(),
    )]);

    signed
        .verify(&keyring, NOW, "0.0.0")
        .expect("manifest verifies");
    assert_eq!(
        signed
            .verify_artifact(&keyring, NOW, "0.0.0", "installer")
            .expect("artifact verifies")
            .file_name,
        "long-live-10.msi"
    );
}

#[test]
fn reuses_an_artifact_already_in_the_output_directory() {
    let dir = tempdir().expect("temp dir");
    let out_dir = dir.path().join("out");
    let artifacts_dir = out_dir.join(ARTIFACTS_DIR);
    fs::create_dir_all(&artifacts_dir).expect("artifact directory");
    let artifact = write_artifact(&artifacts_dir, "long-live-10.msi", b"abc");
    let mut release = spec(dir.path());
    release.artifacts[0].path = artifact.clone();
    release.artifacts.truncate(1);

    let candidate = ReleaseCandidate::build(release).expect("release builds");

    candidate
        .stage_artifacts(&out_dir)
        .expect("artifacts stage without copying onto themselves");
    assert_eq!(fs::read(artifact).expect("artifact is intact"), b"abc");
}

#[test]
fn decodes_signing_keys_and_reports_public_keys() {
    let key = signing_key();
    let encoded = Base64.encode(key.to_bytes());

    let decoded = signing_key_from_base64(&encoded).expect("key decodes");

    assert_eq!(decoded.to_bytes(), key.to_bytes());
    assert_eq!(
        public_key_base64(&decoded),
        Base64.encode(key.verifying_key().to_bytes())
    );
}

#[test]
fn rejects_malformed_signing_keys() {
    assert!(signing_key_from_base64("not-base64").is_err());
    assert!(signing_key_from_base64(&Base64.encode([1u8; 31])).is_err());
}

#[test]
fn requires_release_notes() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.notes.clear();

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_empty_note_lines() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.notes = vec!["  ".to_string()];

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn requires_rollback_steps() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.rollback_steps.clear();

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_a_plain_http_base_url() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.base_url = "http://example.invalid/releases".to_string();

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_a_reversed_validity_window() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.expires_at = release.published_at;

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_a_minimum_version_newer_than_the_release() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.minimum_version = Some("2.0.0".to_string());

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_an_invalid_version() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.version = "not-a-version".to_string();

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_a_release_without_artifacts() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.artifacts.clear();

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_a_missing_artifact_file() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.artifacts[0].path = dir.path().join("missing.msi");

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_an_empty_artifact_file() {
    let dir = tempdir().expect("temp dir");
    let mut release = spec(dir.path());
    release.artifacts[0].path = write_artifact(dir.path(), "empty.msi", b"");

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn rejects_duplicate_artifact_file_names() {
    let dir = tempdir().expect("temp dir");
    let first = write_artifact(dir.path(), "installer.msi", b"abc");
    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested).expect("nested directory");
    let second = write_artifact(&nested, "installer.msi", b"def");
    let mut release = spec(dir.path());
    release.artifacts = vec![
        ArtifactSpec {
            id: "first".to_string(),
            path: first,
        },
        ArtifactSpec {
            id: "second".to_string(),
            path: second,
        },
    ];

    assert!(ReleaseCandidate::build(release).is_err());
}

#[test]
fn cli_builds_a_signed_release_from_a_spec() {
    let dir = tempdir().expect("temp dir");
    let release = spec(dir.path());
    let spec_path = dir.path().join("release.json");
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&release).expect("spec serializes"),
    )
    .expect("spec writes");
    let out_dir = dir.path().join("out");
    let seed = Base64.encode(signing_key().to_bytes());

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_release-build"))
        .arg("--spec")
        .arg(&spec_path)
        .arg("--out")
        .arg(&out_dir)
        .env("LL10_SIGNING_KEY", seed)
        .output()
        .expect("release-build runs");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join(MANIFEST_FILE).is_file());
    assert!(out_dir
        .join(ARTIFACTS_DIR)
        .join("long-live-10.msi")
        .is_file());
    assert!(out_dir.join("release-public-key.txt").is_file());

    let signed: SignedManifest =
        serde_json::from_slice(&fs::read(out_dir.join(MANIFEST_FILE)).expect("manifest reads"))
            .expect("manifest parses");
    let keyring = KeyRing::new(vec![TrustedKey::new(
        "release-test",
        signing_key().verifying_key().to_bytes(),
    )]);

    signed
        .verify(&keyring, NOW, "0.0.0")
        .expect("CLI output verifies");
}
