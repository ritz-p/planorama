use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/iam-credentials-plan.json");

#[test]
fn all_credential_helpers_require_exact_current_user_name_provenance() {
    let value: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in 1..6 {
        let path =
            super::owner(value["resource_changes"][index]["type"].as_str().unwrap()).unwrap();
        let mut pair = value.clone();
        pair["resource_changes"] = json!([
            value["resource_changes"][0],
            value["resource_changes"][index]
        ]);
        pair["configuration"]["root_module"]["resources"] = json!([
            value["configuration"]["root_module"]["resources"][0],
            value["configuration"]["root_module"]["resources"][index]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            "Credential owner",
            path,
        );
        for case in [
            "valid",
            "wrong-output",
            "sensitive",
            "endpoint-sensitive",
            "missing-binding",
            "account-conflict",
            "different-account",
            "same-account-alias",
            "unknown-account-alias",
        ] {
            let mut v = pair.clone();
            match case {
                "wrong-output" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"][path] =
                        json!({"references":["aws_iam_user.main.id"]})
                }
                "sensitive" => {
                    v["resource_changes"][1]["change"]["after_sensitive"] = json!({path:true})
                }
                "endpoint-sensitive" => {
                    v["resource_changes"][0]["change"]["after_sensitive"] = json!({"name":true})
                }
                _ => {}
            }
            let mut p = plan::parse(&v.to_string()).unwrap();
            for n in &mut p.nodes {
                let binding = n.provider_configuration.as_mut().unwrap();
                if case.ends_with("alias") {
                    binding.account = if case.starts_with("unknown") {
                        None
                    } else {
                        crate::model::AwsAccountScope::new("123456789012")
                    };
                }
                if n.resource_type == "aws_iam_user" {
                    match case {
                        "account-conflict" => binding.account_conflict = true,
                        "different-account" => {
                            binding.account = crate::model::AwsAccountScope::new("210987654321")
                        }
                        "same-account-alias" | "unknown-account-alias" => {
                            binding.key = "aws.other".into()
                        }
                        "missing-binding" => n.provider_configuration = None,
                        _ => {}
                    }
                }
            }
            let g = semantic::transform(&p);
            let helper = g
                .nodes
                .iter()
                .find(|n| n.resource_type != "aws_iam_user")
                .unwrap();
            assert_eq!(
                helper.member_owner.is_some(),
                matches!(case, "valid" | "same-account-alias"),
                "{path} {case}"
            );
            assert_eq!(g.coverage.selected.resources, 2);
        }
    }
}

#[test]
fn credentials_keep_independent_actions_without_displaying_payloads() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][1]["change"]["actions"] = json!(["delete", "create"]);
    v["resource_changes"][2]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][3]["change"]["actions"] = json!(["delete"]);
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 6);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        4
    );
    assert!(
        g.nodes
            .iter()
            .all(|n| n.placement == crate::model::Placement::Logical)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            &v.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("Credential owner"));
        assert!(!out.contains("PRIVATE_CREDENTIAL_PAYLOAD"));
    }
}
