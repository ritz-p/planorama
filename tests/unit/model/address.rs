use crate::{layout::Layout, plan, semantic, svg};
use serde_json::json;

#[test]
fn qualified_addresses_preserve_native_module_and_instance_identity() {
    for (native, module, local, qualified) in [
        (
            "aws_vpc.main",
            "root",
            "aws_vpc.main",
            "module.root.aws_vpc.main",
        ),
        (
            "data.aws_vpc.shared",
            "root",
            "data.aws_vpc.shared",
            "module.root.data.aws_vpc.shared",
        ),
        (
            "aws_instance.app[0]",
            "root",
            "aws_instance.app[0]",
            "module.root.aws_instance.app[0]",
        ),
        (
            r#"aws_instance.app["a.b"]"#,
            "root",
            r#"aws_instance.app["a.b"]"#,
            r#"module.root.aws_instance.app["a.b"]"#,
        ),
        (
            "module.app.aws_instance.web",
            "module.app",
            "aws_instance.web",
            "module.app.aws_instance.web",
        ),
        (
            "module.root.aws_vpc.main",
            "module.root",
            "aws_vpc.main",
            "module.root.aws_vpc.main",
        ),
        (
            r#"module.app["a.b"].module.workers[0].aws_instance.web["x\"y"]"#,
            r#"module.app["a.b"].module.workers[0]"#,
            r#"aws_instance.web["x\"y"]"#,
            r#"module.app["a.b"].module.workers[0].aws_instance.web["x\"y"]"#,
        ),
    ] {
        let raw = plan::parse(
            &json!({"format_version":"1.2","resource_changes":[{"address":native}]}).to_string(),
        )
        .unwrap();
        let node = &raw.nodes[0];
        let address = node.resource_address();
        assert_eq!(node.address, native);
        assert_eq!(node.module, module);
        assert_eq!(address.terraform(), native);
        assert_eq!(address.qualified(), qualified);
        assert_eq!(address.local(), local);
        assert_eq!(address.qualified(), address.qualified());
    }
}

#[test]
fn display_qualification_does_not_change_resolution_or_layout() {
    let raw = plan::parse(include_str!("../../../examples/plan.json")).unwrap();
    let original = raw.clone();
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    let positions = layout.positions.clone();
    let paths = layout.paths.clone();
    let output = svg::render(&graph, &layout);
    assert!(output.contains("data-qualified-address=\"module.root.aws_vpc"));
    assert_eq!(raw, original);
    assert!(!raw.edges.is_empty());
    assert!(
        raw.nodes
            .iter()
            .any(|node| node.module == "root" && !node.address.starts_with("module."))
    );
    assert_eq!(positions, Layout::new(&graph).positions);
    assert_eq!(paths, Layout::new(&graph).paths);
}
