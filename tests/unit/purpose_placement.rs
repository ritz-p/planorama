use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write, fs};

const TYPES: [&str; 7] = [
    "aws_ssm_parameter",
    "aws_cloudwatch_metric_alarm",
    "aws_iam_role",
    "aws_iam_user",
    "aws_iam_group",
    "aws_iam_openid_connect_provider",
    "aws_iam_saml_provider",
];

fn eligible(node: &crate::model::Node) -> bool {
    node.provider.is_aws() && TYPES.contains(&node.resource_type.as_str())
}

fn monitoring() -> Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_db_instance.database","type":"aws_db_instance","change":{"actions":["create"],"after":{"identifier":"PRIVATE_DATABASE"}}},
        {"address":"aws_cloudwatch_metric_alarm.cpu","type":"aws_cloudwatch_metric_alarm","change":{"actions":["update"],"after":{"namespace":"AWS/RDS","metric_name":"CPUUtilization","dimensions":{"DBInstanceIdentifier":"PRIVATE_DATABASE"}}}},
        {"address":"aws_cloudwatch_metric_alarm.literal","type":"aws_cloudwatch_metric_alarm","change":{"actions":["create"],"after":{"namespace":"AWS/RDS","metric_name":"CPUUtilization","dimensions":{"DBInstanceIdentifier":"PRIVATE_DATABASE"}}}}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_cloudwatch_metric_alarm.cpu","expressions":{"dimensions":{"references":["aws_db_instance.database.id"]}}}
    ]}}})
}

fn repeat(input: &Value, count: usize) -> Value {
    fn rename(value: &mut Value, addresses: &[&str], copy: usize) {
        match value {
            Value::String(text) => {
                if let Some(address) = addresses.iter().find(|address| {
                    text.strip_prefix(**address)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
                }) {
                    *text = format!("{address}_copy{copy}{}", &text[address.len()..]);
                }
            }
            Value::Array(values) => {
                for value in values {
                    rename(value, addresses, copy);
                }
            }
            Value::Object(values) => {
                for value in values.values_mut() {
                    rename(value, addresses, copy);
                }
            }
            _ => {}
        }
    }
    if count == 1 {
        return input.clone();
    }
    let mut addresses: Vec<_> = input["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|resource| resource["address"].as_str().unwrap())
        .collect();
    addresses.sort_by_key(|address| std::cmp::Reverse(address.len()));
    let mut output = input.clone();
    output["resource_changes"] = json!([]);
    output["configuration"]["root_module"]["resources"] = json!([]);
    for copy in 0..count {
        let mut part = input.clone();
        rename(&mut part, &addresses, copy);
        for field in ["/resource_changes", "/configuration/root_module/resources"] {
            output
                .pointer_mut(field)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .extend(
                    part.pointer(field)
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned(),
                );
        }
    }
    output
}

#[test]
#[ignore = "purpose-dependent placement review; writes comparison SVGs and measurements"]
fn compare_purpose_placements() {
    let directory = std::path::Path::new("output/purpose-placement");
    fs::create_dir_all(directory).unwrap();
    let mut report = String::from(
        "| Scenario | Copies | Resources / relationships | View | Spatial / logical | Canvas | Visible reference rows |\n| --- | ---: | --- | --- | --- | --- | ---: |\n",
    );
    let scenarios = [
        (
            "infrastructure",
            serde_json::from_str(include_str!("../fixtures/viewer-plan.json")).unwrap(),
        ),
        ("monitoring", monitoring()),
        (
            "iam",
            serde_json::from_str(include_str!("../fixtures/iam-plan.json")).unwrap(),
        ),
        (
            "federation",
            serde_json::from_str(include_str!("../fixtures/identity-providers-plan.json")).unwrap(),
        ),
        (
            "configuration",
            serde_json::from_str(include_str!("../fixtures/ssm-plan.json")).unwrap(),
        ),
    ];
    let mut covered = BTreeSet::new();
    for (name, input) in scenarios {
        for copies in [1, 4] {
            let input = repeat(&input, copies);
            let original = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            let targets: BTreeSet<_> = original
                .nodes
                .iter()
                .filter(|node| eligible(node))
                .flat_map(|node| node.entity.provenance.iter().cloned())
                .collect();
            for node in &original.nodes {
                if eligible(node) {
                    covered.insert(node.resource_type.clone());
                    assert_eq!(node.placement, Placement::Logical);
                    assert!(node.member_owner.is_none());
                    assert!(node.routes.owner.is_none());
                }
            }
            for independent in [false, true] {
                let mut candidate = original.clone();
                if independent {
                    for node in &mut candidate.0.nodes {
                        if eligible(node) {
                            node.placement = Placement::Spatial;
                        }
                        for reference in &mut node.references {
                            for target in &mut reference.targets {
                                if targets.contains(&target.id) {
                                    target.placement = Placement::Spatial;
                                }
                            }
                        }
                    }
                }
                let projected = view::single(&candidate, &view::Options::default()).unwrap();
                assert_eq!(projected.nodes.len(), original.nodes.len());
                assert_eq!(projected.edges, original.edges);
                assert_eq!(projected.relationships, original.relationships);
                assert_eq!(projected.coverage, original.coverage);
                for (before, after) in original.nodes.iter().zip(&projected.nodes) {
                    assert_eq!(before.entity, after.entity);
                    assert_eq!(before.action, after.action);
                    assert_eq!(before.role, after.role);
                }
                let output = svg::render(&projected, &Layout::new(&projected));
                assert_eq!(
                    candidate,
                    view::single(&candidate, &view::Options::default())
                        .unwrap()
                        .0
                );
                assert!(!output.contains("PRIVATE_"));
                assert!(!output.contains("SYNTHETIC_PARAMETER_NOT_FOR_OUTPUT"));
                let doc = roxmltree::Document::parse(&output).unwrap();
                assert_eq!(
                    doc.descendants()
                        .filter(|node| node.attribute("data-terraform-address").is_some())
                        .count(),
                    original.nodes.len()
                );
                for href in doc.descendants().filter_map(|node| node.attribute("href")) {
                    assert!(
                        doc.descendants()
                            .any(|node| node.attribute("id") == href.strip_prefix('#'))
                    );
                }
                let root = doc.root_element();
                let logical = projected
                    .nodes
                    .iter()
                    .filter(|node| node.placement == Placement::Logical)
                    .count();
                let rows = projected
                    .nodes
                    .iter()
                    .map(|node| view::references::rows(node).len())
                    .sum::<usize>();
                let variant = if independent {
                    "independent"
                } else {
                    "logical"
                };
                writeln!(report, "| {name} | {copies} | {} / {} | {variant} | {} / {logical} | {} × {} | {rows} |",
                    projected.nodes.len(), projected.edges.len(), projected.nodes.len() - logical,
                    root.attribute("width").unwrap(), root.attribute("height").unwrap()).unwrap();
                fs::write(
                    directory.join(format!("{name}-{copies}-{variant}.svg")),
                    output,
                )
                .unwrap();
            }
        }
    }
    assert_eq!(covered, TYPES.into_iter().map(String::from).collect());
    fs::write(directory.join("measurements.md"), report).unwrap();
}
