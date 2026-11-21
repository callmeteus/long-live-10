use anyhow::{bail, Context, Result};
use catalog_schema::{validation, Artifact, PatchEntry, TargetBuild, Validate, ValidationResult};
use serde_json::Value;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    validation::run_validator_cli(
        "catalog-validate",
        "Validates patch, artifact, target build, and validation result JSON files.",
        "catalogs",
        validate_contract,
    )
}

fn validate_contract(value: &Value) -> Result<Option<&'static str>> {
    let object = value
        .as_object()
        .context("document must be a JSON object")?;

    if object.contains_key("cves") {
        let entry: PatchEntry = serde_json::from_value(value.clone()).context("invalid patch entry")?;
        entry.validate()?;
        return Ok(Some("patch"));
    }
    if object.contains_key("file_name") {
        let artifact: Artifact = serde_json::from_value(value.clone()).context("invalid artifact entry")?;
        artifact.validate()?;
        return Ok(Some("artifact"));
    }
    if object.contains_key("product_name") {
        let build: TargetBuild = serde_json::from_value(value.clone()).context("invalid target build")?;
        build.validate()?;
        return Ok(Some("target build"));
    }
    if object.contains_key("checks") {
        let result: ValidationResult =
            serde_json::from_value(value.clone()).context("invalid validation result")?;
        result.validate()?;
        return Ok(Some("validation result"));
    }
    bail!("cannot determine contract type; expected patch, artifact, target build, or validation result")
}
