use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/redshift-cluster-plan.json");
fn verified(v: &Value, target: usize, path: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&raw);
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        path,
                    )
        })
}
#[test]
fn redshift_cluster_extensions_verify_exact_outputs_and_keep_shared_resources() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (6, "iam_roles"),
        (6, "default_iam_role_arn"),
        (6, "kms_key_id"),
        (6, "master_password_secret_kms_key_id"),
        (6, "snapshot_identifier"),
        (10, "cluster_identifier"),
        (10, "iam_role_arns"),
        (10, "default_iam_role_arn"),
    ] {
        assert!(verified(&v, t, p), "{p}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 11);
    assert!(
        g.nodes
            .iter()
            .find(|n| n.address == "aws_redshift_cluster_iam_roles.roles")
            .unwrap()
            .member_owner
            .is_some()
    );
    for kind in [
        "aws_iam_role",
        "aws_kms_key",
        "aws_redshift_cluster_snapshot",
    ] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_none()
        );
    }
}
#[test]
fn redshift_cluster_extensions_reject_unknown_sensitive_partial_and_foreign_proof() {
    for (t, s, p, out) in [
        (6, 7, "iam_roles", "arn"),
        (6, 7, "default_iam_role_arn", "arn"),
        (6, 8, "kms_key_id", "arn"),
        (6, 8, "master_password_secret_kms_key_id", "key_id"),
        (6, 9, "snapshot_identifier", "snapshot_identifier"),
        (10, 6, "cluster_identifier", "cluster_identifier"),
        (10, 7, "iam_role_arns", "arn"),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "source-unknown",
            "source-sensitive",
            "mismatch",
            "wrong-output",
            "data",
            "delete",
            "deposed",
            "provider",
            "region",
            "scalar",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"constant_value":"literal"})
                }
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({p:true})
                }
                "source-unknown" | "source-sensitive" => {
                    v["resource_changes"][s]["change"][if case == "source-unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({out:true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"references":[format!("{a}.id")]});
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => v["resource_changes"][s]["provider_name"] = json!("other/aws"),
                "region" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("other");
                }
                "scalar" => {
                    let value = v["resource_changes"][t]["change"]["after"][p].clone();
                    v["resource_changes"][t]["change"]["after"][p] = if value.is_array() {
                        value[0].clone()
                    } else {
                        json!([value])
                    };
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{p} {case}");
        }
    }
}
#[test]
fn redshift_cluster_modes_and_restore_qualifiers_are_conservative() {
    for (field, value, path) in [
        ("encrypted", json!(true), "kms_key_id"),
        ("encrypted", json!("false"), "kms_key_id"),
        (
            "manage_master_password",
            json!(false),
            "master_password_secret_kms_key_id",
        ),
        (
            "snapshot_cluster_identifier",
            json!("different"),
            "snapshot_identifier",
        ),
        (
            "owner_account",
            json!("999999999999"),
            "snapshot_identifier",
        ),
        ("snapshot_arn", json!("other"), "snapshot_identifier"),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][6]["change"]["after"][field] = value;
        assert!(!verified(&v, 6, path), "{field}");
    }
}
#[test]
fn redshift_cluster_network_requires_security_groups_in_subnet_vpc() {
    for case in ["known", "unknown", "partial", "foreign", "literal"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"vpc_security_group_ids":true})
            }
            "partial" => {
                v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"] =
                    json!(["sg-fixture", "other"])
            }
            "foreign" => v["resource_changes"][5]["change"]["after"]["vpc_id"] = json!("other"),
            "literal" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["vpc_security_group_ids"] =
                    json!({"constant_value":["sg-fixture"]})
            }
            _ => (),
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let target = g
            .nodes
            .iter()
            .position(|n| n.address == "aws_redshift_cluster.main")
            .unwrap();
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.to == target && e.kind == EdgeKind::Containment),
            case == "known",
            "{case}"
        );
    }
}
#[test]
fn redshift_cluster_restore_owner_qualifies_source_without_relocating_cluster() {
    for allowed in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
        let mut other = v["configuration"]["provider_config"]["aws"].clone();
        other["alias"] = json!("source");
        other["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
            json!("arn:aws:iam::999999999999:role/Test");
        v["configuration"]["provider_config"]["aws.source"] = other;
        v["configuration"]["root_module"]["resources"][9]["provider_config_key"] =
            json!("aws.source");
        v["resource_changes"][9]["change"]["after"]["owner_account"] = json!("999999999999");
        if allowed {
            v["resource_changes"][6]["change"]["after"]["owner_account"] = json!("999999999999");
        }
        let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let g = semantic::transform(&parsed);
        let cluster = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_redshift_cluster.main")
            .unwrap();
        assert_eq!(
            cluster
                .references
                .iter()
                .any(|r| r.attribute == "snapshot_identifier"
                    && r.label == "Restore snapshot"
                    && r.complete),
            allowed
        );
        assert_eq!(
            cluster.entity.scope.as_ref().unwrap().account,
            crate::model::AwsAccountScope::new("123456789012")
        );
    }
}
#[test]
fn redshift_cluster_restore_arn_collections_and_selection_preserve_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][6]["change"]["after"]["snapshot_identifier"] = Value::Null;
    v["resource_changes"][6]["change"]["after"]["snapshot_arn"] =
        v["resource_changes"][9]["change"]["after"]["arn"].clone();
    v["configuration"]["root_module"]["resources"][6]["expressions"]["snapshot_arn"] =
        json!({"references":["aws_redshift_cluster_snapshot.history.arn"]});
    assert!(verified(&v, 6, "snapshot_arn"));
    for path in ["iam_roles", "iam_role_arns"] {
        let t = if path == "iam_roles" { 6 } else { 10 };
        let mut p = v.clone();
        p["resource_changes"][t]["change"]["after"][path]
            .as_array_mut()
            .unwrap()
            .push(json!("extra"));
        assert!(!verified(&p, t, path));
    }
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_redshift_cluster.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                &v.to_string(),
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: v.to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
#[test]
fn redshift_cluster_filtered_subnets_remove_indirect_placement() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][5]["change"]["actions"] = json!(["update"]);
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    let target = selected
        .nodes
        .iter()
        .position(|n| n.address == "aws_redshift_cluster.main")
        .unwrap();
    assert!(
        !selected
            .edges
            .iter()
            .any(|e| e.to == target && e.kind == EdgeKind::Containment)
    );
}
