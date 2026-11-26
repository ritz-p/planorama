use super::{cli, support};
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn modules_cli_union_is_deterministic_in_svg_and_html() {
    let input = json!({"format_version":"1.2","resource_changes":[
        {"address":"module.app[\"a,b:c.d\"].terraform_data.x","type":"terraform_data","change":{"actions":["create"]}},
        {"address":"module.db.terraform_data.x","type":"terraform_data","change":{"actions":["no-op"]}},
        {"address":"module.application.terraform_data.outside","type":"terraform_data","change":{"actions":["create"]}}
    ]}).to_string();
    for format in ["svg", "html"] {
        let first = support::run_with_args(
            input.as_bytes(),
            &[
                "--format",
                format,
                "--modules",
                r#"module.app["a,b:c.d"]"#,
                "module.db",
            ],
        );
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        let second = support::run_with_args(
            input.as_bytes(),
            &[
                "--format",
                format,
                "--modules",
                "module.db",
                "--modules",
                "module.app",
                "module.app",
            ],
        );
        assert!(second.status.success());
        if format == "html" {
            let first_html = String::from_utf8(first.stdout.clone()).unwrap();
            let second_html = String::from_utf8(second.stdout.clone()).unwrap();
            let diagram = |html: &str| {
                html[html.find("<svg ").unwrap()..html.rfind("</svg>").unwrap()].to_owned()
            };
            assert_eq!(diagram(&first_html), diagram(&second_html));
            assert_ne!(first_html, second_html);
            let reordered = support::run_with_args(
                input.as_bytes(),
                &["--format", "html", "--modules", "module.app", "module.db"],
            );
            assert!(reordered.status.success());
            assert_eq!(second.stdout, reordered.stdout);
        } else {
            assert_eq!(first.stdout, second.stdout);
        }
        assert!(
            !String::from_utf8(first.stdout)
                .unwrap()
                .contains("terraform_data.outside")
        );
    }
    for args in [
        vec!["--modules"],
        vec!["--modules", "module.missing"],
        vec!["--modules", "module.app.terraform_data.x"],
    ] {
        let result = support::run_with_args(input.as_bytes(), &args);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn focus_preserves_ancestors_without_unrelated_nodes_and_is_deterministic() {
    let mut plan: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/containment-plan.json")).unwrap();
    plan["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_instance.unrelated","type":"aws_instance","change":{"actions":["no-op"]}}));
    let chain = ["terraform_data.near", "terraform_data.far"];
    for (address, reference) in [
        (chain[0], "aws_instance.app.id"),
        (chain[1], "terraform_data.near.id"),
    ] {
        plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "address": address, "type": "terraform_data", "change": {"actions": ["no-op"]}
            }));
        plan["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "address": address, "expressions": {"input": {"references": [reference]}}
            }));
    }
    let input = plan.to_string();
    let args = ["--focus", "aws_instance.app", "--focus-depth", "0"];
    let output = support::run_with_args(input.as_bytes(), &args);
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    for address in ["aws_vpc.main", "aws_subnet.private", "aws_instance.app"] {
        assert!(svg.contains(&format!("data-terraform-address=\"{address}\"")));
    }
    assert!(!svg.contains("aws_instance.unrelated"));
    assert_eq!(
        svg.as_bytes(),
        support::run_with_args(input.as_bytes(), &args).stdout
    );
    let context = ["aws_instance.app", "aws_subnet.private", "aws_vpc.main"];
    for depth in 0..=2 {
        let result = support::run_with_args(
            input.as_bytes(),
            &[
                "--focus",
                "aws_instance.app",
                "--focus-depth",
                &depth.to_string(),
                "--format",
                "json",
            ],
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let doc: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let addresses: BTreeSet<_> = doc["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["address"].as_str().unwrap())
            .collect();
        let expected: BTreeSet<_> = context.iter().chain(&chain[..depth]).copied().collect();
        assert_eq!(addresses, expected, "focus depth {depth}");
    }
    let full = support::run(input.as_bytes());
    assert!(
        String::from_utf8(full.stdout)
            .unwrap()
            .contains("aws_instance.unrelated")
    );
}

#[test]
fn changed_only_keeps_lowered_changes_and_focus_depth_is_effective() {
    let mut plan: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/association-plan.json")).unwrap();
    for change in plan["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .take(2)
    {
        change["change"]["actions"] = json!(["no-op"]);
    }
    let input = plan.to_string();
    let output = support::run_with_args(input.as_bytes(), &["--changed-only"]);
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert_eq!(svg.matches("<g id=\"resource-").count(), 2);
    for address in ["aws_subnet.private", "aws_route_table.private"] {
        assert!(
            svg.contains(&format!("data-terraform-address=\"{address}\"")),
            "{address}"
        );
    }
    for (depth, count) in [("0", 1), ("1", 2)] {
        let output = support::run_with_args(
            input.as_bytes(),
            &["--focus", "aws_subnet.private", "--focus-depth", depth],
        );
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .matches("<g id=\"resource-")
                .count(),
            count
        );
    }
    let output = support::run_with_args(
        input.as_bytes(),
        &[
            "--focus",
            "aws_route_table_association.private",
            "--focus-depth",
            "0",
        ],
    );
    assert!(output.status.success());
}

#[test]
fn invalid_focus_and_depth_fail_without_output() {
    let mut input = include_bytes!("../fixtures/containment-plan.json").to_vec();
    input.resize(input.len() + 256 * 1024, b' ');
    for args in [
        vec!["--focus", "missing.resource"],
        vec!["--focus-depth", "2"],
        vec!["--focus", "aws_vpc.main", "--focus-depth", "-1"],
    ] {
        let output = support::run_with_args(&input, &args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains("focus"));
    }
}

#[test]
fn multi_state_focus_disambiguates_and_keeps_cross_state_context() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/containment-plan.json"
    );
    for (focus, success) in [("aws_instance.app", false), ("one:aws_instance.app", true)] {
        let output = cli()
            .args([
                "--state",
                &format!("one={path}"),
                "--state",
                &format!("two={path}"),
                "--focus",
                focus,
                "--focus-depth",
                "0",
                "-o",
                "-",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success);
        if !success {
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("ambiguous")
            );
        }
    }
    let output = cli()
        .args([
            "--state",
            "producer=tests/fixtures/multi/producer.json",
            "--state",
            "consumer=tests/fixtures/multi/consumer.json",
            "--remote-state",
            "consumer:data.terraform_remote_state.network=producer",
            "--focus",
            "consumer:aws_instance.app",
            "--focus-depth",
            "0",
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert_eq!(svg.matches("data-cross-state=\"true\"").count(), 3);
    assert!(svg.contains("data-terraform-address=\"aws_subnet.app\""));
}
