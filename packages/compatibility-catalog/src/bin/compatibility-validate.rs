use anyhow::{bail, Context, Result};
use compatibility_catalog::CompatibilityEntry;
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
        roots.push(PathBuf::from("entries"));
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
            Ok(()) => println!("ok {}", file.display()),
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
    println!("Usage: compatibility-validate [path ...]");
    println!();
    println!("Validates device, driver, and software compatibility entries.");
    println!("Paths may be files or directories. Defaults to ./entries.");
}

fn validate_file(path: &Path) -> Result<()> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&text).context("invalid JSON")?;
    let object = value
        .as_object()
        .context("document must be a JSON object")?;

    if !object.contains_key("tested_builds") || !object.contains_key("sources") {
        bail!("not a compatibility entry: missing tested_builds or sources");
    }

    let entry: CompatibilityEntry =
        serde_json::from_value(value).context("invalid compatibility entry")?;

    entry.validate()?;

    Ok(())
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
