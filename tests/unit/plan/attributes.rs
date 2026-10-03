use crate::plan;

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
        assert_eq!(
            super::resolve(&refs, &instances, &BTreeMap::new()),
            (expected, complete)
        );
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
