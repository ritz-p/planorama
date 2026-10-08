mod support;

#[test]
fn security_groups_fixture_preserves_policy_cards_and_inspectable_connections() {
    let input = include_bytes!("../fixtures/security-groups-plan.json");
    let svg = render(input);
    assert!(svg.contains("7 resources (7 cards), 10 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 4);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 6);
    for (group, target) in [
        ("web", "aws_instance.app"),
        ("shared", "aws_instance.app"),
        ("web", "aws_lb.app"),
        ("shared", "aws_ecs_service.app"),
    ] {
        assert!(svg.contains(&format!(
            "aws_security_group.{group} → {target} (connection)"
        )));
        assert!(svg.contains(&format!(
            "data-terraform-address=\"aws_security_group.{group}\""
        )));
    }
    assert_eq!(svg, render(input));
}

#[test]
fn drift_fixture_keeps_apply_actions_and_reports_drift_without_values() {
    let input = include_bytes!("../fixtures/drift-plan.json");
    let svg = render(input);
    assert!(svg.contains("data-drift=\"delete\""));
    assert!(svg.contains("data-drift=\"update\""));
    assert!(svg.contains("create (1)"));
    assert!(svg.contains("delete (0)"));
    assert!(!svg.contains("TOP_SECRET"));
    let output = support::run_with_args(input, &["--diagnostics"]);
    assert!(output.status.success());
    assert_eq!(svg.as_bytes(), output.stdout);
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        diagnostics
            .matches("Terraform-reported resource drift")
            .count(),
        2
    );
    assert!(!diagnostics.contains("TOP_SECRET"));
    assert_eq!(svg, render(input));
}

#[test]
fn lifecycle_fixture_distinguishes_import_forget_and_destroy_without_values() {
    let input = include_bytes!("../fixtures/lifecycle-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("data-import=\"true\"").count(), 2);
    assert!(svg.contains("data-state-removal=\"forget\""));
    assert!(svg.contains("data-state-removal=\"create then forget\""));
    assert!(svg.contains("import / update"));
    assert!(svg.contains("delete (1)"));
    assert!(!svg.contains("TOP_SECRET"));
    let diagnostic = support::run_with_args(input, &["--diagnostics"]);
    assert!(diagnostic.status.success());
    assert!(!String::from_utf8_lossy(&diagnostic.stderr).contains("TOP_SECRET"));
    assert_eq!(svg.as_bytes(), diagnostic.stdout);
    assert_eq!(svg, render(input));
}

#[test]
fn moved_metadata_survives_cards_and_lowered_relationships_and_is_escaped() {
    let mut input: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/association-plan.json")).unwrap();
    input["resource_changes"][0]["previous_address"] =
        serde_json::json!("aws_subnet.old[\"<a>&\"]");
    input["resource_changes"][2]["previous_address"] =
        serde_json::json!("module.old.aws_route_table_association.private");
    let svg = render(input.to_string().as_bytes());
    assert!(svg.contains("data-previous-address=\"aws_subnet.old[&quot;&lt;a&gt;&amp;&quot;]\""));
    assert!(
        svg.contains("data-previous-address=\"module.old.aws_route_table_association.private\"")
    );
    assert_eq!(svg.matches("data-previous-address=").count(), 2);
    assert!(svg.contains("data-terraform-address=\"aws_subnet.private\""));
    assert!(!svg.contains("<a>"));
}

#[test]
fn deposed_fixture_preserves_every_change_card_and_action() {
    let input = include_bytes!("../fixtures/deposed-plan.json");
    let svg = render(input);
    assert_eq!(svg.matches("<g id=\"resource-").count(), 4);
    assert_eq!(svg.matches("data-deposed-key=").count(), 2);
    assert!(svg.contains("delete (2)"));
    assert!(svg.contains("create (1)"));
    assert_eq!(svg, render(input));
}

#[test]
fn committed_svg_samples_match_the_current_renderer() {
    for (input, expected) in [
        (
            include_bytes!("../fixtures/spanning-dense-plan.json").as_slice(),
            include_str!("../../examples/spanning-dense.svg"),
        ),
        (
            include_bytes!("../fixtures/dense-architecture-plan.json").as_slice(),
            include_str!("../../examples/dense-architecture.svg"),
        ),
        (
            include_bytes!("../../examples/plan.json").as_slice(),
            include_str!("../../examples/diagram.svg"),
        ),
        (
            include_bytes!("../../examples/bundling-plan.json").as_slice(),
            include_str!("../../examples/bundling.svg"),
        ),
        (
            include_bytes!("../fixtures/terraform-plan.json").as_slice(),
            include_str!("../../examples/terraform.svg"),
        ),
        (
            include_bytes!("../fixtures/association-plan.json").as_slice(),
            include_str!("../../examples/association.svg"),
        ),
        (
            include_bytes!("../fixtures/containment-plan.json").as_slice(),
            include_str!("../../examples/containment.svg"),
        ),
        (
            include_bytes!("../../examples/data-plan.json").as_slice(),
            include_str!("../../examples/data.svg"),
        ),
    ] {
        assert_eq!(render(input), expected.replace("\r\n", "\n"));
    }
}

