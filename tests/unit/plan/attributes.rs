use crate::plan;

#[test]
fn direct_block_metadata_named_fields_keep_paths_and_literal_privacy() {
    use serde_json::json;
    for name in ["references", "constant_value"] {
        let raw = plan::parse(&json!({"format_version":"1.2","resource_changes":[
            {"address":"terraform_data.source","type":"terraform_data"},
            {"address":"terraform_data.consumer","type":"terraform_data"}
        ],"configuration":{"root_module":{"resources":[{"address":"terraform_data.consumer","expressions":{
            "block":{
                (name):{"references":["terraform_data.source.output"]},
                "sibling":{"references":["terraform_data.source.output"]},
                "deeper":{(name):{"references":["terraform_data.source.output"]},"sibling":{"constant_value":false}}
            },
            "literal":{"constant_value":{"references":["TOP_SECRET"],"sibling":{"references":["TOP_SECRET"]}}}
        }}]}}}).to_string()).unwrap();
        for path in [
            format!("block.{name}"),
            "block.sibling".into(),
            format!("block.deeper.{name}"),
        ] {
            let attribute = raw.attributes.iter().find(|r| r.attribute == path).unwrap();
            assert!(attribute.complete, "{path}");
            assert_eq!(attribute.sources.len(), 1);
        }
        assert_eq!(raw.edges.len(), 1);
        assert!(
            !raw.attributes
                .iter()
                .any(|r| r.attribute.starts_with("literal."))
        );
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    }
}

