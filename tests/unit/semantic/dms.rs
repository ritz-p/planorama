use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/dms-plan.json");

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == "aws_dms_replication_instance.main"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

fn verified(v: &Value, address: &str, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label) == super::label(address.split('.').next().unwrap(), field)
        })
}

#[test]
fn dms_network_and_directional_task_links_do_not_infer_database_ownership() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 12);
    for node in &g.nodes {
        assert!(node.member_owner.is_none());
        if node.resource_type.starts_with("aws_dms_")
            && node.address != "aws_dms_replication_instance.main"
        {
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == EdgeKind::Containment
                        && g.nodes[e.to].address == node.address)
            );
        }
    }
    for field in [
        "replication_instance_arn",
        "source_endpoint_arn",
        "target_endpoint_arn",
    ] {
        assert!(verified(&v, "aws_dms_replication_task.verified", field));
        assert!(!verified(&v, "aws_dms_replication_task.literal", field));
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("Source endpoint"));
        assert!(out.contains("Target endpoint"));
        assert!(out.contains("Replication instance"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:dms:"));
    }
}

#[test]
fn dms_references_require_exact_outputs_values_scope_and_current_managed_identity() {
    for (target, source, field, output) in [
        (6, 5, "replication_subnet_group_id", "id"),
        (9, 6, "replication_instance_arn", "replication_instance_arn"),
        (9, 7, "source_endpoint_arn", "endpoint_arn"),
        (9, 8, "target_endpoint_arn", "endpoint_arn"),
    ] {
        for case in [
            "literal",
            "wrong-output",
            "mismatch",
            "unknown",
            "source-unknown",
            "wrong-type",
            "deleted",
            "deposed",
            "data",
            "provider",
            "region",
            "ambiguous",
            "consumer-delete",
            "consumer-deposed",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let address = v["resource_changes"][target]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"constant_value":"literal"})
                }
                "wrong-output" => {
                    let source_address = v["resource_changes"][source]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":[format!("{source_address}.arn")]});
                }
                "mismatch" => {
                    v["resource_changes"][target]["change"]["after"][field] = json!("other")
                }
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({field:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_dms_s3_endpoint")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-delete" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "region" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":["var.dynamic"]})
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, &address, field), "{field}: {case}");
        }
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["root_module"]["resources"][6]["expressions"]["replication_subnet_group_id"] = json!({"references":["aws_dms_replication_subnet_group.shared.replication_subnet_group_id"]});
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
}

#[test]
fn dms_endpoint_direction_requires_known_type_for_each_task_role() {
    for (source, field, wrong) in [
        (7, "source_endpoint_arn", "target"),
        (8, "target_endpoint_arn", "source"),
    ] {
        for case in ["wrong", "unknown", "missing"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "wrong" => {
                    v["resource_changes"][source]["change"]["after"]["endpoint_type"] = json!(wrong)
                }
                "unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] =
                        json!({"endpoint_type":true})
                }
                "missing" => {
                    v["resource_changes"][source]["change"]["after"]
                        .as_object_mut()
                        .unwrap()
                        .remove("endpoint_type");
                }
                _ => unreachable!(),
            }
            assert!(
                !verified(&v, "aws_dms_replication_task.verified", field),
                "{field}: {case}"
            );
            assert!(verified(
                &v,
                "aws_dms_replication_task.verified",
                "replication_instance_arn"
            ));
        }
    }
}

#[test]
fn dms_network_requires_complete_common_vpc_evidence_not_computed_defaults() {
    for case in [
        "extra-subnet",
        "unknown-subnet",
        "partial-subnet",
        "extra-sg",
        "unknown-sg",
        "missing-sg",
        "literal",
        "mixed-vpc",
        "deleted-subnet",
        "data-subnet",
        "unknown-vpc",
        "single-subnet",
        "duplicate-output",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "extra-subnet" => v["resource_changes"][5]["change"]["after"]["subnet_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("unproven")),
            "unknown-subnet" => {
                v["resource_changes"][5]["change"]["after_unknown"] = json!({"subnet_ids":true})
            }
            "partial-subnet" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"subnet_ids":[false,true]})
            }
            "extra-sg" => v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("unproven")),
            "unknown-sg" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"vpc_security_group_ids":true})
            }
            "missing-sg" => {
                v["resource_changes"][6]["change"]["after"]
                    .as_object_mut()
                    .unwrap()
                    .remove("vpc_security_group_ids");
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["subnet_ids"] =
                    json!({"constant_value":["subnet-a","subnet-b"]})
            }
            "mixed-vpc" => {
                v["resource_changes"][4]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][4]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            "deleted-subnet" => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            "data-subnet" => v["resource_changes"][2]["mode"] = json!("data"),
            "unknown-vpc" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
            }
            "duplicate-output" => {
                v["resource_changes"][3]["change"]["after"]["id"] = json!("subnet-a")
            }
            "single-subnet" => {
                v["resource_changes"][5]["change"]["after"]["subnet_ids"] = json!(["subnet-a"]);
                v["configuration"]["root_module"]["resources"][5]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_subnet.a.id"]});
            }
            _ => unreachable!(),
        }
        if case == "single-subnet" {
            assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
        } else {
            assert!(parent(&v).is_none(), "{case}");
        }
    }
}

#[test]
fn dms_named_states_filtering_and_lifecycle_keep_exact_coverage() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][9]["previous_address"] = json!("aws_dms_replication_task.old");
    v["resource_changes"][9]["change"]["actions"] = json!(["update"]);
    let mut old = v["resource_changes"][9].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for depth in [0, 1, 2] {
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                focus: Some("aws_dms_replication_instance.main".into()),
                depth: Some(depth),
                ..Default::default()
            },
        )
        .unwrap();
        let placed = selected.edges.iter().any(|e| {
            e.kind == EdgeKind::Containment
                && selected.nodes[e.to].address == "aws_dms_replication_instance.main"
        });
        assert_eq!(placed, depth == 2, "focus depth {depth}");
    }
    for source in &parsed.nodes {
        let found = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, source.action);
        assert_eq!(found[0].previous_address, source.previous_address);
    }
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_dms_replication_instance.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment),
        "{:?}",
        selected
            .edges
            .iter()
            .map(|e| (
                &selected.nodes[e.from].address,
                &selected.nodes[e.to].address,
                e.kind
            ))
            .collect::<Vec<_>>()
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(7);
    assert!(!verified(
        &missing,
        "aws_dms_replication_task.verified",
        "source_endpoint_arn"
    ));
    let out = crate::generate_named(
        &[
            crate::NamedInput {
                state: "one".into(),
                json: v.to_string(),
            },
            crate::NamedInput {
                state: "two".into(),
                json: missing.to_string(),
            },
        ],
        &crate::Options {
            format: crate::OutputFormat::Html,
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(!out.contains("PRIVATE_"));
    assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    assert!(out.contains("data-state-id=\"one\""));
    assert!(out.contains("data-state-id=\"two\""));
}

#[test]
fn dms_retains_verified_account_scope_and_rejects_incompatible_accounts() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_dms_"))
    {
        assert_eq!(
            n.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012"
        );
    }
    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"},"assume_role":[{"role_arn":{"constant_value":"arn:aws:iam::999999999999:role/Test"}}]}});
    v["configuration"]["root_module"]["resources"][7]["provider_config_key"] = json!("other");
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    assert!(
        !parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "source_endpoint_arn" && r.service_reference_verified)
    );
}
