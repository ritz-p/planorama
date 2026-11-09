use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/identity-center-plan.json");

fn verified(v: &Value, target: usize, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        r.attribute == path
            && raw.nodes[r.target].address
                == v["resource_changes"][target]["address"].as_str().unwrap()
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn assignment_and_owner_references_require_exact_current_managed_endpoints() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let p = plan::parse(FIXTURE).unwrap();
    let raw = semantic::Input::new(&p);
    let refs: Vec<_> = raw
        .attributes
        .iter()
        .filter(|r| super::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some())
        .collect();
    assert_eq!(refs.len(), 18);
    for r in refs {
        let target = v["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.target].address)
            .unwrap();
        let source = v["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let path = r.attribute.as_str();
        let field = r.source_attributes.first().unwrap();
        for case in [
            "valid",
            "literal",
            "unknown",
            "sensitive",
            "endpoint-sensitive",
            "wrong-value",
            "wrong-output",
            "deleted",
            "data",
            "deposed",
            "foreign",
        ] {
            let mut p = v.clone();
            match case {
                "literal" => {
                    p["configuration"]["root_module"]["resources"][target]["expressions"][path] =
                        json!({})
                }
                "unknown" => {
                    p["resource_changes"][target]["change"]["after_unknown"] = json!({path:true})
                }
                "sensitive" => {
                    p["resource_changes"][target]["change"]["after_sensitive"] = json!({path:true})
                }
                "endpoint-sensitive" => {
                    p["resource_changes"][source]["change"]["after_sensitive"] = json!({field:true})
                }
                "wrong-value" => {
                    p["resource_changes"][source]["change"]["after"][field] = json!("other")
                }
                "wrong-output" => {
                    let address = p["resource_changes"][source]["address"].as_str().unwrap();
                    p["configuration"]["root_module"]["resources"][target]["expressions"][path] =
                        json!({"references":[format!("{address}.name")]});
                }
                "deleted" => p["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "data" => p["resource_changes"][source]["mode"] = json!("data"),
                "deposed" => p["resource_changes"][source]["deposed"] = json!("old"),
                "foreign" => {
                    p["resource_changes"][source]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                _ => {}
            }
            assert_eq!(
                verified(&p, target, path),
                case == "valid",
                "{} {path} {case}",
                v["resource_changes"][target]["address"]
            );
        }
    }
}

#[test]
fn principals_require_discriminator_and_unambiguous_instance_store_bridge() {
    for target in [6, 7, 9, 10] {
        for case in [
            "valid",
            "indexed",
            "different-lookup",
            "sensitive-lookup",
            "wrong-type",
            "unknown-type",
            "wrong-store",
            "unknown-store",
            "literal-store",
            "unknown-instances",
            "multiple-instances",
            "different-instance",
            "literal-instance",
            "deleted-bridge",
            "managed-bridge",
            "foreign-bridge",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let principal = if matches!(target, 6 | 9) { 1 } else { 2 };
            let instance_owner = if target < 8 { target } else { 4 };
            match case {
                "indexed" => {
                    v["configuration"]["root_module"]["resources"][instance_owner]["expressions"]
                        ["instance_arn"] =
                        json!({"references":["data.aws_ssoadmin_instances.current.arns[0]"]});
                    v["configuration"]["root_module"]["resources"][principal]["expressions"]["identity_store_id"] = json!({"references":["data.aws_ssoadmin_instances.current.identity_store_ids[0]"]});
                }
                "different-lookup" => {
                    let mut change = v["resource_changes"][0].clone();
                    change["address"] = json!("data.aws_ssoadmin_instances.other");
                    v["resource_changes"].as_array_mut().unwrap().push(change);
                    let mut config = v["configuration"]["root_module"]["resources"][0].clone();
                    config["address"] = json!("data.aws_ssoadmin_instances.other");
                    v["configuration"]["root_module"]["resources"]
                        .as_array_mut()
                        .unwrap()
                        .push(config);
                    v["configuration"]["root_module"]["resources"][principal]["expressions"]["identity_store_id"] = json!({"references":["data.aws_ssoadmin_instances.other.identity_store_ids"]});
                }
                "sensitive-lookup" => {
                    v["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"identity_store_ids":[true]})
                }
                "wrong-type" => {
                    v["resource_changes"][target]["change"]["after"]["principal_type"] =
                        json!("OTHER")
                }
                "unknown-type" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({"principal_type":true})
                }
                "wrong-store" => {
                    v["resource_changes"][principal]["change"]["after"]["identity_store_id"] =
                        json!("d-other")
                }
                "unknown-store" => {
                    v["resource_changes"][principal]["change"]["after_unknown"] =
                        json!({"identity_store_id":true})
                }
                "literal-store" => {
                    v["configuration"]["root_module"]["resources"][principal]["expressions"] =
                        json!({})
                }
                "unknown-instances" => {
                    v["resource_changes"][0]["change"]["after_unknown"] = json!({"arns":[true]})
                }
                "multiple-instances" => {
                    v["resource_changes"][0]["change"]["after"]["arns"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("other"));
                    v["resource_changes"][0]["change"]["after"]["identity_store_ids"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("d-other"));
                }
                "different-instance" => {
                    v["resource_changes"][instance_owner]["change"]["after"]["instance_arn"] =
                        json!("other")
                }
                "literal-instance" => {
                    v["configuration"]["root_module"]["resources"][instance_owner]["expressions"]
                        ["instance_arn"] = json!({})
                }
                "deleted-bridge" => {
                    v["resource_changes"][0]["change"]["actions"] = json!(["delete"])
                }
                "managed-bridge" => v["resource_changes"][0]["mode"] = json!("managed"),
                "foreign-bridge" => {
                    v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws")
                }
                _ => {}
            }
            assert_eq!(
                verified(&v, target, "principal_id"),
                matches!(case, "valid" | "indexed"),
                "{target} {case}"
            );
        }
    }
}

#[test]
fn permission_set_instance_and_target_account_are_independent_qualifiers() {
    for target in [6, 7, 12, 13, 14, 15, 16, 17] {
        for case in ["different", "unknown", "sensitive"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "different" => {
                    v["resource_changes"][target]["change"]["after"]["instance_arn"] =
                        json!("other")
                }
                "unknown" => {
                    v["resource_changes"][3]["change"]["after_unknown"] =
                        json!({"instance_arn":true})
                }
                _ => {
                    v["resource_changes"][target]["change"]["after_sensitive"] =
                        json!({"instance_arn":true})
                }
            }
            assert!(
                !verified(&v, target, "permission_set_arn"),
                "{target} {case}"
            );
        }
    }
    for case in ["wrong-type", "unknown-type", "bad-id"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "wrong-type" => {
                v["resource_changes"][6]["change"]["after"]["target_type"] = json!("OTHER")
            }
            "unknown-type" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"target_type":true})
            }
            _ => {
                v["resource_changes"][6]["change"]["after"]["target_id"] = json!("bad");
                v["resource_changes"][5]["change"]["after"]["id"] = json!("bad");
            }
        }
        assert!(!verified(&v, 6, "target_id"));
        assert!(verified(&v, 6, "permission_set_arn"));
        assert!(verified(&v, 6, "principal_id"));
    }
}

#[test]
fn identity_center_preserves_owners_empty_controllers_and_scope_boundaries() {
    let p = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 18);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        10
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_ssoadmin_account_assignment"
                    | "aws_identitystore_user"
                    | "aws_identitystore_group"
                    | "aws_ssoadmin_permission_set"
                    | "aws_ssoadmin_application"
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
        assert!(out.contains("Identity Center principal"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("OPAQUE_TARGET"));
    }
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_ssoadmin_application_assignment.user".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
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
    let mut p = p;
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_ssoadmin_instances" {
                "210987654321"
            } else {
                "123456789012"
            });
    }
    let raw = semantic::Input::new(&p);
    assert!(
        !raw.attributes.iter().any(|r| r.attribute == "principal_id"
            && super::super::service_references::verified(&raw, r))
    );
}
