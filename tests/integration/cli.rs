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

#[test]
fn address_format_validation_rejects_missing_and_unknown_values() {
    for args in [vec!["--address-format"], vec!["--address-format", "local"]] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("qualified or terraform"));
    }
}

#[test]
fn selected_addresses_keep_modules_indices_and_full_titles() {
    use std::io::Write;
    use std::process::Stdio;
    let addresses = [
        "aws_vpc.main",
        "module.app.aws_instance.web",
        "module.net.module.app[0].aws_instance.web[1]",
        "module.net.aws_subnet.private[\"a&b\"]",
    ];
    let changes: Vec<_> = addresses
        .iter()
        .map(|address| serde_json::json!({"address":address,"type":"aws_instance"}))
        .collect();
    let input = serde_json::json!({"format_version":"1.2","resource_changes":changes}).to_string();
    let run = |format: Option<&str>| {
        let mut command = cli();
        command.args(["-", "-o", "-"]);
        if let Some(format) = format {
            command.args(["--address-format", format]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let default = run(None);
    assert_eq!(default, run(Some("qualified")));
    for (svg, root) in [
        (default, "module.root.aws_vpc.main"),
        (run(Some("terraform")), "aws_vpc.main"),
    ] {
        assert!(svg.contains(&format!(">{root}</text>")));
        assert!(svg.contains(">module.app.aws_instance.web</text>"));
        assert!(svg.contains("<title>module.net.module.app[0].aws_instance.web[1]"));
        assert!(svg.contains("<title>module.net.aws_subnet.private[&quot;a&amp;b&quot;]"));
        assert!(svg.contains("data-terraform-address=\"aws_vpc.main\""));
    }
}
