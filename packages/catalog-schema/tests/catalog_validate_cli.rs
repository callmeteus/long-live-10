use std::process::{Command, Output};

fn run_validator(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_catalog-validate"))
        .args(arguments)
        .output()
        .expect("validator starts")
}

#[test]
fn accepts_valid_fixtures() {
    let output = run_validator(&["tests/fixtures/valid"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("4 file(s), 0 failure(s)"));
    assert!(stdout.contains("(patch)"));
    assert!(stdout.contains("(artifact)"));
    assert!(stdout.contains("(target build)"));
    assert!(stdout.contains("(validation result)"));
}

#[test]
fn rejects_invalid_fixtures() {
    let output = run_validator(&["tests/fixtures/invalid"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error "));
    assert!(stdout.contains("12 file(s), 12 failure(s)"));
}

#[test]
fn rejects_an_unknown_contract_shape() {
    let output = run_validator(&["tests/fixtures/unknown-shape.json"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("cannot determine contract type"));
}

#[test]
fn rejects_a_missing_path() {
    let output = run_validator(&["tests/fixtures/does-not-exist"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("failed to inspect"));
}
