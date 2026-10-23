use anyhow::{bail, Context, Result};
use release_tools::{
    public_key_base64, signing_key_from_base64, ReleaseCandidate, ReleaseSpec, ARTIFACTS_DIR,
    MANIFEST_FILE,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let arguments: Vec<String> = env::args().skip(1).collect();

    if arguments
        .iter()
        .any(|value| value == "--help" || value == "-h")
    {
        print_usage();
        return Ok(());
    }

    let mut spec_path: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut signing_key_value: Option<String> = None;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        let mut next = || -> Result<String> {
            index += 1;
            arguments
                .get(index)
                .cloned()
                .with_context(|| format!("{argument} requires a value"))
        };

        match argument.as_str() {
            "--spec" => spec_path = Some(PathBuf::from(next()?)),
            "--out" => out_dir = Some(PathBuf::from(next()?)),
            "--signing-key" => signing_key_value = Some(next()?),
            other => bail!("unknown argument: {other}"),
        }

        index += 1;
    }

    let spec_path = spec_path.context("missing --spec")?;
    let out_dir = out_dir.context("missing --out")?;
    let signing_key_value = signing_key_value
        .or_else(|| env::var("LL10_SIGNING_KEY").ok())
        .context("missing --signing-key or LL10_SIGNING_KEY")?;

    let spec_text = fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    let spec: ReleaseSpec =
        serde_json::from_str(&spec_text).context("release spec is not valid JSON")?;

    let signing_key = signing_key_from_base64(&signing_key_value)?;
    let candidate = ReleaseCandidate::build(spec)?;
    let manifest_path = candidate.package(&out_dir, &signing_key)?;

    let public_key_path = out_dir.join("release-public-key.txt");
    fs::write(&public_key_path, public_key_base64(&signing_key))
        .with_context(|| format!("failed to write {}", public_key_path.display()))?;

    println!("release {}", candidate.manifest.version);
    println!("artifacts {}", out_dir.join(ARTIFACTS_DIR).display());
    println!("manifest {}", manifest_path.display());
    println!("public key {}", public_key_path.display());
    println!("manifest file {}", MANIFEST_FILE);

    Ok(())
}

fn print_usage() {
    println!("Usage: release-build --spec <file> --out <dir> [--signing-key <base64>]");
    println!();
    println!("Builds, hashes, signs, and stages a release from a JSON spec.");
    println!("The signing key can also be provided through LL10_SIGNING_KEY.");
}
