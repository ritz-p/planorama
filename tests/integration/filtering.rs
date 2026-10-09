use super::{cli, support};
use serde_json::json;

#[test]
fn focus_preserves_ancestors_without_unrelated_nodes_and_is_deterministic() {
    let mut plan: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/containment-plan.json")).unwrap();
    plan["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_instance.unrelated","type":"aws_instance","change":{"actions":["no-op"]}}));
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
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .matches("<g id=\"resource-")
            .count(),
        2
    );
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
    for args in [
        vec!["--focus", "missing.resource"],
        vec!["--focus-depth", "2"],
        vec!["--focus", "aws_vpc.main", "--focus-depth", "-1"],
    ] {
        let output =
            support::run_with_args(include_bytes!("../fixtures/containment-plan.json"), &args);
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
