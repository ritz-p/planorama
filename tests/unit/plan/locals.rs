use crate::{model::EdgeKind, plan, semantic};
use serde_json::json;

#[test]
fn local_chains_preserve_dependencies_and_containment_in_each_module() {
    let module = json!({
        "locals": {
            "network": {"references": ["aws_vpc.main.id"]},
            "vpc_id": {"expression": {"references": ["local.network"]}},
            "subnet_id": {"references": ["aws_subnet.main.id"]},
            "image": {"references": ["data.aws_ami.image.id"]}
        },
        "resources": [
            {"address": "aws_vpc.main"},
            {"address": "data.aws_ami.image"},
            {"address": "aws_subnet.main", "expressions": {
                "vpc_id": {"references": ["local.vpc_id"]}
            }},
            {"address": "aws_instance.app", "expressions": {
                "subnet_id": {"references": ["local.subnet_id"]},
                "ami": {"references": ["local.image"]}
            }}
        ]
    });
    let mut root = module.clone();
    root["module_calls"] = json!({"child": {"module": module}});
    let resources: Vec<_> = ["", "module.child."]
        .into_iter()
        .flat_map(|scope| {
            ["aws_vpc.main", "aws_subnet.main", "aws_instance.app", "data.aws_ami.image"]
                .map(|address| json!({
                    "address": format!("{scope}{address}"),
                    "type": if address.starts_with("data.") { "aws_ami" } else { address.split('.').next().unwrap() },
                    "mode": if address.starts_with("data.") { "data" } else { "managed" }
                }))
        })
        .collect();
    let raw = plan::parse(
        &json!({
            "format_version": "1.2", "resource_changes": resources,
            "configuration": {"root_module": root}
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(raw.edges.len(), 6);
    assert!(
        raw.attributes
            .iter()
            .all(|a| a.complete && a.sources.len() == 1)
    );
    for edge in &raw.edges {
        assert_eq!(raw.nodes[edge.from].module, raw.nodes[edge.to].module);
    }
    let architecture = semantic::transform(&raw);
    assert_eq!(
        architecture
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Containment)
            .count(),
        4
    );
}

#[test]
fn local_cycles_terminate_and_do_not_claim_complete_attribute_resolution() {
    let raw = plan::parse(
        &json!({
            "format_version": "1.2",
            "resource_changes": [{"address": "aws_vpc.main"}, {"address": "aws_subnet.main"}],
            "configuration": {"root_module": {
                "locals": {
                    "a": {"references": ["local.b", "aws_vpc.main.id"]},
                    "b": {"references": ["local.a"]},
                    "self": {"references": ["local.self"]},
                    "constant": {"constant_value": "local.a"}
                },
                "resources": [{"address": "aws_subnet.main", "expressions": {
                    "vpc_id": {"references": ["local.a"]},
                    "self": {"references": ["local.self"]},
                    "constant": {"references": ["local.constant"]},
                    "missing": {"references": ["local.missing"]}
                }}]
            }}
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(raw.edges.len(), 1);
    assert!(raw.attributes.iter().all(|a| !a.complete));
}

#[test]
fn shared_local_in_diamond_is_not_a_cycle() {
    let raw = plan::parse(
        &json!({
            "format_version": "1.2",
            "resource_changes": [{"address": "aws_vpc.main"}, {"address": "aws_subnet.main"}],
            "configuration": {"root_module": {
                "locals": {
                    "a": {"references": ["local.shared"]},
                    "b": {"references": ["local.shared"]},
                    "shared": {"references": ["aws_vpc.main.id"]}
                },
                "resources": [{"address": "aws_subnet.main", "expressions": {
                    "vpc_id": {"references": ["local.a", "local.b"]}
                }}]
            }}
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(raw.attributes[0].sources.len(), 1);
    assert!(raw.attributes[0].complete);
}
