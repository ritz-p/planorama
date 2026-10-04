#[path = "../../examples/terraform-large/generator.rs"]
mod generator;
mod support;

#[test]
fn large_fixture_is_reproducible_from_terraform_sources_and_renders_deterministically() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/terraform-large");
    let generated = generator::generate(&root).unwrap();
    let committed = include_str!("../../examples/terraform-large/plan.json").replace("\r\n", "\n");
    assert_eq!(generated, committed);
    let output = support::run(generated.as_bytes());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let svg = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        svg,
        include_str!("../../examples/terraform-large/diagram.svg").replace("\r\n", "\n")
    );
    assert_eq!(svg.matches("data-edge-kind=\"association\"").count(), 4);
    assert_eq!(svg.matches("data-container=").count(), 6);
    assert_eq!(svg.matches("data-external=\"true\"").count(), 3);
    assert!(svg.contains("module.application.aws_instance.workers_a[0]"));
    assert!(svg.contains("data-role=\"policy\""));
    assert!(svg.contains("data-role=\"controller\""));
    assert!(!svg.contains("data.aws_region.current"));
    assert!(!svg.contains("stroke-dasharray=\"5 4\""));
    assert_eq!(output_svg(generated.as_bytes()), svg);
}

fn output_svg(input: &[u8]) -> String {
    let output = support::run(input);
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}
