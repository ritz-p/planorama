use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/redshift-snapshots-plan.json");
fn verified(v: &Value, t: usize, p: &str) -> bool {
    let g = semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
#[test]
fn redshift_snapshots_keep_sources_grants_and_schedules_independent() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (1, "cluster_identifier"),
        (3, "cluster_identifier"),
        (3, "schedule_identifier"),
        (5, "kms_key_id"),
        (6, "cluster_identifier"),
        (6, "snapshot_copy_grant_name"),
        (8, "namespace_name"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 9);
    for n in &g.nodes {
        if matches!(
            n.resource_type.as_str(),
            "aws_redshift_cluster_snapshot"
                | "aws_redshiftserverless_snapshot"
                | "aws_redshift_snapshot_copy_grant"
                | "aws_redshift_snapshot_schedule"
        ) {
            assert!(n.member_owner.is_none());
        }
    }
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for kind in [
        "aws_redshift_snapshot_copy",
        "aws_redshift_snapshot_schedule_association",
    ] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_some()
        );
    }
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshift_snapshot_copy")
            .unwrap()
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region,
        Some(crate::model::RegionScope("us-east-1".into()))
    );
}
#[test]
fn redshift_snapshot_references_require_exact_current_known_scalar_provenance() {
    for (t, s, p, out) in [
        (1, 0, "cluster_identifier", "cluster_identifier"),
        (3, 2, "schedule_identifier", "identifier"),
        (5, 4, "kms_key_id", "arn"),
        (6, 5, "snapshot_copy_grant_name", "snapshot_copy_grant_name"),
        (8, 7, "namespace_name", "namespace_name"),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "source-sensitive",
            "source-mismatch",
            "output",
            "array",
            "data",
            "deleted",
            "deposed",
            "provider",
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
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                "source-mismatch" => {
                    v["resource_changes"][s]["change"]["after"][out] = json!("different")
                }
                "output" => {
                    let a = v["resource_changes"][s]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"references":[format!("{a}.unsupported")]});
                }
                "array" => {
                    v["resource_changes"][t]["change"]["after"][p] =
                        json!([v["resource_changes"][t]["change"]["after"][p]])
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "deleted" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => v["resource_changes"][s]["provider_name"] = json!("foreign/aws"),
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{p} {case}");
        }
    }
}
#[test]
fn redshift_snapshot_grants_require_destination_region_and_explicit_key() {
    for value in [json!(null), json!("us-east-1"), json!("")] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][6]["change"]["after"]["destination_region"] = value;
        assert!(!verified(&v, 6, "snapshot_copy_grant_name"));
        assert!(verified(&v, 6, "cluster_identifier"));
    }
    for mask in ["after_unknown", "after_sensitive"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][6]["change"][mask] = json!({"destination_region":true});
        assert!(!verified(&v, 6, "snapshot_copy_grant_name"));
    }
    for field in ["id", "key_id"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][5]["change"]["after"]["kms_key_id"] = json!("example");
        v["configuration"]["root_module"]["resources"][5]["expressions"]["kms_key_id"] =
            json!({"references":[format!("aws_kms_key.copy.{field}")]});
        assert!(verified(&v, 5, "kms_key_id"));
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["root_module"]["resources"][5]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("kms_key_id");
    assert!(!verified(&v, 5, "kms_key_id"));
}
#[test]
fn redshift_snapshots_filter_and_named_states_preserve_coverage() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_redshift_snapshot_copy.west".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                FIXTURE,
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
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
#[test]
fn redshift_snapshot_key_arns_can_qualify_foreign_accounts_but_ids_cannot() {
    for arn in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
        let mut other = v["configuration"]["provider_config"]["aws"].clone();
        other["alias"] = json!("foreign");
        other["expressions"]["region"] = json!({"constant_value":"us-west-2"});
        other["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
            json!("arn:aws:iam::999999999999:role/Test");
        v["configuration"]["provider_config"]["aws.foreign"] = other;
        v["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
            json!("aws.foreign");
        v["resource_changes"][4]["change"]["after"]["arn"] =
            json!("arn:aws:kms:us-west-2:999999999999:key/example");
        let field = if arn { "arn" } else { "key_id" };
        v["resource_changes"][5]["change"]["after"]["kms_key_id"] =
            v["resource_changes"][4]["change"]["after"][field].clone();
        v["configuration"]["root_module"]["resources"][5]["expressions"]["kms_key_id"] =
            json!({"references":[format!("aws_kms_key.copy.{field}")]});
        let g = semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
        let grant = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshift_snapshot_copy_grant")
            .unwrap();
        assert_eq!(
            grant.references.iter().any(|r| r.attribute == "kms_key_id"
                && r.complete
                && r.label == "Snapshot grant key"),
            arn
        );
    }
}
#[test]
fn redshift_snapshot_destination_alias_requires_verified_compatible_account() {
    for foreign in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        if foreign {
            v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"][0]["role_arn"]
                ["constant_value"] = json!("arn:aws:iam::999999999999:role/Test");
        }
        assert_eq!(verified(&v, 6, "snapshot_copy_grant_name"), !foreign);
        assert!(verified(&v, 6, "cluster_identifier"));
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let copy = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_redshift_snapshot_copy")
        .unwrap();
    assert!(
        !copy
            .references
            .iter()
            .any(|r| r.attribute == "snapshot_copy_grant_name"
                && r.label == "Destination snapshot grant"
                && r.complete)
    );
}