#[test]
fn repeated_block_fields_can_use_expression_metadata_names() {
    use serde_json::json;
    for name in ["references", "constant_value"] {
        let block = |source: &str| {
            json!({
                (name): {"references":[format!("terraform_data.{source}.output")]},
                "sibling":{"references":["terraform_data.sibling.output"]},
                "nested":[{(name):{"references":[format!("terraform_data.{source}.output")]}}],
                "literal":{"constant_value":{"references":["TOP_SECRET"],"constant_value":"TOP_SECRET"}}
            })
        };
        let mut input = json!({"format_version":"1.2", "resource_changes":[
            {"address":"terraform_data.a","type":"terraform_data"},
            {"address":"terraform_data.b","type":"terraform_data"},
            {"address":"terraform_data.sibling","type":"terraform_data"},
            {"address":"terraform_data.consumer","type":"terraform_data"}
        ], "configuration":{"root_module":{"resources":[{"address":"terraform_data.consumer","expressions":{
            "blocks":[block("a"),block("b")]
        }}]}}});
        let raw = plan::parse(&input.to_string()).unwrap();
        for path in [
            format!("blocks.{name}"),
            format!("blocks.nested.{name}"),
            "blocks.sibling".into(),
        ] {
            let attribute = raw.attributes.iter().find(|r| r.attribute == path).unwrap();
            assert!(attribute.complete, "{path}");
            assert_eq!(
                attribute.sources.len(),
                if path == "blocks.sibling" { 1 } else { 2 }
            );
        }
        assert_eq!(raw.edges.len(), 3);
        assert!(
            !raw.attributes
                .iter()
                .any(|r| r.attribute.starts_with("blocks.literal."))
        );
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
        input["configuration"]["root_module"]["resources"][0]["expressions"]["blocks"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert_eq!(raw, plan::parse(&input.to_string()).unwrap());
    }
}

#[test]
fn generic_nested_paths_preserve_fields_depth_scope_and_repeated_blocks() {
    use serde_json::json;
    let block = |name: &str| {
        json!({
            "target_group_arn":{"references":[format!("aws_lb_target_group.{name}.arn")]},
            "forward":[{"target_group":[{"arn":{"references":[format!("aws_lb_target_group.{name}.arn")]}}]}]
        })
    };
    let mut input = json!({"format_version":"1.2", "resource_changes":[
        {"address":"module.app.aws_lb_target_group.a","type":"aws_lb_target_group"},
        {"address":"module.app.aws_lb_target_group.b","type":"aws_lb_target_group"},
        {"address":"module.app.aws_lb_listener.main","type":"aws_lb_listener"}
    ], "configuration":{"root_module":{"module_calls":{"app":{"module":{"resources":[{
        "address":"aws_lb_listener.main",
        "unrelated_metadata":{"references":["aws_lb_target_group.a.arn"]},
        "expressions":{
            "default_action":[block("a"),block("b")],
            "network_configuration":{"subnets":{"references":["aws_lb_target_group.a.arn"]},"security_groups":{"references":["aws_lb_target_group.b.arn"]}},
            "tags":{"constant_value":{"references":["TOP_SECRET"],"nested":{"references":["aws_lb_target_group.a.arn"]}}}
        }
    }]}}}}}});
    let raw = plan::parse(&input.to_string()).unwrap();
    for (name, count) in [
        ("default_action", 2),
        ("default_action.target_group_arn", 2),
        ("default_action.forward.target_group.arn", 2),
        ("network_configuration.subnets", 1),
        ("network_configuration.security_groups", 1),
    ] {
        let attr = raw.attributes.iter().find(|r| r.attribute == name).unwrap();
        assert!(attr.complete, "{name}");
        assert_eq!(attr.sources.len(), count);
        assert!(attr.sources.iter().all(|&i| {
            raw.nodes[i]
                .address
                .starts_with("module.app.aws_lb_target_group.")
        }));
    }
    assert!(
        !raw.attributes
            .iter()
            .any(|r| r.attribute.starts_with("tags.")
                || r.attribute.contains("references")
                || r.attribute.contains("metadata"))
    );
    assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    input["configuration"]["root_module"]["module_calls"]["app"]["module"]["resources"][0]["expressions"]["default_action"].as_array_mut().unwrap().reverse();
    assert_eq!(raw, plan::parse(&input.to_string()).unwrap());
}

#[test]
fn repeated_nested_fields_require_complete_provenance_in_every_block() {
    use serde_json::json;
    for other in [
        json!({}),
        json!({"target":{"constant_value":"id"}}),
        json!({"target":{"references":["var.missing"]}}),
        json!({"target":{"references":["terraform_data.a.id", false]}}),
    ] {
        let raw = plan::parse(&json!({"format_version":"1.2", "resource_changes":[
            {"address":"terraform_data.a","type":"terraform_data"},
            {"address":"terraform_data.b","type":"terraform_data"}
        ], "configuration":{"root_module":{"resources":[{"address":"terraform_data.b","expressions":{
            "blocks":[{"target":{"references":["terraform_data.a.id"]}},other]
        }}]}}}).to_string()).unwrap();
        let attr = raw
            .attributes
            .iter()
            .find(|r| r.attribute == "blocks.target")
            .unwrap();
        assert_eq!(attr.sources.len(), 1);
        assert!(!attr.complete);
    }
}

#[test]
fn absent_resources_do_not_resolve_to_their_configuration_dependencies() {
    use serde_json::json;
    for scope in ["", "module.child.", "module.parent.module.child."] {
        for resource in ["aws_instance.optional", "data.aws_instance.optional"] {
            let mut module = json!({
                "resources": [
                    {"address": "data.aws_ami.image"},
                    {"address": resource, "count_expression": {"constant_value": 0},
                     "expressions": {"ami": {"references": ["data.aws_ami.image.id"]}}},
                    {"address": "terraform_data.consumer", "expressions": {
                        "missing": {"references": [format!("{resource}[*].id")]},
                        "mixed": {"references": [format!("{resource}[*].id"), "data.aws_ami.image.id" ]},
                        "existing": {"references": ["data.aws_ami.image.id"]}
                    }}
                ]
            });
            let names: Vec<_> = scope.split('.').filter(|part| !part.is_empty()).collect();
            for pair in names.chunks(2).rev() {
                module = json!({"module_calls": {pair[1]: {"module": module}}});
            }
            let raw = plan::parse(
                &json!({
                    "format_version": "1.2",
                    "resource_changes": [
                        {"address": format!("{scope}data.aws_ami.image")},
                        {"address": format!("{scope}terraform_data.consumer")}
                    ],
                    "configuration": {"root_module": module}
                })
                .to_string(),
            )
            .unwrap();
            let source = raw
                .nodes
                .iter()
                .position(|node| node.address == format!("{scope}data.aws_ami.image"))
                .unwrap();
            for (name, sources, complete) in [
                ("missing", vec![], false),
                ("mixed", vec![source], false),
                ("existing", vec![source], true),
            ] {
                let attribute = raw
                    .attributes
                    .iter()
                    .find(|attribute| attribute.attribute == name)
                    .unwrap();
                assert_eq!(
                    (&attribute.sources, attribute.complete),
                    (&sources, complete),
                    "{scope}{resource}: {name}"
                );
            }
        }
    }
}

#[test]
fn whole_module_attributes_include_all_descendant_instances() {
    let raw = plan::parse(include_str!("../../fixtures/terraform-plan.json")).unwrap();
    let target = raw
        .nodes
        .iter()
        .position(|n| n.address == "terraform_data.consumer")
        .unwrap();
    let attribute = raw
        .attributes
        .iter()
        .find(|a| a.target == target && a.attribute == "input")
        .unwrap();
    let expected: Vec<_> = raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.address.starts_with("module.service["))
        .map(|(i, _)| i)
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(attribute.sources, expected);
    assert!(attribute.complete);
}

