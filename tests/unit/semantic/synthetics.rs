use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/synthetics-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
fn attached(v: &Value) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.edges.iter().any(|e| {
        e.kind == crate::model::EdgeKind::Containment
            && g.nodes[e.to].resource_type == "aws_synthetics_canary"
    })
}
#[test]
fn canary_scalar_contracts_preserve_identity_lifecycle_filter_and_states() {
    for (s, t, path, label) in [
        (4, 8, "execution_role_arn", "Canary execution role"),
        (5, 8, "s3_bucket", "Canary script bucket"),
        (6, 8, "kms_key_arn", "Canary environment key"),
        (9, 10, "group_name", "Synthetics group"),
        (8, 10, "canary_arn", "Associated canary"),
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
fn artifact_key_is_distinct_mode_gated_and_exactly_nested() {
    for case in 0..9 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][8]["change"]["after"]["artifact_config"][0]["s3_encryption"]
                    [0]["encryption_mode"] = json!("SSE_S3")
            }
            2 => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"artifact_config":[{"s3_encryption":[{"encryption_mode":true}]}]})
            }
            3 => {
                v["resource_changes"][8]["change"]["after_sensitive"] =
                    json!({"artifact_config":true})
            }
            4 => {
                v["resource_changes"][8]["change"]["after"]["artifact_config"] =
                    v["resource_changes"][8]["change"]["after"]["artifact_config"][0].clone()
            }
            5 => v["resource_changes"][8]["change"]["after"]["artifact_config"][0]["s3_encryption"] =
                v["resource_changes"][8]["change"]["after"]["artifact_config"][0]["s3_encryption"]
                    [0]
                .clone(),
            6 => v["resource_changes"][7]["type"] = json!("aws_kms_alias"),
            7 => v["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
            8 => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["artifact_config"]
                    [0]["s3_encryption"][0]["kms_key_arn"] =
                    json!({"references":["aws_kms_key.environment.arn"]})
            }
            _ => {}
        }
        assert_eq!(complete(&v, 8, "Canary artifact key"), case == 0, "{case}");
        assert!(complete(&v, 8, "Canary environment key"));
    }
}
#[test]
fn canary_containment_requires_complete_configured_subnets_groups_and_common_vpc() {
    assert!(attached(&fixture()));
    for case in 0..9 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][8]["change"]["after"]["vpc_config"][0]["subnet_ids"] =
                    json!(["subnet-a", "missing"])
            }
            1 => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"vpc_config":[{"security_group_ids":true}]})
            }
            2 => v["resource_changes"][8]["change"]["after_sensitive"] = json!({"vpc_config":true}),
            3 => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["vpc_config"] =
                    json!([])
            }
            4 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            5 => v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other"),
            6 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            7 => {
                v["resource_changes"][8]["change"]["after"]["vpc_config"][0]["security_group_ids"] =
                    json!([])
            }
            8 => {
                v["resource_changes"][8]["change"]["after"]["vpc_config"] =
                    v["resource_changes"][8]["change"]["after"]["vpc_config"][0].clone()
            }
            _ => unreachable!(),
        }
        assert!(!attached(&v), "{case}");
    }
}
#[test]
fn canary_network_rechecks_account_and_region_at_presentation() {
    for case in 0..4 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            n.provider_configuration.as_mut().unwrap().account =
                crate::model::AwsAccountScope::new("111111111111");
        }
        let c = p
            .nodes
            .iter_mut()
            .find(|n| n.resource_type == "aws_security_group")
            .unwrap()
            .provider_configuration
            .as_mut()
            .unwrap();
        match case {
            1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
            2 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
            3 => c.account_conflict = true,
            _ => {}
        }
        let g = semantic::transform(&p);
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].resource_type == "aws_synthetics_canary"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn canary_is_independent_spatial_workload_without_engine_or_payload_synthesis() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 11);
    let canary = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_synthetics_canary")
        .unwrap();
    assert_eq!(canary.placement, crate::model::Placement::Spatial);
    assert!(canary.member_owner.is_none());
    assert!(
        !g.nodes
            .iter()
            .any(|n| n.resource_type == "aws_lambda_function")
    );
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
            .contains("PRIVATE_SYNTHETICS_PAYLOAD")
        );
    }
}

#[test]
fn canary_does_not_span_distinct_explicit_vpc_parents() {
    let mut v = fixture();
    let mut other = v["resource_changes"][0].clone();
    other["address"] = json!("aws_vpc.other");
    other["name"] = json!("other");
    other["change"]["after"]["id"] = json!("vpc-other");
    v["resource_changes"].as_array_mut().unwrap().push(other);
    let mut config = v["configuration"]["root_module"]["resources"][0].clone();
    config["address"] = json!("aws_vpc.other");
    config["name"] = json!("other");
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
    v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
        json!({"references":["aws_vpc.other.id"]});
    assert!(!attached(&v));
    assert!(complete(&v, 8, "Canary subnets"));
    assert!(complete(&v, 8, "Canary security groups"));
}
