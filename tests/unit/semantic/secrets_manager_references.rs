use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/secrets-manager-references-plan.json");
fn pair(s: usize, t: usize) -> Value {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut p = v.clone();
    p["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][t]]);
    p["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][s],
        v["configuration"]["root_module"]["resources"][t]
    ]);
    p
}
fn complete(v: &Value, label: &str, accounts: bool) -> bool {
    let mut p = plan::parse(&v.to_string()).unwrap();
    if accounts {
        for n in &mut p.nodes {
            if let Some(c) = n.provider_configuration.as_mut() {
                c.account = crate::model::AwsAccountScope::new("123456789012");
            }
        }
    }
    semantic::transform(&p)
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}
#[test]
fn secret_helpers_use_exact_current_outputs_and_preserve_independent_identity() {
    for (s, t, path, label) in [
        (0, 2, "kms_key_id", "Secret encryption key"),
        (2, 5, "secret_arn", "Secret owner"),
        (2, 6, "secret_id", "Secret owner"),
        (2, 7, "secret_id", "Secret owner"),
        (2, 8, "secret_id", "Secret owner"),
        (3, 6, "rotation_lambda_arn", "Configured rotation function"),
        (
            4,
            7,
            "external_secret_rotation_role_arn",
            "Configured partner role",
        ),
    ] {
        super::super::service_references::tests::check_service_fixture(
            &pair(s, t).to_string(),
            label,
            path,
        );
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 9);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        4
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
        assert!(!out.contains("PRIVATE_SECRET_PAYLOAD"));
    }
}
#[test]
fn rotation_mode_requires_known_enabled_and_exclusive_configuration() {
    for (s, t, path, other, label) in [
        (
            3,
            6,
            "rotation_lambda_arn",
            "external_secret_rotation_role_arn",
            "Configured rotation function",
        ),
        (
            4,
            7,
            "external_secret_rotation_role_arn",
            "rotation_lambda_arn",
            "Configured partner role",
        ),
    ] {
        for case in 0..7 {
            let mut v = pair(s, t);
            match case {
                1 => v["resource_changes"][1]["change"]["after"]["rotation_enabled"] = json!(false),
                2 => {
                    v["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"rotation_enabled":true})
                }
                3 => v["resource_changes"][1]["change"]["after"][other] = json!("competing"),
                4 => v["resource_changes"][1]["change"]["after_sensitive"] = json!({other:true}),
                5 => v["resource_changes"][1]["change"]["after_sensitive"] = json!({path:true}),
                6 => {
                    v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                _ => {}
            }
            assert_eq!(complete(&v, label, false), case == 0, "{path}: {case}");
        }
    }
}
#[test]
fn replica_keys_require_correlated_regions_complete_sets_and_account_proof() {
    for case in 0..10 {
        let mut v = pair(1, 2);
        match case {
            1 => {
                v["resource_changes"][1]["change"]["after"]["replica"][0]["region"] =
                    json!("us-east-1")
            }
            2 => {
                v["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"replica":[{"region":true}]})
            }
            3 => v["resource_changes"][1]["change"]["after_sensitive"] = json!({"replica":true}),
            4 => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["replica"][0]["kms_key_id"] =
                    json!({"references":["aws_kms_replica_key.west.arn"]})
            }
            5 => v["resource_changes"][1]["change"]["after"]["replica"]
                .as_array_mut()
                .unwrap()
                .push(json!({"kms_key_id":"literal","region":"eu-west-1"})),
            6 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            7 => v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            8 => {
                v["resource_changes"][1]["change"]["after"]["replica"][0]["kms_key_id"] =
                    json!(["key-west"])
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, "Replica encryption keys", case != 9),
            case == 0,
            "case {case}"
        );
    }
}

#[test]
fn replica_rows_cannot_swap_keys_between_regions() {
    let mut v = pair(1, 2);
    let mut key = v["resource_changes"][0].clone();
    key["address"] = json!("aws_kms_key.eu");
    key["type"] = json!("aws_kms_key");
    key["change"]["after"] = json!({"key_id":"key-eu"});
    v["resource_changes"].as_array_mut().unwrap().push(key);
    v["configuration"]["provider_config"]["aws.eu"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"eu","expressions":{"region":{"constant_value":"eu-west-1"}}});
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_kms_key.eu","provider_config_key":"aws.eu","expressions":{}}));
    v["resource_changes"][1]["change"]["after"]["replica"]
        .as_array_mut()
        .unwrap()
        .push(json!({"region":"eu-west-1","kms_key_id":"key-eu"}));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["replica"].as_array_mut().unwrap().push(json!({"kms_key_id":{"references":["aws_kms_key.eu.key_id"]},"region":{"constant_value":"eu-west-1"}}));
    assert!(complete(&v, "Replica encryption keys", true));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["replica"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(!complete(&v, "Replica encryption keys", true));
}
