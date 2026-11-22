use super::cli;
use serde_json::{Value, json};
use std::fs;

fn report(before: &Value, after: &Value, flags: &[&str]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.json"), dir.path().join("b.json"));
    fs::write(&a, before.to_string()).unwrap();
    fs::write(&b, after.to_string()).unwrap();
    let file = dir.path().join("changes.md");
    let result = cli()
        .arg("diff")
        .arg(a)
        .arg(b)
        .args(["--format", "markdown", "-o"])
        .arg(&file)
        .args(flags)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    fs::read_to_string(file).unwrap()
}

#[test]
fn markdown_reports_listener_and_subnet_changes_and_ignores_order_and_secrets() {
    let mut before: Value =
        serde_json::from_slice(include_bytes!("../fixtures/components-plan.json")).unwrap();
    before["resource_changes"].as_array_mut().unwrap().extend([
        json!({"address":"aws_lb_target_group.green","type":"aws_lb_target_group"}),
        json!({"address":"aws_subnet.blue","type":"aws_subnet"}),
        json!({"address":"aws_subnet.green","type":"aws_subnet"}),
        json!({"address":"aws_instance.app","type":"aws_instance"}),
    ]);
    before["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(
        json!({"address":"aws_instance.app","expressions":{"subnet_id":{"references":["aws_subnet.blue.id"]}}}));
    let mut after = before.clone();
    let resources = &mut after["configuration"]["root_module"]["resources"];
    resources[0]["expressions"]["default_action"][0]["target_group_arn"]["references"] =
        json!(["aws_lb_target_group.green.arn"]);
    resources[3]["expressions"]["subnet_id"]["references"] = json!(["aws_subnet.green.id"]);
    let md = report(&before, &after, &[]);
    assert!(md.contains("## Relationship endpoints changed"), "{md}");
    assert!(
        md.lines().any(|l| l.contains("aws_lb_listener.https")
            && l.contains("changed from")
            && l.contains("aws_lb_target_group.shared")
            && l.contains("aws_lb_target_group.green")),
        "{md}"
    );
    assert!(md.contains("## Containment changed"));
    assert!(md.lines().any(|l| l.contains("aws_instance.app")
        && l.contains("moved from")
        && l.contains("aws_subnet.blue")
        && l.contains("aws_subnet.green")));
    assert!(!md.contains("TOP_SECRET_PATH"));
    after["resource_changes"].as_array_mut().unwrap().reverse();
    assert_eq!(md, report(&before, &after, &[]));
    let masked = report(&before, &after, &["--anonymize"]);
    assert!(masked.contains("Resource ") && masked.contains("moved from"));
    assert!(!masked.contains("aws_") && !masked.contains("default") && !masked.contains("SECRET"));
    before["resource_changes"].as_array_mut().unwrap().reverse();
    assert_eq!(
        report(&before, &before, &[]),
        "# Architecture diff\n\nNo architecture changes.\n"
    );
}

#[test]
fn markdown_distinguishes_unknown_matches_and_escapes_identifiers() {
    let address = "aws_s3_bucket.example[\"two``ticks | <img>\"]";
    let before = json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.same","type":"aws_vpc","provider_name":"hashicorp/aws"},
        {"address":"aws_s3_bucket.old","type":"aws_s3_bucket"}]});
    let after = json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.same","type":"aws_vpc","provider_name":"other/aws"},
        {"address":address,"type":"aws_s3_bucket"}]});
    let md = report(&before, &after, &[]);
    assert!(
        md.contains("## Resources added")
            && md.contains("## Resources removed")
            && md.contains("## Unknown resource matches"),
        "{md}"
    );
    assert!(md.contains(&format!("- ``` default:{address} ```")), "{md}");
    assert!(!md.contains("No architecture changes.") && !md.contains("other/aws"));
}

#[test]
fn markdown_leaves_ambiguous_relationships_unpaired() {
    let snapshot = |refs: &[&str]| {
        let resources = [
            "aws_s3_bucket.a",
            "aws_s3_bucket.b",
            "aws_s3_bucket.c",
            "aws_instance.app",
        ]
        .map(|address| json!({"address":address,"type":address.split('.').next().unwrap()}));
        json!({"format_version":"1.2","resource_changes":resources,"configuration":{"root_module":{"resources":[
            {"address":"aws_instance.app","expressions":{"input":{"references":refs}}}]}}})
    };
    let md = report(
        &snapshot(&["aws_s3_bucket.a.id", "aws_s3_bucket.b.id"]),
        &snapshot(&["aws_s3_bucket.c.id"]),
        &[],
    );
    assert!(
        md.contains("## Relationships added") && md.contains("## Relationships removed"),
        "{md}"
    );
    assert!(!md.contains("changed from"));
}

#[test]
fn markdown_distinguishes_unavailable_snapshots_from_no_changes() {
    let result = cli()
        .args([
            "diff",
            "--before-state",
            "producer=tests/fixtures/multi/producer.json",
            "--after-state",
            "consumer=tests/fixtures/multi/consumer.json",
            "--format",
            "markdown",
            "--anonymize",
        ])
        .output()
        .unwrap();
    let md = String::from_utf8(result.stdout).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        md.contains("## Comparison unavailable")
            && md.contains("After snapshot")
            && md.contains("unmapped remote state"),
        "{md}"
    );
    assert!(!md.contains("aws_") && !md.contains("No architecture changes."));
}
