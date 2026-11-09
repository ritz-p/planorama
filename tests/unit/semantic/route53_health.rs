use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/route53-health-plan.json");
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
#[test]
fn health_scalar_references_preserve_identity_lifecycle_filter_and_states() {
    for (s, t, path, label) in [
        (3, 4, "cloudwatch_alarm_name", "Health check metric alarm"),
        (5, 6, "routing_control_arn", "Health check routing control"),
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
fn calculated_children_are_a_complete_unordered_id_set() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][2]["change"]["after"]["child_healthchecks"] =
                    json!(["health-http", "missing"])
            }
            2 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"child_healthchecks":[false,true]})
            }
            3 => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"child_healthchecks":true})
            }
            4 => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][1]["type"] = json!("aws_route53_zone"),
            6 => {
                v["resource_changes"][2]["change"]["after"]["child_healthchecks"] =
                    json!(["health-http", "health-http"])
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 2, "Calculated child health checks"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn health_modes_are_known_and_exact() {
    for (t, label) in [
        (2, "Calculated child health checks"),
        (4, "Health check metric alarm"),
        (6, "Health check routing control"),
    ] {
        for case in 0..4 {
            let mut v = fixture();
            match case {
                1 => v["resource_changes"][t]["change"]["after"]["type"] = json!("HTTP"),
                2 => v["resource_changes"][t]["change"]["after_unknown"] = json!({"type":true}),
                3 => v["resource_changes"][t]["change"]["after_sensitive"] = json!({"type":true}),
                _ => {}
            }
            assert_eq!(complete(&v, t, label), case == 0, "{t} {case}");
        }
    }
}
#[test]
fn metric_alarm_uses_explicit_region_and_alarm_name_output() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][4]["change"]["after"]["cloudwatch_alarm_region"] =
                    json!("us-west-2")
            }
            2 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"cloudwatch_alarm_region":true})
            }
            3 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["cloudwatch_alarm_name"] =
                    json!({"references":["aws_cloudwatch_metric_alarm.main.id"]})
            }
            4 => v["resource_changes"][3]["type"] = json!("aws_cloudwatch_composite_alarm"),
            5 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["region"] =
                    json!({"references":["var.unknown"]})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 4, "Health check metric alarm"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn health_global_scope_does_not_inherit_alarm_region_or_claim_reachability() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        let c = n.provider_configuration.as_mut().unwrap();
        c.account = crate::model::AwsAccountScope::new("111111111111");
        if n.resource_type == "aws_route53_health_check" {
            c.region = Some(crate::model::RegionScope("eu-west-1".into()));
        }
    }
    let g = semantic::transform(&p);
    assert!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == "aws_route53_health_check")
            .all(|n| n.entity.scope.as_ref().unwrap().global && n.member_owner.is_none())
    );
    assert!(
        g.nodes
            .iter()
            .find(|n| n.address == "aws_route53_health_check.metric")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Health check metric alarm" && r.complete)
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
            .contains("PRIVATE_HEALTH_PAYLOAD")
        );
    }
}
