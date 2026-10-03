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
    assert!(svg.contains("16 resources, 34 reference edges"));
    assert_eq!(svg.matches("marker-end=").count(), 34);
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
