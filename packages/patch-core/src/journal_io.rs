//! Shared journal file I/O.
//!
//! The engine and the CLI keep their own on-disk journal formats (both are
//! protected surfaces), but the mechanics of reading and writing a JSON file
//! are identical. Keeping them here avoids two subtly different error paths.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;

/// Reads a JSON document from `path`, with a consistent context message.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read journal {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid journal JSON in {}", path.display()))
}

/// Writes a JSON document to `path`, creating parent directories as needed.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create journal directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value).context("failed to serialize journal")?;
    fs::write(path, content).with_context(|| format!("failed to write journal {}", path.display()))
}