#[test]
fn module_expansion_respects_boundaries_and_incomplete_references() {
    use std::collections::{BTreeMap, BTreeSet};
    let instances = BTreeMap::from([
        ("module.app.aws_vpc.main".into(), vec![0]),
        ("module.app.module.child.aws_subnet.main".into(), vec![1]),
        ("module.apple.aws_vpc.main".into(), vec![2]),
    ]);
    for (refs, expected, complete) in [
        (vec!["module.app"], vec![0, 1], true),
        (vec!["module.missing"], vec![], false),
        (vec!["module.app", "module.missing"], vec![0, 1], false),
    ] {
        let refs: BTreeSet<_> = refs.into_iter().map(str::to_owned).collect();
        let result = super::resolve(&refs, &instances, &BTreeMap::new());
        assert_eq!((result.sources, result.complete), (expected, complete));
    }
}

#[test]
fn attribute_provenance_excludes_depends_on_and_marks_partial_resolution() {
    let raw = plan::parse(
        r#"{
        "format_version":"1.2",
        "resource_changes":[{"address":"aws_vpc.v"},{"address":"aws_subnet.s"}],
        "configuration":{"root_module":{"resources":[
            {"address":"aws_vpc.v"},
            {"address":"aws_subnet.s","depends_on":["aws_vpc.v"],"expressions":{
                "vpc_id":{"references":["aws_vpc.v.id"]},
                "tags":{"references":["aws_vpc.v.id","var.unknown"]},
                "cidr_block":{"constant_value":"10.0.0.0/24"}
            }}
        ]}}
    }"#,
    )
    .unwrap();
    assert_eq!(raw.attributes.len(), 3);
    let vpc = raw
        .nodes
        .iter()
        .position(|n| n.address == "aws_vpc.v")
        .unwrap();
    let explicit = raw
        .attributes
        .iter()
        .find(|a| a.attribute == "vpc_id")
        .unwrap();
    assert_eq!(explicit.sources, [vpc]);
    assert!(explicit.complete);
    assert!(
        !raw.attributes
            .iter()
            .find(|a| a.attribute == "tags")
            .unwrap()
            .complete
    );
    assert!(
        !raw.attributes
            .iter()
            .find(|a| a.attribute == "cidr_block")
            .unwrap()
            .complete
    );
}

#[test]
fn module_input_references_are_resolved_without_copying_attribute_values() {
    let raw = plan::parse(include_str!("../../../examples/plan.json")).unwrap();
    let subnet = raw
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_subnet")
        .unwrap();
    let vpc = raw
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_vpc")
        .unwrap();
    let reference = raw
        .attributes
        .iter()
        .find(|a| a.target == subnet && a.attribute == "vpc_id")
        .unwrap();
    assert_eq!(reference.sources, [vpc]);
    assert!(reference.complete);
}
