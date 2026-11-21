use anyhow::{bail, Context, Result};
use catalog_schema::{validation, PatchEntry, Validate};
use compatibility_catalog::CompatibilityEntry;
use serde_json::Value;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    validation::run_validator_cli(
        "compatibility-validate",
        "Validates device, driver, and software compatibility entries.",
        "entries",
        validate_contract,
    )
}

fn validate_contract(value: &Value) -> Result<Option<&'static str>> {
    let object = value
        .as_object()
        .context("document must be a JSON object")?;

    if object.contains_key("cves") {
        let entry: PatchEntry =
            serde_json::from_value(value.clone()).context("invalid patch entry")?;
        entry.validate()?;
        return Ok(None);
    }

    if !object.contains_key("tested_builds") || !object.contains_key("sources") {
        bail!("not a compatibility entry: missing tested_builds or sources");
    }

    let entry: CompatibilityEntry =
        serde_json::from_value(value.clone()).context("invalid compatibility entry")?;
    entry.validate()?;
    Ok(None)
}
