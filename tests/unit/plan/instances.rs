use crate::plan;
use serde_json::{Value, json};

#[test]
fn large_fixture_attachments_select_one_worker_instead_of_both() {
    let raw = plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    let attachments: Vec<_> = raw
        .attributes
        .iter()
        .filter(|a| a.attribute == "target_id")
        .collect();
    assert_eq!(attachments.len(), 4);
    for attribute in attachments {
        assert!(attribute.complete);
        assert_eq!(attribute.sources.len(), 1);
        let target = &raw.nodes[attribute.target].address;
        let suffix = target.rsplit('.').next().unwrap();
        let worker = format!(
            "module.application.aws_instance.workers_{}[{}]",
            &suffix[..1],
            &suffix[1..]
        );
        assert_eq!(raw.nodes[attribute.sources[0]].address, worker);
    }
}

fn parse(module: Value, addresses: &[&str]) -> crate::model::TerraformGraph {
    plan::parse(&json!({
        "format_version": "1.2",
        "resource_changes": addresses.iter().map(|address| json!({"address": address})).collect::<Vec<_>>(),
        "configuration": {"root_module": module}
    }).to_string()).unwrap()
}

fn sources(
    raw: &crate::model::TerraformGraph,
    target: &str,
    attribute: &str,
) -> (Vec<String>, bool) {
    let target = raw.nodes.iter().position(|n| n.address == target).unwrap();
    let attr = raw
        .attributes
        .iter()
        .find(|a| a.target == target && a.attribute == attribute)
        .unwrap();
    let sources: Vec<_> = attr
        .sources
        .iter()
        .map(|&i| raw.nodes[i].address.clone())
        .collect();
    let edges: Vec<_> = raw
        .edges
        .iter()
        .filter(|e| e.to == target)
        .map(|e| raw.nodes[e.from].address.clone())
        .collect();
    assert_eq!(sources, edges);
    (sources, attr.complete)
}

#[test]
fn numeric_string_dynamic_and_attribute_indexes_select_the_right_instances() {
    for (reference, expected) in [
        ("aws_instance.app[0].id", vec!["aws_instance.app[0]"]),
        ("aws_instance.app[1].id", vec!["aws_instance.app[1]"]),
        (
            "aws_instance.app[\"blue\"].id",
            vec!["aws_instance.app[\"blue\"]"],
        ),
        (
            "aws_instance.app[\"0\"].id",
            vec!["aws_instance.app[\"0\"]"],
        ),
        (
            "aws_instance.app[\"a.b]\\\"c\"].id",
            vec!["aws_instance.app[\"a.b]\\\"c\"]"],
        ),
        ("aws_instance.app[99].id", vec![]),
    ] {
        let raw = parse(
            json!({"resources": [{"address": "test.consumer", "expressions": {
                "input": {"references": [reference, "aws_instance.app"]}
            }}]}),
            &[
                "test.consumer",
                "aws_instance.app[0]",
                "aws_instance.app[1]",
                "aws_instance.app[\"blue\"]",
                "aws_instance.app[\"0\"]",
                "aws_instance.app[\"a.b]\\\"c\"]",
            ],
        );
        assert_eq!(
            sources(&raw, "test.consumer", "input"),
            (
                expected.iter().map(|s| s.to_string()).collect(),
                !expected.is_empty()
            ),
            "{reference}"
        );
    }
    for reference in [
        "aws_instance.app",
        "aws_instance.app[*].id",
        "aws_instance.app[count.index].id",
        "aws_instance.app[var.keys[0]].id",
        "aws_instance.app.tags[0]",
    ] {
        let raw = parse(
            json!({"resources": [{"address": "test.consumer", "expressions": {
                "input": {"references": [reference]}
            }}]}),
            &[
                "test.consumer",
                "aws_instance.app[0]",
                "aws_instance.app[1]",
            ],
        );
        assert_eq!(
            sources(&raw, "test.consumer", "input"),
            (
                vec!["aws_instance.app[0]".into(), "aws_instance.app[1]".into()],
                true
            ),
            "{reference}"
        );
    }
}

#[test]
fn module_outputs_and_local_aliases_keep_nested_instance_keys() {
    for reference in [
        "module.service[0].module.child[\"blue\"].id",
        "module.service[0].module.child[\"blue\"].aws_instance.app[1].id",
        "module.service[0].module.child[\"blue\"]",
    ] {
        let raw = parse(
            json!({
                "resources": [{"address": "test.consumer", "expressions": {"input": {"references": [reference]}}}],
                "module_calls": {"service": {"module": {"module_calls": {"child": {"module": {
                    "locals": {"selected": {"references": ["aws_instance.app[1].id"]}},
                    "outputs": {"id": {"expression": {"references": ["local.selected"]}}},
                    "resources": [{"address": "aws_instance.app"}]
                }}}}}}
            }),
            &[
                "test.consumer",
                "module.service[0].module.child[\"blue\"].aws_instance.app[1]",
                "module.service[0].module.child[\"red\"].aws_instance.app[1]",
                "module.service[1].module.child[\"blue\"].aws_instance.app[1]",
            ],
        );
        assert_eq!(
            sources(&raw, "test.consumer", "input"),
            (
                vec!["module.service[0].module.child[\"blue\"].aws_instance.app[1]".into()],
                true
            )
        );
    }
}

#[test]
fn dynamic_module_keys_expand_only_within_known_ancestor_keys() {
    let raw = parse(
        json!({"resources": [{"address": "test.consumer", "expressions": {
            "input": {"references": ["module.service[0].module.child[each.key].aws_instance.app[1].id"]}
        }}]}),
        &[
            "test.consumer",
            "module.service[0].module.child[\"blue\"].aws_instance.app[1]",
            "module.service[0].module.child[\"red\"].aws_instance.app[1]",
            "module.service[1].module.child[\"blue\"].aws_instance.app[1]",
        ],
    );
    assert_eq!(sources(&raw, "test.consumer", "input").0.len(), 2);
}

#[test]
fn separate_whole_collection_expression_is_not_discarded() {
    let raw = parse(
        json!({"resources": [{"address": "test.consumer", "expressions": {
            "exact": {"references": ["aws_instance.app[0].id", "aws_instance.app"]},
            "all": {"references": ["aws_instance.app"]}
        }}]}),
        &[
            "test.consumer",
            "aws_instance.app[0]",
            "aws_instance.app[1]",
        ],
    );
    assert_eq!(raw.edges.len(), 2);
    assert_eq!(
        raw.attributes
            .iter()
            .find(|a| a.attribute == "exact")
            .unwrap()
            .sources
            .len(),
        1
    );
    assert_eq!(
        raw.attributes
            .iter()
            .find(|a| a.attribute == "all")
            .unwrap()
            .sources
            .len(),
        2
    );
}
