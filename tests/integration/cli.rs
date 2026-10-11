mod filtering;
mod states;
mod support;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_planorama"))
}

#[test]
fn help_documents_remote_state_mapping_syntax() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("--remote-state CONSUMER:ADDRESS=PRODUCER")
    );
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

#[test]
fn optional_diagnostics_distinguish_failures_without_changing_svg_or_leaking_values() {
    use serde_json::json;
    let cases = [
        (
            json!({"references":["aws_subnet.missing.id"]}),
            json!({}),
            "unresolved reference",
        ),
        (
            json!({"references":["aws_subnet.private.id"]}),
            json!({}),
            "multiple matching instances",
        ),
        (
            json!({"references":["aws_subnet.private[count.index].id"]}),
            json!({}),
            "dynamic instance selection",
        ),
        (
            json!({"references":["local.a"]}),
            json!({"a":{"references":["local.b"]},"b":{"references":["local.a"]}}),
            "alias/local/module resolution cycle",
        ),
        (
            json!({"references":["aws_vpc.main.id"]}),
            json!({}),
            "semantic endpoint type mismatch",
        ),
        (
            json!({"constant_value":"TOP_SECRET"}),
            json!({}),
            "attribute has no resolvable resource reference",
        ),
    ];
    for (expression, locals, reason) in cases {
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"aws_vpc.main","type":"aws_vpc"},
            {"address":"aws_subnet.private[0]","type":"aws_subnet"},
            {"address":"aws_subnet.private[1]","type":"aws_subnet"},
            {"address":"aws_instance.app","type":"aws_instance","change":{"after":{"password":"TOP_SECRET"}}}
        ],"configuration":{"root_module":{"locals":locals,"resources":[
            {"address":"aws_subnet.private","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}},
            {"address":"aws_instance.app","expressions":{"subnet_id":expression,"password":{"constant_value":"TOP_SECRET"}}}
        ]}}}).to_string();
        let normal = support::run(input.as_bytes());
        let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
        assert!(output.status.success());
        assert_eq!(output.stdout, normal.stdout);
        assert!(normal.stderr.is_empty());
        let diagnostics = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostics.contains(reason), "{diagnostics}");
        assert!(diagnostics.contains("address=\"aws_instance.app\""));
        assert!(diagnostics.contains("attribute=\"subnet_id\""));
        assert!(!diagnostics.contains("TOP_SECRET"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("TOP_SECRET"));
        assert_eq!(
            diagnostics.as_bytes(),
            support::run_with_args(input.as_bytes(), &["--diagnostics"]).stderr
        );
    }
}

#[test]
fn plan_level_root_variables_resolve_through_child_inputs_without_values() {
    use serde_json::json;
    for declared in [false, true] {
        let input = json!({"format_version":"1.2",
            "variables": if declared { json!({"region":{"value":"TOP_SECRET"}}) } else { json!({}) },
            "resource_changes":[
                {"address":"test.root","type":"test"},
                {"address":"module.child.test.inner","type":"test"},
                {"address":"module.child.aws_instance.inner","type":"aws_instance"}
            ],"configuration":{"root_module":{
                "resources":[{"address":"test.root","expressions":{"region":{"references":["var.region"]}}}],
                "module_calls":{"child":{"expressions":{"input":{"references":["var.region"]}},"module":{
                    "variables":{"input":{}},
                    "resources":[
                        {"address":"test.inner","expressions":{"region":{"references":["var.input"]}}},
                        {"address":"aws_instance.inner","expressions":{"subnet_id":{"references":["var.input"]}}}
                    ]
                }}}
            }}}).to_string();
        let output = support::run_with_args(input.as_bytes(), &["--diagnostics"]);
        assert!(output.status.success());
        let warnings = String::from_utf8(output.stderr).unwrap();
        if declared {
            assert!(!warnings.contains("unresolved reference"), "{warnings}");
            assert!(
                warnings.contains("attribute has no resolvable resource reference"),
                "{warnings}"
            );
            assert_eq!(warnings.lines().count(), 1);
        } else {
            assert!(warnings.contains("unresolved reference"), "{warnings}");
        }
        assert!(!warnings.contains("TOP_SECRET"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("TOP_SECRET"));
        assert_eq!(output.stdout, support::run(input.as_bytes()).stdout);
    }
}