#[test]
fn cli_emits_containment_metadata_without_dropping_resources_or_dependencies() {
    let svg = render(include_bytes!("../fixtures/containment-plan.json"));
    assert!(svg.contains("3 resources (3 cards), 3 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 2);
    assert_eq!(svg.matches("marker-end=").count(), 0);
    assert_eq!(svg.matches("data-container=").count(), 2);
}

#[test]
fn association_resources_are_lowered_by_the_cli() {
    let svg = render(include_bytes!("../fixtures/association-plan.json"));
    assert!(svg.contains("3 resources (2 cards), 1 relationships"));
    assert!(svg.contains("aws_subnet.private → aws_route_table.private"));
    assert!(!svg.contains("<title>aws_route_table_association.private</title>"));
    assert!(svg.contains("association; create: aws_route_table_association.private"));
    assert!(svg.contains("create (3)"));
    assert!(!svg.contains("reference edges"));
    assert!(!svg.contains("dependency → dependent"));
    assert!(!svg.contains("Arrows point from dependencies"));
    assert!(svg.contains("data-edge-kind=\"association\""));
}

fn render(input: &[u8]) -> String {
    let output = support::run(input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn output_is_deterministic_and_empty_plans_render() {
    let input = include_bytes!("../../examples/plan.json");
    assert_eq!(render(input), render(input));
    assert!(render(br#"{"format_version":"1.0","resource_changes":[]}"#).contains("No resources"));
}

#[test]
fn escapes_untrusted_labels_and_omits_values() {
    let input = br#"{"format_version":"1.0","resource_changes":[{"address":"test.<script>&\"","type":"test","change":{"actions":["create"],"after":{"password":"TOP_SECRET"}}}]}"#;
    let svg = render(input);
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;&amp;&quot;"));
    assert!(!svg.contains("TOP_SECRET"));
}

#[test]
fn bundled_fixture_preserves_individual_dependency_arrows() {
    let svg = render(include_bytes!("../../examples/bundling-plan.json"));
    assert!(svg.contains("5 resources, 6 reference edges"));
    assert_eq!(svg.matches("marker-end=").count(), 6);
    assert!(svg.contains("<circle "));
    for name in ["a", "b", "c"] {
        assert!(svg.contains(&format!(
            "<title>terraform_data.seed → terraform_data.{name}</title>"
        )));
        assert!(svg.contains(&format!(
            "<title>terraform_data.{name} → terraform_data.aggregate</title>"
        )));
    }
}
#[test]
fn multi_container_fixture_renders_one_card_per_resource_and_subnet_connections() {
    let output = support::run(include_bytes!("../fixtures/multi-container-plan.json"));
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert_eq!(svg.matches("<g id=\"resource-").count(), 6);
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 5);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 4);
    assert_eq!(
        svg,
        include_str!("../../examples/multi-container.svg").replace("\r\n", "\n")
    );
}

#[test]
fn explicit_network_scopes_render_as_nested_architecture() {
    let input = include_bytes!("../fixtures/network-scope-plan.json");
    let result = support::run(input);
    assert!(result.status.success());
    let svg = String::from_utf8(result.stdout).unwrap();
    assert!(svg.contains("7 resources (7 cards), 6 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 6);
    assert_eq!(svg.matches("marker-end=").count(), 0);
    assert_eq!(svg.as_bytes(), support::run(input).stdout);
}

#[test]
fn rds_subnet_group_fixture_preserves_cards_and_infers_common_scope() {
    let input = include_bytes!("../fixtures/rds-subnet-group-plan.json");
    let svg = render(input);
    assert!(svg.contains("6 resources (6 cards), 8 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 4);
    assert_eq!(svg.matches("<g id=\"resource-").count(), 6);
    assert_eq!(svg, render(input));
}

#[test]
fn mixed_aws_relationships_render_actions_without_helper_cards() {
    let input = include_bytes!("../fixtures/aws-relationships-plan.json");
    let svg = render(input);
    assert!(svg.contains("9 resources (6 cards), 7 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"association\"").count(), 3);
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 4);
    for (kind, action) in [
        ("aws_route_table_association", "create"),
        ("aws_lb_target_group_attachment", "replace"),
        ("aws_vpc_endpoint_route_table_association", "delete"),
    ] {
        assert!(!svg.contains(&format!("<title>{kind}.main</title>")));
        assert!(svg.contains(&format!("association; {action}: {kind}.main")));
        assert!(svg.contains(&format!("{action} (1)")));
    }
    assert_eq!(svg, render(input));
}
