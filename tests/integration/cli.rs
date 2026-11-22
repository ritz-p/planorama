mod anonymization;
mod athena;
mod cloudflare;
mod cloudflare_recovery;
mod configuration;
mod coverage_json;
mod coverage_report;
mod diff_diagrams;
mod filtering;
mod firehose;
mod glue;
mod html;
mod layout_diagnostics;
mod messaging;
mod modules;
mod provider_plan;
mod relationship_legend;
mod service_names;
mod states;
mod support;
mod timings;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_planorama"))
}

#[test]
fn help_documents_remote_state_mapping_syntax() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("planorama —"));
    assert!(text.contains("--remote-state CONSUMER:ADDRESS=PRODUCER"));
    let options = text.split("\n\n").nth(2).unwrap();
    assert!(options.contains("--layout-diagnostics  "));
    assert!(options.contains("--layout-diagnostics-json PATH"));
    let modes = "service-name (default; icon unavailable in this build)";
    assert!(options.contains(modes));
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

#[test]
fn architecture_json_exports_selected_topology_deterministically() {
    let input = include_bytes!("../../examples/plan.json");
    let export = || support::run_with_args(input, &["--format", "json"]);
    let first = export();
    assert!(first.status.success());
    assert_eq!(first.stdout, export().stdout);
    let doc: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(doc["schema_version"], 1);
    let resources = doc["resources"].as_array().unwrap();
    assert!(
        resources
            .iter()
            .any(|r| r["address"] == "aws_vpc.main" && r["mode"] == "managed")
    );
    let edges = doc["relationships"].as_array().unwrap();
    assert!(
        edges
            .iter()
            .any(|e| e["kind"] == "containment" && e["direction"] == "directed")
    );
    for edge in edges {
        for end in ["from", "to"] {
            assert!(resources.iter().any(|r| r["identity"] == edge[end]));
        }
    }
}

#[test]
fn containment_matches_reference_proof_cases() {
    use serde_json::json;
    for line in include_str!("../experiments/architecture/reference-cases.txt").lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let count: usize = fields[0].parse().unwrap();
        let provider = if fields[1] == "0" {
            "hashicorp/aws"
        } else {
            "custom/aws"
        };
        let mut changes = vec![
            json!({"address":"aws_subnet.child","type":"aws_subnet","provider_name":"hashicorp/aws"}),
        ];
        let mut refs = Vec::new();
        for index in 0..count {
            let address = format!("aws_vpc.parent{index}");
            changes.push(json!({"address":address,"type":"aws_vpc","provider_name":provider}));
            refs.push(format!("{address}.id"));
        }
        let plan = json!({"format_version":"1.2","resource_changes":changes,"configuration":{"root_module":{"resources":[{"address":"aws_subnet.child","expressions":{"vpc_id":{"references":refs}}}]}}});
        let result = support::run_with_args(plan.to_string().as_bytes(), &["--format", "json"]);
        assert!(result.status.success());
        let doc: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            doc["relationships"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["kind"] == "containment"),
            fields[2] == "accept",
            "{line}"
        );
    }
}

