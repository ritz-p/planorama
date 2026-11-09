use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/rds-auxiliary-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn explicit_auxiliary_references_preserve_lifecycle_selection_and_state_identity() {
    for (s, t, path, label) in [
        (0, 5, "resource_arn", "Activity stream cluster"),
        (2, 5, "kms_key_id", "RDS auxiliary KMS key"),
        (1, 6, "identifier", "Managed DB instance"),
        (0, 7, "db_cluster_identifier", "Shard group cluster"),
        (0, 8, "source_arn", "Integration source database"),
        (3, 8, "target_arn", "Integration target warehouse"),
        (2, 8, "kms_key_id", "RDS auxiliary KMS key"),
        (1, 9, "source_arn", "Integration source database"),
        (4, 9, "target_arn", "Integration target warehouse"),
    ] {
        let v = fixture();
        let mut pair = v.clone();
        pair["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][t]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][s],
            v["configuration"]["root_module"]["resources"][t]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
    }
}
#[test]
fn warehouse_namespace_and_shard_mode_are_not_interchangeable() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => {
                v["configuration"]["root_module"]["resources"][9]["expressions"]["target_arn"] =
                    json!({"references":["aws_redshift_cluster.target.arn"]})
            }
            2 => v["resource_changes"][3]["type"] = json!("aws_redshiftserverless_workgroup"),
            3 => {
                v["resource_changes"][0]["change"]["after"]["cluster_scalability_type"] =
                    json!("standard")
            }
            4 => {
                v["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"cluster_scalability_type":true})
            }
            5 => {
                v["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"cluster_scalability_type":true})
            }
            _ => {}
        }
        assert_eq!(complete(&v, 9, "Integration target warehouse"), case != 1);
        assert_eq!(complete(&v, 8, "Integration target warehouse"), case != 2);
        assert_eq!(complete(&v, 7, "Shard group cluster"), case < 3);
    }
}
#[test]
fn activity_stream_is_cluster_only_and_alias_identity_is_not_key_identity() {
    for case in 0..5 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][0]["type"] = json!("aws_db_instance"),
            2 => v["resource_changes"][2]["type"] = json!("aws_kms_alias"),
            3 => {
                v["resource_changes"][2]["type"] = json!("aws_kms_alias");
                v["resource_changes"][2]["change"]["after"]["name"] = json!("alias/audit");
                v["resource_changes"][5]["change"]["after"]["kms_key_id"] = json!("alias/audit");
                v["configuration"]["root_module"]["resources"][5]["expressions"]["kms_key_id"] =
                    json!({"references":["aws_kms_key.rds.name"]});
            }
            4 => v["resource_changes"][5]["change"]["after"]["kms_key_id"] = json!(["rds"]),
            _ => {}
        }
        assert_eq!(complete(&v, 5, "Activity stream cluster"), case != 1);
        assert_eq!(
            complete(&v, 5, "RDS auxiliary KMS key"),
            matches!(case, 0 | 1 | 3)
        );
        assert_eq!(
            complete(&v, 8, "RDS auxiliary KMS key"),
            !matches!(case, 2 | 3)
        );
    }
}
#[test]
fn auxiliary_scope_and_shard_ownership_do_not_invent_networks() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_rds_cluster" {
                match case {
                    1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    2 => c.account_conflict = true,
                    3 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                    4 => c.region = None,
                    _ => {}
                }
            }
        }
        let g = semantic::transform(&p);
        let shard = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_rds_shard_group")
            .unwrap();
        assert_eq!(
            shard
                .references
                .iter()
                .any(|r| r.label == "Shard group cluster" && r.complete),
            case == 0
        );
        assert_eq!(shard.member_owner.is_some(), case == 0);
        assert_eq!(g.coverage.selected.resources, 10);
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
        assert!(!out.contains("PRIVATE_"));
    }
}
