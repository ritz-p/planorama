use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/iam-policies-plan.json");

#[test]
fn global_aliases_require_known_accounts_and_filtering_preserves_helpers() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"eu-west-1"}}});
    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws.other");
    for case in ["unknown", "matching", "conflict"] {
        let mut p = plan::parse(&v.to_string()).unwrap();
        for n in &mut p.nodes {
            n.provider_configuration.as_mut().unwrap().account = if case == "unknown" {
                None
            } else {
                crate::model::AwsAccountScope::new(
                    if case == "conflict" && n.resource_type == "aws_iam_policy" {
                        "210987654321"
                    } else {
                        "123456789012"
                    },
                )
            };
        }
        let raw = semantic::Input::new(&p);
        for r in raw.attributes.iter().filter(|r| {
            matches!(
                r.attribute.as_str(),
                "policy_arn" | "policy_arns" | "permissions_boundary" | "managed_policy_arns"
            )
        }) {
            assert_eq!(
                super::super::service_references::verified(&raw, r),
                case == "matching",
                "{case}"
            );
        }
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_iam_role_policy")
        .unwrap()
        .action = crate::model::Action::Update;
    let graph = semantic::transform(&p);
    let view = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_iam_role_policy.inline".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(view.coverage.selected.resources, 1);
    assert_eq!(view.nodes[0].resource_type, "aws_iam_role_policy");
}

#[test]
fn policy_references_require_current_exact_complete_evidence() {
    let value: Value = serde_json::from_str(FIXTURE).unwrap();
    let p = plan::parse(FIXTURE).unwrap();
    let raw = semantic::Input::new(&p);
    let refs: Vec<_> = raw
        .attributes
        .iter()
        .filter(|r| super::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some())
        .collect();
    assert_eq!(refs.len(), 23);
    for r in refs {
        let target = value["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.target].address)
            .unwrap();
        let source = value["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let field = r.source_attributes.first().unwrap();
        for case in [
            "valid",
            "literal",
            "unknown",
            "sensitive",
            "endpoint-sensitive",
            "wrong-value",
            "wrong-output",
            "incomplete",
            "deleted",
            "data",
            "deposed",
            "foreign",
        ] {
            let mut v = value.clone();
            let path = r.attribute.as_str();
            match case {
                "literal" => v["configuration"]["root_module"]["resources"][target]["expressions"][path]=json!({}),
                "unknown" => v["resource_changes"][target]["change"]["after_unknown"]=json!({path:true}),
                "sensitive" => v["resource_changes"][target]["change"]["after_sensitive"]=json!({path:true}),
                "endpoint-sensitive" => v["resource_changes"][source]["change"]["after_sensitive"]=json!({field:true}),
                "wrong-value" => v["resource_changes"][source]["change"]["after"][field]=json!("other"),
                "wrong-output" => { let address=v["resource_changes"][source]["address"].as_str().unwrap(); v["configuration"]["root_module"]["resources"][target]["expressions"][path]=json!({"references":[format!("{address}.id")]}); }
                "incomplete" => v["configuration"]["root_module"]["resources"][target]["expressions"][path]["references"].as_array_mut().unwrap().push(json!("aws_iam_policy.missing.arn")),
                "deleted" => v["resource_changes"][source]["change"]["actions"]=json!(["delete"]),
                "data" => v["resource_changes"][source]["mode"]=json!("data"),
                "deposed" => v["resource_changes"][source]["deposed"]=json!("old"),
                "foreign" => v["resource_changes"][source]["provider_name"]=json!("example.com/foreign/aws"),
                _=>{}
            }
            let p = plan::parse(&v.to_string()).unwrap();
            let raw = semantic::Input::new(&p);
            assert_eq!(
                raw.attributes.iter().any(|a| a.attribute == path
                    && raw.nodes[a.target].address
                        == v["resource_changes"][target]["address"].as_str().unwrap()
                    && super::super::service_references::verified(&raw, a)),
                case == "valid",
                "{} {path} {case}",
                v["resource_changes"][target]["address"]
            );
        }
    }
}

#[test]
fn exclusive_inline_policy_identity_includes_owner_and_empty_sets_keep_controller() {
    for principal in ["group", "user", "role"] {
        for case in ["valid", "wrong-owner", "literal-owner", "empty"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let resources = v["resource_changes"].as_array().unwrap();
            let controller = resources
                .iter()
                .position(|r| r["type"] == format!("aws_iam_{principal}_policies_exclusive"))
                .unwrap();
            let policy = resources
                .iter()
                .position(|r| r["type"] == format!("aws_iam_{principal}_policy"))
                .unwrap();
            match case {
                "wrong-owner" => {
                    v["resource_changes"][policy]["change"]["after"][principal] = json!("other")
                }
                "literal-owner" => {
                    v["configuration"]["root_module"]["resources"][policy]["expressions"] =
                        json!({})
                }
                "empty" => {
                    v["resource_changes"][controller]["change"]["after"]["policy_names"] =
                        json!([]);
                    v["configuration"]["root_module"]["resources"][controller]["expressions"]["policy_names"] =
                        json!({"constant_value":[]});
                }
                _ => {}
            }
            let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let n = g
                .nodes
                .iter()
                .find(|n| {
                    n.address
                        == v["resource_changes"][controller]["address"]
                            .as_str()
                            .unwrap()
                })
                .unwrap();
            assert!(n.member_owner.is_some(), "{principal} {case}");
            assert_eq!(
                n.references
                    .iter()
                    .any(|r| r.label == "Exclusive inline policies" && r.complete),
                case == "valid",
                "{principal} {case}"
            );
            assert_eq!(g.coverage.selected.resources, 16);
        }
    }
}

#[test]
fn iam_global_references_recheck_account_and_preserve_independent_policies() {
    for conflict in [false, true] {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            n.provider_configuration.as_mut().unwrap().account = crate::model::AwsAccountScope::new(
                if conflict && n.resource_type == "aws_iam_policy" {
                    "210987654321"
                } else {
                    "123456789012"
                },
            );
        }
        let raw = semantic::Input::new(&p);
        for r in raw.attributes.iter().filter(|r| {
            matches!(
                r.attribute.as_str(),
                "policy_arn" | "policy_arns" | "permissions_boundary" | "managed_policy_arns"
            )
        }) {
            assert_eq!(
                super::super::service_references::verified(&raw, r),
                !conflict
            );
        }
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(
        g.nodes
            .iter()
            .all(|n| n.placement == crate::model::Placement::Logical),
        "{:?}",
        g.nodes
            .iter()
            .map(|n| (&n.resource_type, n.placement))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        7
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_iam_policy"
                    | "aws_iam_instance_profile"
                    | "aws_iam_policy_attachment"
                    | "aws_iam_role_policy_attachment"
            ))
            .all(|n| n.member_owner.is_none())
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
        assert!(out.contains("Exclusive inline policies"));
        assert!(!out.contains("OPAQUE_POLICY_PAYLOAD"));
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
