use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/identitystore-plan.json");

#[test]
fn membership_uses_exact_principal_outputs_and_store_qualifiers() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, path, field, label) in [
        (0, "group_id", "group_id", "Identity Store group"),
        (1, "member_id", "user_id", "Identity Store member"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] = json!([v["resource_changes"][source], v["resource_changes"][2]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][2]
        ]);
        super::super::service_references::tests::check_current_scalar_fixture(
            &pair.to_string(),
            label,
            path,
            field,
        );
        for case in [
            "valid",
            "different-store",
            "missing-store",
            "unknown-store",
            "endpoint-unknown",
            "sensitive-store",
            "composite-output",
        ] {
            let mut p = pair.clone();
            match case {
                "different-store" => {
                    p["resource_changes"][0]["change"]["after"]["identity_store_id"] =
                        json!("d-other")
                }
                "missing-store" => {
                    p["resource_changes"][1]["change"]["after"]["identity_store_id"] = Value::Null
                }
                "unknown-store" => {
                    p["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"identity_store_id":true})
                }
                "endpoint-unknown" => {
                    p["resource_changes"][0]["change"]["after_unknown"] =
                        json!({"identity_store_id":true})
                }
                "sensitive-store" => {
                    p["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"identity_store_id":true})
                }
                "composite-output" => {
                    let address = p["resource_changes"][0]["address"].as_str().unwrap();
                    p["configuration"]["root_module"]["resources"][1]["expressions"][path] =
                        json!({"references":[format!("{address}.id")]});
                }
                _ => {}
            }
            let parsed = plan::parse(&p.to_string()).unwrap();
            let raw = semantic::Input::new(&parsed);
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| r.attribute == path
                        && super::super::service_references::verified(&raw, r)),
                case == "valid",
                "{path} {case}"
            );
        }
    }
}

#[test]
fn membership_keeps_independent_identity_and_rechecks_late_scope() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 3);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(
        g.edges
            .iter()
            .all(|e| e.kind != crate::model::EdgeKind::Containment)
    );
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
            .contains("Identity Store member")
        );
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_identitystore_user" {
                "210987654321"
            } else {
                "123456789012"
            });
    }
    let raw = semantic::Input::new(&p);
    assert!(
        !raw.attributes
            .iter()
            .any(|r| r.attribute == "member_id"
                && super::super::service_references::verified(&raw, r))
    );
    assert!(
        raw.attributes
            .iter()
            .any(|r| r.attribute == "group_id"
                && super::super::service_references::verified(&raw, r))
    );
}
