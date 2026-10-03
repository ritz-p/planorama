mod support;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_planorama"))
}

#[test]
fn stdin_to_stdout_is_svg_and_diagnostics_stay_on_stderr() {
    let output = support::run(include_bytes!("../../examples/plan.json"));
    assert!(output.status.success());
    assert!(output.stdout.starts_with(b"<svg "));
    assert!(output.stdout.ends_with(b"</svg>\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_json_fails_without_svg() {
    let output = support::run(b"broken");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON"));
}

#[test]
fn refuses_to_overwrite_input() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/plan.json");
    let output = cli().args([path, "-o", path]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
    assert_eq!(
        std::fs::read(path).unwrap(),
        include_bytes!("../../examples/plan.json")
    );
}

#[test]
fn help_succeeds_and_unknown_flags_fail() {
    assert!(cli().arg("--help").output().unwrap().status.success());
    assert!(!cli().arg("--unknown").output().unwrap().status.success());
}
