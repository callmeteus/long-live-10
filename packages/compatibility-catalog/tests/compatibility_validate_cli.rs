use std::process::{Command, Output};

fn run_validator(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_compatibility-validate"))
        .args(arguments)
        .output()
        .expect("validator starts")
}

#[test]
fn accepts_valid_fixtures() {
    let output = run_validator(&["tests/fixtures/valid"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("2 file(s), 0 failure(s)"));
}

#[test]
fn accepts_the_initial_production_entries() {
    let output = run_validator(&["entries"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("8 file(s), 0 failure(s)"));
}

#[test]
fn rejects_invalid_fixtures() {
    let output = run_validator(&["tests/fixtures/invalid"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("error "));
    assert!(stdout.contains("10 file(s), 10 failure(s)"));
}

#[test]
fn rejects_an_unknown_contract_shape() {
    let output = run_validator(&["tests/fixtures/unknown-shape.json"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("not a compatibility entry"));
}

#[test]
fn rejects_a_missing_path() {
    let output = run_validator(&["tests/fixtures/does-not-exist"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("failed to inspect"));
}

#[test]
fn prints_usage() {
    let output = run_validator(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Usage: compatibility-validate"));
}
