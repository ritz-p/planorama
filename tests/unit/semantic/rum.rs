use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/rum-plan.json");
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
fn rum_scalar_paths_preserve_lifecycle_filter_and_named_states() {
    for (s, t, path, label) in [
        (2, 3, "app_monitor_name", "RUM app monitor"),
        (0, 3, "iam_role_arn", "Evidently destination role"),
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
fn rum_configuration_requires_correlated_singleton_evidence() {
    for (field, label, s) in [
        ("guest_role_arn", "RUM guest role", 0),
        ("identity_pool_id", "RUM identity pool", 1),
    ] {
        for case in 0..11 {
            let mut v = fixture();
            match case {
                1 => {
                    v["resource_changes"][2]["change"]["after_unknown"] =
                        json!({"app_monitor_configuration":[{field:true}]})
                }
                2 => {
                    v["resource_changes"][2]["change"]["after_sensitive"] =
                        json!({"app_monitor_configuration":true})
                }
                3 => {
                    v["resource_changes"][2]["change"]["after"]["app_monitor_configuration"] =
                        v["resource_changes"][2]["change"]["after"]["app_monitor_configuration"][0]
                            .clone()
                }
                4 => {
                    let b =
                        v["resource_changes"][2]["change"]["after"]["app_monitor_configuration"][0]
                            .clone();
                    v["resource_changes"][2]["change"]["after"]["app_monitor_configuration"]
                        .as_array_mut()
                        .unwrap()
                        .push(b);
                }
                5 => {
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["app_monitor_configuration"]
                        [0][field] = json!({"constant_value":"literal"})
                }
                6 => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                7 => v["resource_changes"][s]["provider_name"] = json!("example.com/foreign/aws"),
                8 => v["resource_changes"][s]["type"] = json!("aws_cognito_user_pool"),
                9 => v["resource_changes"][s]["deposed"] = json!("old"),
                10 => {
                    v["resource_changes"][2]["change"]["after"]["app_monitor_configuration"][0]
                        [field] = json!("wrong")
                }
                _ => {}
            }
            assert_eq!(complete(&v, 2, label), case == 0, "{field} {case}");
        }
    }
}
#[test]
fn rum_destination_role_requires_known_evidently_mode() {
    for mode in ["Evidently", "CloudWatch", "unknown"] {
        let mut v = fixture();
        v["resource_changes"][3]["change"]["after"]["destination"] = json!(mode);
        assert_eq!(
            complete(&v, 3, "Evidently destination role"),
            mode == "Evidently"
        );
        assert_eq!(complete(&v, 3, "RUM app monitor"), mode != "unknown");
    }
    for mask in ["after_unknown", "after_sensitive"] {
        let mut v = fixture();
        v["resource_changes"][3]["change"][mask] = json!({"destination":true});
        assert!(!complete(&v, 3, "Evidently destination role"));
    }
}
#[test]
fn rum_pool_region_and_role_account_are_independent() {
    for (s, label) in [(0, "RUM guest role"), (1, "RUM identity pool")] {
        for case in 0..4 {
            let mut p = plan::parse(FIXTURE).unwrap();
            for n in &mut p.nodes {
                n.provider_configuration.as_mut().unwrap().account =
                    crate::model::AwsAccountScope::new("111111111111");
            }
            let address = fixture()["resource_changes"][s]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            let c = p
                .nodes
                .iter_mut()
                .find(|n| n.address == address)
                .unwrap()
                .provider_configuration
                .as_mut()
                .unwrap();
            match case {
                1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                2 => c.account_conflict = true,
                3 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                _ => {}
            }
            let g = semantic::transform(&p);
            assert_eq!(
                g.nodes
                    .iter()
                    .find(|n| n.resource_type == "aws_rum_app_monitor")
                    .unwrap()
                    .references
                    .iter()
                    .any(|r| r.label == label && r.complete),
                case == 0 || (s == 0 && case == 3),
                "{s} {case}"
            );
        }
    }
}
#[test]
fn rum_shared_identities_and_private_payloads_remain_safe() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 5);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        2
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
        for text in ["PRIVATE_RUM_PAYLOAD", "computed-log", "opaque-monitor-id"] {
            assert!(!out.contains(text));
        }
    }
}
