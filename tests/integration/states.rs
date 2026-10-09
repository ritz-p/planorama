use std::collections::BTreeSet;
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_planorama"))
}

#[test]
fn hard_linked_output_cannot_truncate_single_or_named_inputs() {
    let root = std::env::temp_dir().join(format!("planorama-hardlink-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let input = root.join("input.json");
    let alias = root.join("alias.svg");
    let separate = root.join("separate.svg");
    let bytes = include_bytes!("../fixtures/multi/application.json");
    std::fs::write(&input, bytes).unwrap();
    std::fs::hard_link(&input, &alias).unwrap();
    for named in [false, true] {
        let mut command = cli();
        if named {
            command.args([
                "--state",
                &format!("a={}", fixture("multi/network.json")),
                "--state",
                &format!("z={}", input.display()),
            ]);
        } else {
            command.arg(&input);
        }
        let output = command.arg("-o").arg(&alias).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
        assert_eq!(std::fs::read(&input).unwrap(), bytes);
        assert_eq!(std::fs::read(&alias).unwrap(), bytes);
    }
    // Equal contents in a distinct file must not be mistaken for identity.
    std::fs::write(&separate, bytes).unwrap();
    let output = cli().arg(&input).arg("-o").arg(&separate).output().unwrap();
    assert!(output.status.success());
    assert!(std::fs::read(&separate).unwrap().starts_with(b"<svg"));
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    for path in [alias, separate, input] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}
fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}
fn run(reverse: bool, extra: &[&str]) -> Output {
    let mut args = vec![
        format!("network={}", fixture("multi/network.json")),
        format!("application={}", fixture("multi/application.json")),
    ];
    if reverse {
        args.reverse();
    }
    let mut cmd = cli();
    for arg in args {
        cmd.args(["--state", &arg]);
    }
    cmd.args(["-o", "-"]).args(extra).output().unwrap()
}

#[test]
fn duplicate_addresses_remain_distinct_with_stable_state_scoping() {
    let first = run(false, &["--diagnostics"]);
    let second = run(true, &["--diagnostics"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    let svg = String::from_utf8(first.stdout).unwrap();
    assert_eq!(
        svg.matches("data-terraform-address=\"aws_security_group.app\"")
            .count(),
        3
    );
    assert_eq!(svg.matches("data-deposed-key=\"old\"").count(), 1);
    let application = svg.find("data-state-id=\"application\"").unwrap();
    let network = svg.find("data-state-id=\"network\"").unwrap();
    assert!(application < network);
    assert!(svg[application..network].contains("data-plan-errored=\"true\""));
    assert!(svg[network..].contains("data-plan-errored=\"false\""));
    assert!(svg[application..network].contains("0 pass / 1 fail"));
    assert!(svg[network..].contains("1 pass / 0 fail"));
    assert!(!svg.contains("TOP_SECRET"));
    let diagnostics = String::from_utf8(first.stderr).unwrap();
    assert!(diagnostics.lines().all(|line| line.starts_with("state=")));
    assert!(diagnostics.contains("state=\"application\""));
    assert!(diagnostics.contains("state=\"network\""));
    assert!(!diagnostics.contains("TOP_SECRET"));
}

#[test]
fn nested_svg_ids_links_icons_and_markers_are_unique_and_local() {
    let path = fixture("components-plan.json");
    let output = cli()
        .args([
            "--state",
            &format!("one={path}"),
            "--state",
            &format!("two={path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let ids: Vec<_> = svg
        .split(" id=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len());
    for section in svg.split("data-state-id=\"").skip(1) {
        let state = section.split('"').next().unwrap();
        let prefix = match state {
            "one" => "state-6f6e65-",
            "two" => "state-74776f-",
            _ => unreachable!(),
        };
        for token in [" href=\"#", "marker-end=\"url(#"] {
            for reference in section.split(token).skip(1) {
                let id = reference.split(['"', ')']).next().unwrap();
                assert!(unique.contains(id), "missing {id}");
                assert!(id.starts_with(prefix), "cross-state reference {id}");
            }
        }
    }
}

#[test]
fn named_input_validation_happens_before_reading_files() {
    for (args, error) in [
        (
            vec!["--state", "same=missing", "--state", "same=also-missing"],
            "duplicate state ID",
        ),
        (vec!["--state"], "ID=PATH"),
        (vec!["--state", "name"], "ID=PATH"),
        (vec!["--state", "=missing"], "state ID"),
        (vec!["--state", "name="], "nonempty input path"),
        (vec!["--state", "name=missing", "positional"], "do not mix"),
        (
            vec!["--state", "one=-", "--state", "two=-"],
            "only one state",
        ),
    ] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(error));
    }
}

#[test]
fn named_stdin_is_supported_and_errors_keep_state_context() {
    let mut child = cli()
        .args([
            "--state",
            "stdin=-",
            "--state",
            &format!("file={}", fixture("multi/network.json")),
            "-o",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("../fixtures/multi/application.json"))
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("data-state-id=\"stdin\""));
    let output = cli()
        .args(["--state", "missing=nonexistent-plan-125.json", "-o", "-"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("state \"missing\""));
}

#[test]
fn refuses_to_overwrite_any_named_input() {
    let first = fixture("multi/network.json");
    let second = fixture("multi/application.json");
    let before = std::fs::read(&second).unwrap();
    let output = cli()
        .args([
            "--state",
            &format!("one={first}"),
            "--state",
            &format!("two={second}"),
            "-o",
            &second,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("different files"));
    assert_eq!(std::fs::read(second).unwrap(), before);
}

#[test]
fn invalid_named_plan_produces_no_partial_output() {
    let output_path = std::env::temp_dir().join(format!(
        "planorama-multi-invalid-{}.svg",
        std::process::id()
    ));
    std::fs::write(&output_path, "existing output").unwrap();
    let invalid = format!("{}/docs/multi-plan.md", env!("CARGO_MANIFEST_DIR"));
    let output = cli()
        .args([
            "--state",
            &format!("a={}", fixture("multi/network.json")),
            "--state",
            &format!("z={invalid}"),
            "--diagnostics",
            "-o",
        ])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("state \"z\": invalid JSON"));
    assert!(!stderr.contains("warning:"));
    assert_eq!(
        std::fs::read_to_string(&output_path).unwrap(),
        "existing output"
    );
    std::fs::remove_file(output_path).unwrap();
}

#[test]
fn multi_plan_sample_matches_renderer() {
    let output = run(false, &[]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("../../examples/multi-plan.svg").replace("\r\n", "\n")
    );
}
