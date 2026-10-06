mod support;

#[test]
fn real_plan_renders_module_and_data_source_dependencies() {
    let output = support::run(include_bytes!("../fixtures/terraform-plan.json"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.contains("16 resources, 28 reference edges"));
    assert_eq!(svg.matches("marker-end=").count(), 28);
    for (source, target) in [
        ("terraform_data.independent", "terraform_data.explicit"),
        (
            "terraform_data.ready",
            "data.terraform_remote_state.after_ready",
        ),
        (
            "data.terraform_remote_state.existing",
            "terraform_data.from_state",
        ),
        (
            "data.terraform_remote_state.after_ready",
            "terraform_data.from_state",
        ),
        (
            "module.service[&quot;api&quot;].terraform_data.worker[0]",
            "module.service[&quot;api&quot;].module.nested.terraform_data.leaf",
        ),
    ] {
        assert!(svg.contains(&format!("<title>{source} → {target}</title>")));
    }
    assert!(svg.contains("create (14)"));
    assert!(svg.contains("read (1)"));
    assert!(svg.contains("unchanged (1)"));
}

#[test]
fn captured_aws_plan_renders_offline_with_real_expression_shapes() {
    let input = include_bytes!("../fixtures/aws-captured/plan.json");
    let plan: serde_json::Value = serde_json::from_slice(input).unwrap();
    assert_eq!(plan["terraform_version"], "1.9.8");
    assert_eq!(plan["resource_changes"].as_array().unwrap().len(), 7);
    assert!(plan.get("timestamp").is_none());
    let output = support::run(input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert!(svg.contains("7 resources (7 cards), 12 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 3);
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 2);
    assert!(!svg.contains("data.aws_iam_policy_document.fixture"));
    for index in 0..2 {
        assert!(svg.contains(&format!("module.network.aws_subnet.private[{index}]")));
        assert!(svg.contains(&format!(
            "module.network.aws_route_table_association.private[{index}]"
        )));
    }
    assert_eq!(svg.as_bytes(), support::run(input).stdout);
}
