mod support;

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
    assert_eq!(svg.matches("data-edge-kind=\"connection\"").count(), 4);
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
