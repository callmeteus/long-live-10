use anyhow::{bail, Context, Result};
use catalog_schema::{Artifact, PatchEntry, TargetBuild, Validate, ValidationResult};
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if arguments
        .iter()
        .any(|value| value == "--help" || value == "-h")
    {
        print_usage();
        return Ok(());
    }

    let mut roots: Vec<PathBuf> = arguments.into_iter().map(PathBuf::from).collect();

    if roots.is_empty() {
        roots.push(PathBuf::from("catalogs"));
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
        match validate_file(file) {
            Ok(kind) => println!("ok {} ({kind})", file.display()),
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

fn print_usage() {
    println!("Usage: catalog-validate [path ...]");
    println!();
    println!("Validates patch, artifact, target build, and validation result JSON files.");
    println!("Paths may be files or directories. Defaults to ./catalogs.");
}

fn validate_file(path: &Path) -> Result<&'static str> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&text).context("invalid JSON")?;
    let object = value
        .as_object()
        .context("document must be a JSON object")?;

    if object.contains_key("cves") {
        let entry: PatchEntry = serde_json::from_value(value).context("invalid patch entry")?;
        entry.validate()?;
        return Ok("patch");
    }

    if object.contains_key("file_name") {
        let artifact: Artifact = serde_json::from_value(value).context("invalid artifact entry")?;
        artifact.validate()?;
        return Ok("artifact");
    }

    if object.contains_key("product_name") {
        let build: TargetBuild = serde_json::from_value(value).context("invalid target build")?;
        build.validate()?;
        return Ok("target build");
    }

    if object.contains_key("checks") {
        let result: ValidationResult =
            serde_json::from_value(value).context("invalid validation result")?;
        result.validate()?;
        return Ok("validation result");
    }

    bail!("cannot determine contract type; expected patch, artifact, target build, or validation result")
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
        let entry =
            entry.with_context(|| format!("failed to read directory {}", path.display()))?;
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
