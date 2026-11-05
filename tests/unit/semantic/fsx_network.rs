use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/fsx-network-plan.json")).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, index: usize, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][index]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][index]["type"].as_str().unwrap(),
                        field,
                    )
        })
}
fn contained(v: &Value, index: usize) -> bool {
    let g = graph(v);
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.to].address == v["resource_changes"][index]["address"].as_str().unwrap()
    })
}

#[test]
fn variants_have_complete_network_references_and_independent_cards() {
    let v = fixture();
    for i in 11..15 {
        for field in ["subnet_ids", "security_group_ids", "kms_key_id"] {
            assert!(complete(&v, i, field), "{i} {field}");
        }
        assert!(contained(&v, i), "{i}");
    }
    for (i, field) in [
        (11, "active_directory_id"),
        (11, "backup_id"),
        (11, "audit_log_configuration.audit_log_destination"),
        (12, "route_table_ids"),
        (13, "route_table_ids"),
        (13, "backup_id"),
    ] {
        assert!(complete(&v, i, field), "{i} {field}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 15);
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_fsx_") && n.resource_type != "aws_fsx_backup")
    {
        assert_eq!(n.placement, crate::model::Placement::Spatial);
        assert!(n.member_owner.is_none());
    }
}

#[test]
fn variants_reject_incomplete_ambiguous_unknown_lifecycle_and_scope() {
    for i in 11..15 {
        for (field, source, output) in [
            ("subnet_ids", 1, "id"),
            ("security_group_ids", 2, "id"),
            ("kms_key_id", 4, "arn"),
        ] {
            for case in [
                "literal",
                "unknown",
                "source-unknown",
                "mismatch",
                "delete",
                "deposed",
                "consumer-delete",
                "consumer-deposed",
                "foreign",
                "wrong-output",
                "partial",
                "provider",
            ] {
                let mut v = fixture();
                match case {
                    "literal" => {
                        v["configuration"]["root_module"]["resources"][i]["expressions"][field] =
                            json!({"constant_value":"opaque"})
                    }
                    "unknown" => {
                        v["resource_changes"][i]["change"]["after_unknown"] = json!({field:true})
                    }
                    "source-unknown" => {
                        v["resource_changes"][source]["change"]["after_unknown"] =
                            json!({output:true})
                    }
                    "mismatch" => {
                        v["resource_changes"][source]["change"]["after"][output] =
                            json!("different")
                    }
                    "delete" => {
                        v["resource_changes"][source]["change"]["actions"] = json!(["delete"])
                    }
                    "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                    "consumer-delete" => {
                        v["resource_changes"][i]["change"]["actions"] = json!(["delete"])
                    }
                    "consumer-deposed" => v["resource_changes"][i]["deposed"] = json!("old"),
                    "foreign" => {
                        v["resource_changes"][source]["provider_name"] =
                            json!("example.org/foreign/aws")
                    }
                    "wrong-output" => {
                        v["configuration"]["root_module"]["resources"][i]["expressions"][field] = json!({"references":[format!("{}.wrong",v["resource_changes"][source]["address"].as_str().unwrap())]})
                    }
                    "partial" => v["configuration"]["root_module"]["resources"][i]["expressions"]
                        [field]["references"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("var.unresolved")),
                    "provider" => {
                        v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                            json!("missing")
                    }
                    _ => unreachable!(),
                }
                assert!(!complete(&v, i, field), "{i} {field} {case}");
                if field != "kms_key_id" {
                    assert!(!contained(&v, i), "{i} {field} {case}");
                }
            }
        }
    }
}

#[test]
fn variants_require_deployment_subnet_membership_and_common_vpc() {
    for i in 11..15 {
        for case in [
            "extra",
            "duplicate",
            "vpc",
            "group-vpc",
            "preferred",
            "mode",
        ] {
            if i == 14 && matches!(case, "preferred" | "mode") {
                continue;
            }
            let mut v = fixture();
            match case {
                "extra" => v["resource_changes"][i]["change"]["after"]["subnet_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("literal")),
                "duplicate" => v["resource_changes"][i]["change"]["after"]["subnet_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("subnet-app")),
                "vpc" => v["resource_changes"][i]["change"]["after"]["vpc_id"] = json!("different"),
                "group-vpc" => {
                    v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("different")
                }
                "preferred" => {
                    v["resource_changes"][i]["change"]["after"]["preferred_subnet_id"] =
                        json!("different")
                }
                "mode" => {
                    v["resource_changes"][i]["change"]["after"]["deployment_type"] =
                        json!("UNKNOWN")
                }
                _ => unreachable!(),
            }
            assert!(!contained(&v, i), "{i} {case}");
        }
    }
}

#[test]
fn auxiliary_references_require_provenance_and_enabled_auditing() {
    for (i, field, source) in [
        (11, "backup_id", 7),
        (11, "active_directory_id", 8),
        (12, "route_table_ids", 6),
        (13, "backup_id", 7),
        (13, "preferred_subnet_id", 1),
    ] {
        for case in ["literal", "unknown", "wrong-source", "delete", "deposed"] {
            let mut v = fixture();
            match case {
                "literal" => v["configuration"]["root_module"]["resources"][i]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field),
                "unknown" => {
                    v["resource_changes"][i]["change"]["after_unknown"] = json!({field:true});
                    None
                }
                "wrong-source" => {
                    v["resource_changes"][source]["type"] = json!("aws_s3_bucket");
                    None
                }
                "delete" => {
                    v["resource_changes"][source]["change"]["actions"] = json!(["delete"]);
                    None
                }
                "deposed" => {
                    v["resource_changes"][source]["deposed"] = json!("old");
                    None
                }
                _ => unreachable!(),
            };
            assert!(!complete(&v, i, field), "{i} {field} {case}");
            if matches!(field, "route_table_ids" | "preferred_subnet_id") {
                assert!(!contained(&v, i));
            }
        }
    }
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after"]["audit_log_configuration"][0]["file_access_audit_log_level"] =
        json!("DISABLED");
    assert!(!complete(
        &v,
        11,
        "audit_log_configuration.audit_log_destination"
    ));
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after"]["self_managed_active_directory"] = json!([{"domain_join_service_account_secret":v["resource_changes"][9]["change"]["after"]["arn"],"domain_name":"PRIVATE_DOMAIN","password":"PRIVATE_PASSWORD"}]);
    v["resource_changes"][11]["change"]["after"]
        .as_object_mut()
        .unwrap()
        .remove("active_directory_id");
    v["configuration"]["root_module"]["resources"][11]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("active_directory_id");
    v["configuration"]["root_module"]["resources"][11]["expressions"]["self_managed_active_directory"] = json!([{"domain_join_service_account_secret":{"references":["aws_secretsmanager_secret.join.arn"]}}]);
    assert!(complete(
        &v,
        11,
        "self_managed_active_directory.domain_join_service_account_secret"
    ));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &v.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
    }
}

#[test]
fn single_az_variants_and_filtered_cards_preserve_identity() {
    for i in 11..14 {
        let mut v = fixture();
        v["resource_changes"][i]["change"]["after"]["deployment_type"] = json!("SINGLE_AZ_2");
        v["resource_changes"][i]["change"]["after"]["subnet_ids"] = json!(["subnet-app"]);
        v["configuration"]["root_module"]["resources"][i]["expressions"]["subnet_ids"] =
            json!({"references":["aws_subnet.app.id"]});
        if i != 12 {
            v["resource_changes"][i]["change"]["after"]
                .as_object_mut()
                .unwrap()
                .remove("preferred_subnet_id");
            v["resource_changes"][i]["change"]["after_unknown"] =
                json!({"preferred_subnet_id":false});
            v["configuration"]["root_module"]["resources"][i]["expressions"]
                .as_object_mut()
                .unwrap()
                .remove("preferred_subnet_id");
        }
        assert!(contained(&v, i), "{i}");
        let g = graph(&v);
        let address = v["resource_changes"][i]["address"].as_str().unwrap();
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            selected
                .nodes
                .iter()
                .filter(|n| n.address == address)
                .count(),
            1
        );
        assert!(
            selected
                .edges
                .iter()
                .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
        );
    }
}
