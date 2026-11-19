use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/dms-serverless-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}
fn proven(v: &Value, address: &str, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && Some(r.label) == super::label(&n.resource_type, path)
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
            })
        })
}
#[test]
fn serverless_replication_preserves_independent_workload_identity() {
    let v = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    let mut checked = 0;
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_some() {
            assert!(
                proven(&v, &n.address, &r.attribute),
                "{} {} {r:?}",
                n.address,
                r.attribute
            );
            checked += 1;
        }
    }
    assert!(checked >= 5, "{checked}");
    assert_eq!(g.nodes.len(), raw.nodes.len());
    assert_eq!(g.coverage.selected.resources, raw.nodes.len());
    for n in &raw.nodes {
        let nodes: Vec<_> = g.nodes.iter().filter(|a| a.address == n.address).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
    }
}
#[test]
fn serverless_replication_rejects_incompatible_missing_and_lifecycle_evidence() {
    let base = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_none() {
            continue;
        }
        let si = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let ti = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == n.address)
            .unwrap();
        let field = r.source_attributes_by_instance[&r.sources[0]]
            .first()
            .unwrap();
        for case in 0..10 {
            let mut v = base.clone();
            match case {
                0 => v["resource_changes"][si]["change"]["after"][field] = json!("other"),
                1 => v["resource_changes"][si]["change"]["after_unknown"] = json!({field:true}),
                2 => v["resource_changes"][si]["change"]["after_sensitive"] = json!({field:true}),
                3 => v["resource_changes"][si]["mode"] = json!("data"),
                4 => v["resource_changes"][si]["deposed"] = json!("old"),
                5 => v["resource_changes"][si]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][si]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][si]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => v["configuration"]["root_module"]["resources"][ti]["expressions"] = json!({}),
                _ => v["resource_changes"][ti]["change"]["after_unknown"] = json!(true),
            }
            assert!(
                !proven(&v, &n.address, &r.attribute),
                "{} {} case{case}",
                n.address,
                r.attribute
            );
        }
    }
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
fn parent(v: &Value, address: &str) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| e.kind == crate::model::EdgeKind::Containment && g.nodes[e.to].address == address)
        .map(|e| g.nodes[e.from].address.clone())
}
#[test]
fn serverless_workload_has_its_own_spatial_card_without_a_synthetic_instance() {
    let v = fixture();
    assert_eq!(
        parent(&v, "aws_dms_replication_config.main").as_deref(),
        Some("aws_vpc.main")
    );
    assert!(parent(&v, "aws_dms_replication_config.literal").is_none());
    let g = graph(&v);
    assert_eq!(g.nodes.len(), 11);
    assert!(
        !g.nodes
            .iter()
            .any(|n| n.resource_type == "aws_dms_replication_instance")
    );
    for n in g.nodes.iter().filter(|n| n.resource_type == super::CONFIG) {
        assert_eq!(n.placement, crate::model::Placement::Spatial);
        assert!(n.member_owner.is_none());
    }
    assert!(super::fields(super::CONFIG, super::KEY, "aws_kms_alias").is_empty());
    for mode in ["full-load", "cdc", "full-load-and-cdc"] {
        let mut v = fixture();
        let i = index(&v, "aws_dms_replication_config.main");
        v["resource_changes"][i]["change"]["after"]["replication_type"] = json!(mode);
        assert!(proven(
            &v,
            "aws_dms_replication_config.main",
            "source_endpoint_arn"
        ));
    }
}
#[test]
fn serverless_network_requires_complete_subnet_group_and_sg_common_vpc_evidence() {
    let base = fixture();
    let address = "aws_dms_replication_config.main";
    let t = index(&base, address);
    let group = index(&base, "aws_dms_replication_subnet_group.main");
    let subnet = index(&base, "aws_subnet.second");
    let sg = index(&base, "aws_security_group.main");
    for case in 0..9 {
        let mut v = base.clone();
        match case {
            0 => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"compute_config":[{"vpc_security_group_ids":true}]})
            }
            1 => {
                v["resource_changes"][t]["change"]["after"]["compute_config"][0]["vpc_security_group_ids"] =
                    json!([])
            }
            2 => {
                v["configuration"]["root_module"]["resources"][t]["expressions"]["compute_config"]
                    [0]["replication_subnet_group_id"] = json!({})
            }
            3 => {
                v["resource_changes"][group]["change"]["after_unknown"] = json!({"subnet_ids":true})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][group]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_subnet.first.id"]})
            }
            5 => {
                v["resource_changes"][subnet]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][subnet]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            6 => {
                v["configuration"]["root_module"]["resources"][sg]["expressions"]["vpc_id"] =
                    json!({})
            }
            7 => v["resource_changes"][subnet]["change"]["after_unknown"] = json!({"vpc_id":true}),
            _ => {
                v["resource_changes"][t]["change"]["after"]["compute_config"][0]["replication_subnet_group_id"] =
                    Value::Null
            }
        }
        assert!(
            !proven(&v, address, super::SUBNETS),
            "subnet group case{case}"
        );
        assert!(!proven(&v, address, super::GROUPS), "SG case{case}");
        assert!(parent(&v, address).is_none());
        assert!(proven(&v, address, super::KEY));
        assert!(proven(&v, address, "target_endpoint_arn"));
    }
}
#[test]
fn serverless_modes_endpoint_direction_and_exact_key_shape_are_required() {
    let base = fixture();
    let address = "aws_dms_replication_config.main";
    let t = index(&base, address);
    for case in 0..5 {
        let mut v = base.clone();
        let endpoint = index(&v, "aws_dms_endpoint.source");
        match case {
            0 => v["resource_changes"][t]["change"]["after"]["replication_type"] = json!("unknown"),
            1 => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"replication_type":true})
            }
            2 => {
                v["resource_changes"][endpoint]["change"]["after"]["endpoint_type"] =
                    json!("target")
            }
            3 => {
                v["configuration"]["root_module"]["resources"][t]["expressions"]["source_endpoint_arn"] =
                    json!({"references":["aws_dms_endpoint.source.endpoint_id"]})
            }
            _ => {
                let a = &mut v["resource_changes"][t]["change"]["after"]["source_endpoint_arn"];
                *a = json!([a.clone()]);
            }
        }
        assert!(
            !proven(&v, address, "source_endpoint_arn"),
            "source case{case}"
        );
    }
    for case in 0..3 {
        let mut v = base.clone();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][t]["expressions"]["compute_config"]
                    [0]["kms_key_id"] = json!({})
            }
            1 => {
                v["resource_changes"][t]["change"]["after"]["compute_config"][0]["kms_key_id"] =
                    json!("serverless");
                v["configuration"]["root_module"]["resources"][t]["expressions"]["compute_config"]
                    [0]["kms_key_id"] = json!({"references":["aws_kms_key.main.key_id"]});
            }
            _ => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"compute_config":[{"kms_key_id":true}]})
            }
        }
        assert!(!proven(&v, address, super::KEY));
        assert_eq!(parent(&v, address).as_deref(), Some("aws_vpc.main"));
    }
    let mut v = base.clone();
    let row = v["resource_changes"][t]["change"]["after"]["compute_config"][0].clone();
    v["resource_changes"][t]["change"]["after"]["compute_config"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert!(!proven(&v, address, super::KEY));
    assert!(parent(&v, address).is_none());
}
#[test]
fn serverless_scope_filtered_views_and_named_states_keep_resource_identity() {
    let mut raw = crate::plan::parse(FIXTURE).unwrap();
    for n in &mut raw.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_vpc" {
                "222222222222"
            } else {
                "111111111111"
            });
    }
    let g = crate::semantic::transform(&raw);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type == super::CONFIG)
    );
    let mut v = fixture();
    let source = index(&v, "aws_dms_endpoint.source");
    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
        json!("aws.other");
    assert!(!proven(
        &v,
        "aws_dms_replication_config.main",
        "source_endpoint_arn"
    ));
    let g = graph(&fixture());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_dms_replication_config.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 2);
    assert!(selected.nodes.iter().all(|n| matches!(
        n.address.as_str(),
        "aws_dms_replication_config.main" | "aws_vpc.main"
    )));
    let mut missing = fixture();
    let i = index(&missing, "aws_dms_replication_subnet_group.main");
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(i);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "first".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "second".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
