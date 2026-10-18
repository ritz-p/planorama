use crate::{model::Placement, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/iam-plan.json");

#[test]
fn iam_name_endpoints_require_compatible_account_evidence() {
    for case in 0..8 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["configuration"]["provider_config"]["aws.other"] = json!({
            "name":"aws", "alias":"other", "full_name":"registry.terraform.io/hashicorp/aws",
            "expressions":{"region":{"constant_value":"eu-west-1"}}
        });
        if case != 0 {
            value["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                json!("aws.other");
        }
        let mut parsed = plan::parse(&value.to_string()).unwrap();
        for node in &mut parsed.nodes {
            let is_alice = node.address == "aws_iam_user.alice";
            let binding = node.provider_configuration.as_mut().unwrap();
            if (2..=5).contains(&case) {
                binding.account = crate::model::AwsAccountScope::new(if case == 3 && is_alice {
                    "210987654321"
                } else {
                    "123456789012"
                });
            }
            if case == 4 && is_alice {
                binding.account_conflict = true;
            }
            if case == 5 && is_alice {
                binding.account = None;
            }
            if case == 6 {
                binding.account_conflict = true;
            }
            if case == 7 && is_alice {
                node.provider_configuration = None;
            }
        }
        let graph = semantic::transform(&parsed);
        for (address, attribute) in [
            ("aws_iam_group_membership.team", "users"),
            ("aws_iam_user_policy_attachment.read", "user"),
            ("aws_iam_user_policy.inline", "user"),
        ] {
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            let reference = node
                .references
                .iter()
                .find(|r| r.attribute == attribute && r.show_spatial)
                .unwrap();
            assert_eq!(
                reference.complete,
                matches!(case, 0 | 2),
                "case {case}: {address}"
            );
            assert_eq!(
                semantic::diagnostics::collect(&parsed)
                    .iter()
                    .any(|diagnostic| diagnostic.address == address
                        && diagnostic.attribute == attribute
                        && diagnostic.reason
                            == crate::model::DiagnosticReason::UnprovenEndpointValue),
                !matches!(case, 0 | 2)
            );
            if !reference.complete {
                assert!(reference.targets.is_empty());
            }
        }
    }
}

#[test]
fn iam_module_selection_keeps_individual_memberships() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    for resource in value["resource_changes"].as_array_mut().unwrap() {
        resource["address"] = json!(format!(
            "module.identity.{}",
            resource["address"].as_str().unwrap()
        ));
    }
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"] =
        json!({"module_calls":{"identity":{"module":{"resources":resources}}}});
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            modules: vec!["module.identity".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 11);
    assert_eq!(selected.coverage.selected.resources, 11);
    assert!(selected.nodes.iter().any(|n| {
        n.address == "module.identity.aws_iam_group_membership.team"
            && n.references
                .iter()
                .any(|r| r.label == "Member users" && r.complete && r.targets.len() == 2)
    }));
}

#[test]
fn iam_preserves_indexed_state_and_filtered_provenance() {
    let indexed = FIXTURE.replace("aws_iam_user.alice", "aws_iam_user.alice[0]");
    let parsed = plan::parse(&indexed).unwrap();
    assert!(
        parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "users" && r.service_reference_verified)
    );
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut second = value["resource_changes"][0].clone();
    value["resource_changes"][0]["address"] = json!("aws_iam_user.alice[0]");
    second["address"] = json!("aws_iam_user.alice[1]");
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(
        !parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "users" && r.service_reference_verified)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for parsed in plans.states.values() {
        assert_eq!(semantic::transform(parsed).coverage.selected.resources, 11);
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_iam_group_membership.team".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_iam_group_membership.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 12);
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.previous_address.as_deref() == Some("aws_iam_group_membership.old"))
    );
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.iter().all(|r| !r.complete))
    );
}

#[test]
fn iam_keeps_membership_attachment_identity_and_private_values() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), 11);
    assert_eq!(graph.coverage.selected.resources, 11);
    assert!(graph.nodes.iter().all(|n| {
        n.placement == Placement::Logical
            && n.entity
                .scope
                .as_ref()
                .is_some_and(|s| s.global && s.region.is_none())
    }));
    assert_eq!(
        parsed
            .attributes
            .iter()
            .filter(|r| r.service_reference_verified)
            .count(),
        11
    );
    let members = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_iam_group_membership.team")
        .unwrap();
    assert_eq!(
        members
            .references
            .iter()
            .find(|r| r.label == "Member users")
            .unwrap()
            .targets
            .len(),
        2
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_iam_group_policy_attachment.read")
            .unwrap()
            .action,
        crate::model::Action::Update
    );
    let mut private: Value = serde_json::from_str(FIXTURE).unwrap();
    for key in [
        "password",
        "secret",
        "access_key",
        "login_profile",
        "policy",
    ] {
        private["resource_changes"][0]["change"]["after"][key] = json!("PRIVATE_SECRET");
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &private.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(output.content.contains("Member users"));
        assert!(output.content.contains("Direct policy"));
        assert!(
            output
                .content
                .contains("aws_iam_user_policy_attachment.read")
        );
    }
}

#[test]
fn iam_requires_complete_current_explicit_unique_members() {
    for case in 0..16 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => {}
            1 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            2 => value["resource_changes"][0]["type"] = json!("aws_instance"),
            3 => value["resource_changes"][0]["mode"] = json!("data"),
            4 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            5 => {
                value["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"users":[false,true]})
            }
            6 => {
                value["resource_changes"][1]["change"]["after"]["users"] =
                    json!(["fixture-alice", "literal"])
            }
            7 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            8 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            9 => value["resource_changes"][0]["deposed"] = json!("old"),
            10 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["users"] =
                    json!({"constant_value":["fixture-alice","fixture-bob"]})
            }
            11 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["users"] =
                    json!({"references":["aws_iam_user.alice.name","var.selector"]})
            }
            12 => value["resource_changes"][2]["change"]["after"]["name"] = json!("fixture-alice"),
            13 => {
                value["resource_changes"][1]["change"]["after"]["users"] =
                    json!(["fixture-alice", "fixture-bob", "literal"])
            }
            14 => value["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws"),
            _ => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["users"] =
                    json!({"references":["aws_iam_user.alice.arn","aws_iam_user.bob.arn"]})
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert_eq!(
            parsed
                .attributes
                .iter()
                .any(
                    |r| parsed.nodes[r.target].address == "aws_iam_group_membership.team"
                        && r.attribute == "users"
                        && r.service_reference_verified
                ),
            case == 0,
            "case {case}"
        );
        assert_eq!(semantic::transform(&parsed).coverage.selected.resources, 11);
    }
}

#[test]
fn iam_scalar_relationships_validate_each_endpoint() {
    for (index, attribute) in [
        (1, "group"),
        (5, "user"),
        (5, "policy_arn"),
        (6, "group"),
        (6, "policy_arn"),
        (7, "user"),
        (7, "groups"),
        (8, "user"),
        (9, "group"),
    ] {
        for invalid in [false, true] {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            if invalid {
                value["resource_changes"][index]["change"]["after"][attribute] = json!("unproven");
            }
            let address = value["resource_changes"][index]["address"]
                .as_str()
                .unwrap();
            let parsed = plan::parse(&value.to_string()).unwrap();
            assert_eq!(
                parsed
                    .attributes
                    .iter()
                    .any(|r| parsed.nodes[r.target].address == address
                        && r.attribute == attribute
                        && r.service_reference_verified),
                !invalid,
                "{address} {attribute}"
            );
        }
    }
}
