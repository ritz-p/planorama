use crate::{model::Placement, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/serverless-references-plan.json");
fn complete(v: &Value, target: usize, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
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
                        &g.nodes
                            .iter()
                            .find(|n| n.address == v["resource_changes"][target]["address"])
                            .unwrap()
                            .resource_type,
                        path,
                    )
        })
}
#[test]
fn serverless_direct_references_preserve_independent_resources() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (target, path) in [
        (3, "collection_group_name"),
        (3, "encryption_config.kms_key_arn"),
        (4, "iam_identity_center_options.instance_arn"),
    ] {
        assert!(complete(&v, target, path), "{path}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 5);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
        if n.resource_type == "aws_opensearchserverless_collection_group" {
            assert_eq!(n.placement, Placement::Logical);
        }
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
#[test]
fn serverless_direct_references_reject_incomplete_and_incompatible_evidence() {
    for (target, source, path, output) in [
        (3, 0, "collection_group_name", "name"),
        (3, 1, "encryption_config.kms_key_arn", "arn"),
        (4, 2, "iam_identity_center_options.instance_arn", "arns"),
    ] {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "mismatch",
            "deleted",
            "deposed",
            "mode",
            "provider",
            "region",
            "account",
            "namespace",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let root = path.split('.').next().unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][root] =
                        json!({"constant_value":"literal"})
                }
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({root:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let address = v["resource_changes"][source]["address"].as_str().unwrap();
                    let expr = json!({"references":[format!("{address}.id")]});
                    v["configuration"]["root_module"]["resources"][target]["expressions"][root] =
                        if let Some((_, field)) = path.split_once('.') {
                            json!([{field:expr}])
                        } else {
                            expr
                        };
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "mode" => {
                    v["resource_changes"][source]["mode"] =
                        json!(if source == 2 { "managed" } else { "data" })
                }
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "namespace" => {
                    v["resource_changes"][source]["address"] = json!("module.other.aws_kms_key.key")
                }
                "region" | "account" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"},"allowed_account_ids":{"constant_value":["999999999999"]}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, path), "{path} {case}");
        }
    }
}
#[test]
fn serverless_discriminators_and_singleton_shapes_are_required() {
    for case in [
        "aws-owned",
        "multi-key",
        "object-key",
        "multi-instance",
        "saml",
        "unknown-type",
        "multi-options",
        "unknown-member",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "aws-owned" => {
                v["resource_changes"][3]["change"]["after"]["encryption_config"][0]["aws_owned_key"] =
                    json!(true)
            }
            "multi-key" => {
                let b = v["resource_changes"][3]["change"]["after"]["encryption_config"][0].clone();
                v["resource_changes"][3]["change"]["after"]["encryption_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(b);
            }
            "object-key" => {
                v["resource_changes"][3]["change"]["after"]["encryption_config"] =
                    v["resource_changes"][3]["change"]["after"]["encryption_config"][0].clone()
            }
            "multi-instance" => v["resource_changes"][2]["change"]["after"]["arns"]
                .as_array_mut()
                .unwrap()
                .push(json!("other")),
            "saml" => v["resource_changes"][4]["change"]["after"]["type"] = json!("saml"),
            "unknown-type" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"type":true})
            }
            "multi-options" => {
                let b = v["resource_changes"][4]["change"]["after"]["iam_identity_center_options"]
                    [0]
                .clone();
                v["resource_changes"][4]["change"]["after"]["iam_identity_center_options"]
                    .as_array_mut()
                    .unwrap()
                    .push(b);
            }
            "unknown-member" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"iam_identity_center_options":[{"instance_arn":true}]})
            }
            _ => unreachable!(),
        }
        let (target, path) = if matches!(case, "aws-owned" | "multi-key" | "object-key") {
            (3, "encryption_config.kms_key_arn")
        } else {
            (4, "iam_identity_center_options.instance_arn")
        };
        assert!(!complete(&v, target, path), "{case}");
    }
}

#[test]
fn serverless_direct_lifecycle_selection_and_named_states() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][3]["previous_address"] = json!("aws_opensearchserverless_collection.old");
    v["resource_changes"][3]["change"]["actions"] = json!(["delete", "create"]);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in &parsed.nodes {
        let matches: Vec<_> = g
            .nodes
            .iter()
            .filter(|other| other.address == n.address && other.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, n.action);
        assert_eq!(matches[0].previous_address, n.previous_address);
    }
    for case in ["delete", "deposed", "data"] {
        let mut bad = v.clone();
        match case {
            "delete" => bad["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            "deposed" => bad["resource_changes"][3]["deposed"] = json!("old"),
            _ => bad["resource_changes"][3]["mode"] = json!("data"),
        }
        assert!(!complete(&bad, 3, "collection_group_name"));
    }
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
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
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(!out.contains("PRIVATE_"));
    }
}
