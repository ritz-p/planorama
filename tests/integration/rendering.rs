mod support;

#[test]
fn cli_emits_containment_metadata_without_dropping_resources_or_dependencies() {
    let svg = render(include_bytes!("../fixtures/containment-plan.json"));
    assert!(svg.contains("3 resources (3 cards), 3 relationships"));
    assert_eq!(svg.matches("data-edge-kind=\"containment\"").count(), 2);
    assert_eq!(svg.matches("marker-end=").count(), 3);
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
