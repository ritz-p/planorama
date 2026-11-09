use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/kms-references-plan.json");

fn pair(source: usize, target: usize) -> Value {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut pair = v.clone();
    pair["resource_changes"] =
        json!([v["resource_changes"][source], v["resource_changes"][target]]);
    pair["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][source],
        v["configuration"]["root_module"]["resources"][target]
    ]);
    pair
}
fn complete(v: &Value, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}

#[test]
fn key_variants_and_configured_helpers_use_only_exact_current_outputs() {
    for (s, t, path, label) in [
        (1, 4, "target_key_id", "Alias key"),
        (2, 5, "target_key_id", "Alias key"),
        (3, 6, "target_key_id", "Alias key"),
        (0, 7, "key_id", "Configured KMS key"),
        (0, 10, "key_id", "Configured KMS key"),
        (0, 11, "key_id", "Configured KMS key"),
        (8, 10, "grantee_principal", "Grantee principal"),
        (9, 10, "retiring_principal", "Retiring principal"),
        (12, 13, "cloud_hsm_cluster_id", "CloudHSM cluster"),
        (
            14,
            15,
            "xks_proxy_vpc_endpoint_service_name",
            "External key proxy service",
        ),
        (13, 16, "custom_key_store_id", "Custom key store"),
    ] {
        super::super::service_references::tests::check_service_fixture(
            &pair(s, t).to_string(),
            label,
            path,
        );
    }
    for key_type in [
        "aws_kms_key",
        "aws_kms_external_key",
        "aws_kms_replica_key",
        "aws_kms_replica_external_key",
    ] {
        for output in ["id", "key_id", "arn"] {
            let mut v = pair(0, 11);
            v["resource_changes"][0]["type"] = json!(key_type);
            v["resource_changes"][0]["change"]["after"][output] = json!("key-main");
            v["configuration"]["root_module"]["resources"][1]["expressions"]["key_id"] =
                json!({"references":[format!("aws_kms_key.main.{output}")]});
            assert_eq!(
                complete(&v, "Configured KMS key"),
                key_type != "aws_kms_external_key" || output != "key_id",
                "{key_type}.{output}"
            );
        }
    }
}

#[test]
fn replicas_require_correct_primary_kind_known_multi_region_and_distinct_regions() {
    for (s, t) in [(0, 2), (1, 3)] {
        for case in 0..15 {
            let mut v = pair(s, t);
            match case {
                1 => v["resource_changes"][0]["change"]["after"]["multi_region"] = json!(false),
                2 => {
                    v["resource_changes"][0]["change"]["after_unknown"] =
                        json!({"multi_region":true})
                }
                3 => {
                    v["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"multi_region":true})
                }
                4 => {
                    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
                        json!({"constant_value":"us-east-1"});
                }
                5 => {
                    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws")
                }
                6 => {
                    v["resource_changes"][0]["type"] = json!(if s == 0 {
                        "aws_kms_external_key"
                    } else {
                        "aws_kms_key"
                    })
                }
                7 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                8 => v["resource_changes"][0]["deposed"] = json!("old"),
                9 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true}),
                10 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"]["primary_key_arn"] =
                        json!({"constant_value":"literal"})
                }
                11 => {
                    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                _ => {}
            }
            let mut parsed = plan::parse(&v.to_string()).unwrap();
            for n in &mut parsed.nodes {
                if let Some(config) = n.provider_configuration.as_mut() {
                    config.account = crate::model::AwsAccountScope::new("123456789012");
                }
            }
            if case == 12 {
                parsed.nodes[1]
                    .provider_configuration
                    .as_mut()
                    .unwrap()
                    .account = crate::model::AwsAccountScope::new("999999999999");
            }
            if case == 13 {
                parsed.nodes[1]
                    .provider_configuration
                    .as_mut()
                    .unwrap()
                    .account_conflict = true;
            }
            if case == 14 {
                for n in &mut parsed.nodes {
                    n.provider_configuration.as_mut().unwrap().account = None;
                }
            }
            assert_eq!(
                semantic::transform(&parsed)
                    .nodes
                    .iter()
                    .flat_map(|n| &n.references)
                    .any(|r| r.label == "Primary KMS key" && r.complete),
                case == 0,
                "{s}: case {case}"
            );
        }
    }
}

#[test]
fn custom_store_modes_and_sensitive_evidence_are_conservative() {
    for (s, t, label, field) in [
        (12, 13, "CloudHSM cluster", "custom_key_store_type"),
        (
            14,
            15,
            "External key proxy service",
            "xks_proxy_connectivity",
        ),
    ] {
        for case in 0..4 {
            let mut v = pair(s, t);
            match case {
                1 => v["resource_changes"][1]["change"]["after"][field] = json!("PUBLIC_ENDPOINT"),
                2 => v["resource_changes"][1]["change"]["after_unknown"] = json!({field:true}),
                3 => v["resource_changes"][1]["change"]["after_sensitive"] = json!({field:true}),
                _ => {}
            }
            assert_eq!(complete(&v, label), case == 0);
        }
    }
    let mut v = pair(8, 10);
    v["resource_changes"][0]["type"] = json!("aws_iam_group");
    assert!(!complete(&v, "Grantee principal"));
}

#[test]
fn key_owners_retain_actions_scope_coverage_and_payload_privacy() {
    let p = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 17);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        5
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_kms_replica_"))
            .all(|n| n.member_owner.is_none() && n.placement == crate::model::Placement::Spatial)
    );
    let mut changed = p.clone();
    for n in &mut changed.nodes {
        if n.resource_type == "aws_iam_role" {
            n.provider_configuration.as_mut().unwrap().account_conflict = true;
        }
    }
    assert!(
        !semantic::transform(&changed)
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Grantee principal" && r.complete)
    );
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
        assert!(!out.contains("PRIVATE_KMS_PAYLOAD"));
    }
}
