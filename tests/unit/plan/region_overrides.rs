use crate::{coverage::Coverage, model::ProviderIdentity, plan, semantic};
use serde_json::{Value, json};

fn fixtures() -> [Value; 3] {
    [
        include_str!("../../fixtures/aws-region-overrides/plan.json"),
        include_str!("../../fixtures/compatibility/1.16.6/overrides/plan.json"),
        include_str!("../../fixtures/compatibility/1.9.8/overrides670/plan.json"),
    ]
    .map(|input| serde_json::from_str(input).unwrap())
}

#[test]
fn region_core_allowlist_has_real_plan_evidence_for_every_entry() {
    let captured =
        fixtures().map(|fixture| fixture["terraform_version"].as_str().unwrap().to_owned());
    assert_eq!(super::REGION_OVERRIDE_CORE_VERSIONS, &captured[..2]);
}

#[test]
fn captured_overrides_preserve_provider_identity_and_independent_account_evidence() {
    for fixture in fixtures() {
        let raw = plan::parse(&fixture.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        for node in &raw.nodes {
            let config = node.provider_configuration.as_ref().unwrap();
            assert_eq!(
                node.provider,
                ProviderIdentity::from_source("hashicorp/aws")
            );
            assert_eq!(config.key, "aws");
            assert_eq!(config.region.as_ref().unwrap().0, "us-east-1");
            assert!(config.account.is_none());
            let scope = crate::provider::scope(node).unwrap();
            assert_eq!(
                scope.region.unwrap().0,
                if node.address.ends_with("default") {
                    "us-east-1"
                } else {
                    "eu-west-3"
                }
            );
            assert!(scope.account.is_none());
        }
        let coverage = Coverage::collect(&raw, &graph);
        assert_eq!(coverage.input.region_applicable, 4);
        assert_eq!(coverage.input.unknown_region, 0);
        assert_eq!(coverage.input.unknown_account, 4);
    }
}

#[test]
fn verified_current_override_evidence_survives_update_noop_and_replacement() {
    for fixture in fixtures() {
        for actions in [
            json!(["update"]),
            json!(["no-op"]),
            json!(["delete", "create"]),
        ] {
            let mut value = fixture.clone();
            value["resource_changes"][0]["change"]["actions"] = actions;
            let raw = plan::parse(&value.to_string()).unwrap();
            let node = raw
                .nodes
                .iter()
                .find(|node| node.address == "aws_cloudwatch_log_group.paris")
                .unwrap();
            assert_eq!(node.region_override.as_ref().unwrap().0, "eu-west-3");
            assert!(crate::provider::scope(node).unwrap().account.is_none());
        }
    }
}

#[test]
fn overrides_reject_unverified_dynamic_ambiguous_and_historical_evidence() {
    for fixture in fixtures() {
        for case in 0..31 {
            let mut value = fixture.clone();
            match case {
                0 => {
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        json!("5.100.0")
                }
                1 => {
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        json!(">= 6.0.0")
                }
                2 => {
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        Value::Null
                }
                3 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                4 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"]
                        ["references"] = json!([])
                }
                5 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        Value::Null
                }
                6 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        json!({"constant_value":"not-a-region"})
                }
                7 => value["resource_changes"][0]["change"]["after"]["region"] = json!("us-east-1"),
                8 => {
                    value["resource_changes"][0]["change"]["after_unknown"]["region"] = json!(true)
                }
                9 => value["resource_changes"][0]["provider_name"] = json!("acme/aws"),
                10 => {
                    value["configuration"]["provider_config"]["aws"]["full_name"] =
                        json!("acme/aws")
                }
                11 => value["resource_changes"][0]["mode"] = json!("data"),
                12 => value["resource_changes"][0]["deposed"] = json!("old"),
                13 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                14 => {
                    value["resource_changes"][0]["change"]["after_sensitive"]["region"] =
                        json!(true)
                }
                15 => value["terraform_version"] = json!("1.8.0"),
                16 => {
                    let duplicate = value["resource_changes"][0].clone();
                    value["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                }
                17 => value["resource_changes"][0]["type"] = json!("aws_s3_bucket"),
                18 => {
                    value["configuration"]["root_module"]["resources"][0]["type"] = json!("aws_vpc")
                }
                19 => value["resource_changes"][0]["change"]["after"]["region"] = Value::Null,
                20 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
                21 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        json!({"constant_value":42})
                }
                22 => {
                    value["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        json!({"constant_value":""})
                }
                23 => value["terraform_version"] = json!("1.16.5"),
                24 => value["terraform_version"] = json!("1.16.7"),
                25 => value["terraform_version"] = json!("1.16.6-rc1"),
                26 => value["terraform_version"] = Value::Null,
                27 => {
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        json!("6.0.1")
                }
                28 => {
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        json!("~> 6.0")
                }
                29 => value["resource_changes"][0]["change"]["after_sensitive"] = json!(true),
                _ => {
                    value["terraform_version"] = json!("1.16.6");
                    value["configuration"]["provider_config"]["aws"]["version_constraint"] =
                        json!("6.70.0");
                }
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let node = raw
                .nodes
                .iter()
                .find(|n| {
                    n.address == "aws_cloudwatch_log_group.paris"
                        && (case != 12 || n.deposed_key.is_some())
                })
                .unwrap();
            assert!(node.region_override.is_none(), "case {case}");
            assert!(
                crate::provider::scope(node).is_none_or(|s| s.region.is_none()),
                "case {case}"
            );
            if !matches!(case, 9 | 11 | 12) {
                assert!(
                    Coverage::collect(&raw, &semantic::transform(&raw))
                        .input
                        .unknown_region
                        > 0,
                    "case {case}"
                );
            }
        }
    }
}

#[test]
fn global_and_non_override_attributes_never_acquire_override_scopes() {
    for fixture in fixtures() {
        for (kind, mode, global) in [
            ("aws_iam_role", "managed", true),
            ("aws_region", "data", false),
            ("aws_vpc_endpoint_service", "data", false),
        ] {
            let mut value = fixture.clone();
            value["resource_changes"][0]["type"] = json!(kind);
            value["resource_changes"][0]["mode"] = json!(mode);
            value["configuration"]["root_module"]["resources"][0]["type"] = json!(kind);
            value["configuration"]["root_module"]["resources"][0]["mode"] = json!(mode);
            let raw = plan::parse(&value.to_string()).unwrap();
            let node = raw
                .nodes
                .iter()
                .find(|n| n.address == "aws_cloudwatch_log_group.paris")
                .unwrap();
            let scope = crate::provider::scope(node);
            assert_eq!(scope.as_ref().is_some_and(|s| s.global), global);
            assert!(scope.is_none_or(|s| s.region.is_none()));
        }
    }
}

#[test]
fn ambiguous_configuration_and_deletion_without_override_block_fallback() {
    for fixture in fixtures() {
        for duplicate in [false, true] {
            let mut value = fixture.clone();
            if duplicate {
                let resource = value["configuration"]["root_module"]["resources"][0].clone();
                value["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(resource);
            } else {
                value["configuration"]["root_module"]["resources"][0]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("region");
                value["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
                value["resource_changes"][0]["change"]["before"] = json!({"region":"eu-west-3"});
                value["resource_changes"][0]["change"]["after"] = Value::Null;
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let node = raw
                .nodes
                .iter()
                .find(|n| n.address == "aws_cloudwatch_log_group.paris")
                .unwrap();
            assert!(crate::provider::scope(node).is_none());
            assert_eq!(
                node.provider_configuration
                    .as_ref()
                    .unwrap()
                    .region
                    .as_ref()
                    .unwrap()
                    .0,
                "us-east-1"
            );
        }
    }
}
