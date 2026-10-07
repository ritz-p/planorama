#[path = "../../examples/terraform-large/generator.rs"]
mod generator;
mod support;

#[test]
fn module_meta_arguments_are_rejected_before_emitting_unkeyed_instances() {
    use serde_json::json;
    let root =
        std::env::temp_dir().join(format!("planorama-module-arguments-{}", std::process::id()));
    std::fs::create_dir_all(root.join("child")).unwrap();
    std::fs::create_dir_all(root.join("wrapper/child")).unwrap();
    let child = json!({"resource":{"aws_vpc":{"main":{"cidr_block":"10.0.0.0/16"}}}});
    for path in ["child/main.tf.json", "wrapper/child/main.tf.json"] {
        std::fs::write(root.join(path), child.to_string()).unwrap();
    }
    for nested in [false, true] {
        for (argument, value) in [
            ("count", json!(0)),
            ("count", json!(1)),
            ("count", json!(2)),
            ("count", json!("${var.instances}")),
            ("for_each", json!({"a":{}})),
            ("for_each", json!("${var.instances}")),
            ("depends_on", json!(["aws_vpc.main"])),
            ("providers", json!({"aws":"aws.shared"})),
        ] {
            let mut call = json!({"source":"./child"});
            call[argument] = value;
            let config = json!({"module":{"child":call}});
            let (path, scope) = match nested {
                true => {
                    std::fs::write(
                        root.join("main.tf.json"),
                        json!({"module":{"wrapper":{"source":"./wrapper"}}}).to_string(),
                    )
                    .unwrap();
                    ("wrapper/main.tf.json", "module.wrapper.module.child")
                }
                false => ("main.tf.json", "module.child"),
            };
            std::fs::write(root.join(path), config.to_string()).unwrap();
            assert_eq!(
                generator::generate(&root).unwrap_err(),
                format!(
                    "fixture generator does not support module meta-argument {argument}: {scope}"
                )
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

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
    assert_eq!(svg.matches("data-edge-kind=\"association\"").count(), 8);
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