#[test]
fn semantic_diff_preserves_explicit_moves_and_compares_containment() {
    use serde_json::{Value, json};
    let dir = tempfile::tempdir().unwrap();
    let before = dir.path().join("before.json");
    let after = dir.path().join("after.json");
    let snapshot = |name: &str, moved: bool, contained: bool| {
        let mut vpc = json!({"address":name,"type":"aws_vpc"});
        if moved {
            vpc["previous_address"] = json!("aws_vpc.sentinel");
        }
        let references = if contained {
            vec![format!("{name}.id")]
        } else {
            Vec::new()
        };
        json!({"format_version":"1.2","resource_changes":[vpc,{"address":"aws_subnet.child","type":"aws_subnet"}],
        "configuration":{"root_module":{"resources":[{"address":"aws_subnet.child","expressions":{"vpc_id":{"references":references}}}]}}}).to_string()
    };
    std::fs::write(&before, snapshot("aws_vpc.sentinel", false, true)).unwrap();
    let run = |options: &[&str]| {
        let output = cli()
            .arg("diff")
            .arg(&before)
            .arg(&after)
            .args(options)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    std::fs::write(&after, snapshot("aws_vpc.renamed", true, true)).unwrap();
    let diff = run(&[]);
    assert_eq!(
        diff["resources"]["changed"][0]["before"]["address"],
        "aws_vpc.sentinel"
    );
    assert_eq!(diff["resources"]["added"], json!([]));
    assert_eq!(diff["relationships"], json!({"added":[],"removed":[]}));
    std::fs::write(&after, snapshot("aws_vpc.renamed", true, false)).unwrap();
    let masked = run(&["--anonymize"]);
    assert!(!masked.to_string().contains("sentinel"));
    assert!(
        masked["relationships"]["removed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "containment")
    );
    std::fs::write(&after, r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap();
    assert_eq!(
        run(&[])["resources"]["removed"].as_array().unwrap().len(),
        2
    );
    let output = cli()
        .arg("diff")
        .arg(&before)
        .arg(&after)
        .arg("-o")
        .arg(&before)
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn named_diff_keeps_provider_conflicts_unknown_and_states_distinct() {
    use serde_json::{Value, json};
    let dir = tempfile::tempdir().unwrap();
    let mut inputs = Vec::new();
    for (file, source) in [("aws", "hashicorp/aws"), ("other", "private/aws")] {
        let path = dir.path().join(file);
        std::fs::write(&path,json!({"format_version":"1.2","resource_changes":[{"address":"aws_vpc.same","type":"aws_vpc","provider_name":source}]}).to_string()).unwrap();
        inputs.push(path.display().to_string());
    }
    let output = cli()
        .arg("diff")
        .args([
            "--before-state",
            &format!("a={}", inputs[0]),
            "--after-state",
            &format!("b={}", inputs[0]),
            "--after-state",
            &format!("a={}", inputs[1]),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["resources"]["unknown"][0]["state"], "a");
    assert_eq!(doc["resources"]["added"][0]["state"], "b");
    assert_eq!(doc["resources"]["removed"], json!([]));
}

#[test]
fn semantic_diff_preserves_a_renamed_organizations_root() {
    use serde_json::{Value, json};
    let dir = tempfile::tempdir().unwrap();
    let before = dir.path().join("before.json");
    let after = dir.path().join("after.json");
    let old = "aws_organizations_organization.before";
    let new = "aws_organizations_organization.after";
    let mut org = json!({"address":old,"mode":"managed","type":"aws_organizations_organization",
        "provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["no-op"],
        "after":{"roots":[{"id":"r-example"}]}}});
    let snapshot =
        |org: &Value| json!({"format_version":"1.2","resource_changes":[org]}).to_string();
    std::fs::write(&before, snapshot(&org)).unwrap();
    org["address"] = json!(new);
    org["previous_address"] = json!(old);
    std::fs::write(&after, snapshot(&org)).unwrap();
    let export = cli()
        .arg(&after)
        .args(["--format", "json", "-o", "-"])
        .output()
        .unwrap();
    let graph: Value = serde_json::from_slice(&export.stdout).unwrap();
    assert!(
        graph["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "component")
    );
    let output = cli().arg("diff").arg(before).arg(after).output().unwrap();
    let diff: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in ["added", "removed", "unknown"] {
        assert_eq!(diff["resources"][field], json!([]));
    }
    assert_eq!(diff["resources"]["changed"].as_array().unwrap().len(), 1);
    assert_eq!(diff["resources"]["changed"][0]["after"]["address"], new);
    assert_eq!(diff["relationships"], json!({"added":[], "removed":[]}));
}

#[test]
fn named_diff_rejects_incomplete_remote_state_evidence() {
    use serde_json::{Value, json};
    let producer = "tests/fixtures/multi/producer.json";
    let consumer = "consumer=tests/fixtures/multi/consumer.json".to_owned();
    let mut input: Value =
        serde_json::from_str(&std::fs::read_to_string(producer).unwrap()).unwrap();
    input["configuration"]["root_module"]["outputs"]["subnet"]["expression"] =
        json!({"constant_value":"PRIVATE_SENTINEL"});
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("incomplete.json");
    std::fs::write(&missing, input.to_string()).unwrap();
    let after = format!("producer={}", missing.display());
    for before in [format!("producer={producer}"), after.clone()] {
        let output = cli()
            .arg("diff")
            .args([
                "--before-state",
                &before,
                "--before-state",
                &consumer,
                "--after-state",
                &after,
                "--after-state",
                &consumer,
                "--anonymize",
                "--remote-state",
                "consumer:data.terraform_remote_state.network=producer",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains("producer output provenance is unresolved"),
            "{error}"
        );
        assert!(!error.contains("PRIVATE_SENTINEL"));
    }
}
