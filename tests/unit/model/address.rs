use crate::{layout::Layout, plan, semantic, svg};
use serde_json::json;

#[test]
fn relationship_local_addresses_preserve_resource_keys_and_strip_nested_modules() {
    for (native, local) in [
        (
            "aws_route_table_association.main",
            "aws_route_table_association.main",
        ),
        (
            "module.root.aws_route_table_association.main[1]",
            "aws_route_table_association.main[1]",
        ),
        (
            r#"module.network["a.b"].module.routes[0].aws_route_table_association.main["x.y"]"#,
            r#"aws_route_table_association.main["x.y"]"#,
        ),
        (
            r#"module.network["a\".b"].aws_route_table_association.main["x\".y"]"#,
            r#"aws_route_table_association.main["x\".y"]"#,
        ),
    ] {
        let change = crate::model::EdgeChange {
            previous_address: None,
            metadata: Default::default(),
            address: native.into(),
            action: crate::model::Action::Create,
        };
        assert_eq!(change.local_address(), local);
        assert_eq!(change.address, native);
    }
}

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
        let graph = crate::semantic::base_graph(&raw);
        let node = &graph.nodes[0];
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
