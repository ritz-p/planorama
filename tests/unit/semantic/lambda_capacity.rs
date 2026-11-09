use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/lambda-capacity-plan.json");

#[test]
fn capacity_network_requires_complete_sets_and_explicit_common_vpc() {
    for case in [
        "valid",
        "literal-member",
        "unknown",
        "sensitive",
        "wrong-vpc",
        "literal-vpc",
        "shape",
        "duplicate-block",
        "missing-groups",
        "deleted",
        "data",
        "deposed",
        "foreign",
        "wrong-output",
        "different-region",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "literal-member" => {
                v["resource_changes"][4]["change"]["after"]["vpc_config"][0]["subnet_ids"] =
                    json!(["subnet-a", "literal"])
            }
            "unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"vpc_config":true})
            }
            "sensitive" => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"vpc_config":[{"subnet_ids":[false,true]}]})
            }
            "wrong-vpc" => v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other"),
            "literal-vpc" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            "shape" => {
                v["resource_changes"][4]["change"]["after"]["vpc_config"] =
                    v["resource_changes"][4]["change"]["after"]["vpc_config"][0].clone()
            }
            "duplicate-block" => {
                let block = v["resource_changes"][4]["change"]["after"]["vpc_config"][0].clone();
                v["resource_changes"][4]["change"]["after"]["vpc_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            }
            "missing-groups" => {
                v["resource_changes"][4]["change"]["after"]["vpc_config"][0]["security_group_ids"] =
                    json!([])
            }
            "deleted" => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            "data" => v["resource_changes"][1]["mode"] = json!("data"),
            "deposed" => v["resource_changes"][1]["deposed"] = json!("old"),
            "foreign" => {
                v["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws")
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["vpc_config"][0]
                    ["subnet_ids"] = json!({"references":["aws_subnet.a.arn", "aws_subnet.b.arn"]})
            }
            "different-region" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 7);
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].resource_type == "aws_lambda_capacity_provider"),
            case == "valid",
            "{case}"
        );
        assert_eq!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == "aws_lambda_capacity_provider")
                .unwrap()
                .placement,
            crate::model::Placement::Spatial
        );
        assert!(g.nodes.iter().all(|n| !matches!(
            n.resource_type.as_str(),
            "aws_instance" | "aws_network_interface"
        )));
    }
}

#[test]
fn operator_role_requires_exact_singleton_permissions_block_and_current_arn() {
    for case in [
        "valid",
        "shape",
        "unknown",
        "sensitive",
        "wrong-output",
        "mismatch",
        "deleted",
        "data",
        "literal",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "shape" => {
                v["resource_changes"][4]["change"]["after"]["permissions_config"] =
                    v["resource_changes"][4]["change"]["after"]["permissions_config"][0].clone()
            }
            "unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"permissions_config":true})
            }
            "sensitive" => {
                v["resource_changes"][5]["change"]["after_sensitive"] = json!({"arn":true})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["permissions_config"]
                    [0]["capacity_provider_operator_role_arn"] =
                    json!({"references":["aws_iam_role.operator.name"]})
            }
            "mismatch" => v["resource_changes"][5]["change"]["after"]["arn"] = json!("other"),
            "deleted" => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            "data" => v["resource_changes"][5]["mode"] = json!("data"),
            "literal" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["permissions_config"] =
                    json!([])
            }
            _ => {}
        }
        let p = plan::parse(&v.to_string()).unwrap();
        let raw = semantic::Input::new(&p);
        assert_eq!(
            raw.attributes.iter().any(|r| r.attribute
                == "permissions_config.capacity_provider_operator_role_arn"
                && super::super::service_references::verified(&raw, r)),
            case == "valid",
            "{case}"
        );
    }
}

#[test]
fn kms_key_contract_preserves_current_identity_filtering_and_named_states() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut pair = v.clone();
    pair["resource_changes"] = json!([v["resource_changes"][6], v["resource_changes"][4]]);
    pair["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][6],
        v["configuration"]["root_module"]["resources"][4]
    ]);
    super::super::service_references::tests::check_current_scalar_fixture(
        &pair.to_string(),
        "Capacity encryption key",
        "kms_key_arn",
        "arn",
    );
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("Capacity subnets")
        );
    }
}

#[test]
fn late_account_and_named_state_boundaries_preserve_capacity_identity() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_vpc")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type == "aws_lambda_capacity_provider")
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
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
