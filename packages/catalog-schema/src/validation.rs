//! Shared validation primitives.
//!
//! These functions answer a yes/no question only. Each crate maps a `false`
//! into its own error type and message, so the on-disk and CLI error wording
//! stays untouched while the rule itself lives in one place.

/// True when a required text field has content after trimming.
pub fn is_nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

/// True for a 64-character hexadecimal digest.
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// True when a URL uses HTTPS, the only accepted source scheme.
pub fn is_https_url(value: &str) -> bool {
    value.starts_with("https://")
}

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Runs the shared CLI driver for a catalog validator.
///
/// The caller supplies only the validator name, its help text, the default
/// directory and the function that maps a JSON value to a contract kind. The
/// flags, output lines, default path and failure count stay identical across
/// validators.
pub fn run_validator_cli(
    name: &str,
    description: &str,
    default_dir: &str,
    validate: fn(&Value) -> Result<Option<&'static str>>,
) -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if arguments.iter().any(|value| value == "--help" || value == "-h") {
        println!("Usage: {name} [path ...]");
        println!();
        println!("{description}");
        println!("Paths may be files or directories. Defaults to ./{default_dir}.");
        return Ok(());
    }

    let mut roots: Vec<PathBuf> = arguments.into_iter().map(PathBuf::from).collect();
    if roots.is_empty() {
        roots.push(PathBuf::from(default_dir));
    }

    let mut files = Vec::new();
    for root in &roots {
        collect_json_files(root, &mut files)?;
    }
    files.sort();

    if files.is_empty() {
        bail!("no JSON files found in the given paths");
    }

    let mut failures = 0;
    for file in &files {
        match validate_file(file, validate) {
            Ok(Some(kind)) => println!("ok {} ({kind})", file.display()),
            Ok(None) => println!("ok {}", file.display()),
            Err(error) => {
                eprintln!("error {}: {error:#}", file.display());
                failures += 1;
            }
        }
    }

    println!("{} file(s), {} failure(s)", files.len(), failures);
    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn validate_file(
    path: &Path,
    validate: fn(&Value) -> Result<Option<&'static str>>,
) -> Result<Option<&'static str>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&text).context("invalid JSON")?;
    validate(&value)
}

fn collect_json_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to inspect {}", path.display()))?;

    if metadata.is_file() {
        if path.extension() != Some(OsStr::new("json")) {
            bail!("not a JSON file: {}", path.display());
        }
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !metadata.is_dir() {
        bail!("unsupported path type: {}", path.display());
    }

    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read directory {}", path.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read directory {}", path.display()))?;
        let child = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", child.display()))?;
        if file_type.is_dir() {
            collect_json_files(&child, files)?;
        } else if file_type.is_file() && child.extension() == Some(OsStr::new("json")) {
            files.push(child);
        }
    }
    Ok(())
}
