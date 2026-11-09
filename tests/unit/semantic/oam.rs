use crate::plan::evidence::tests::parse_fixture as verified_parse;
use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/oam-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    semantic::transform(&verified_parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn link_retains_independently_verified_accounts_and_sink_policy_ownership() {
    let v = fixture();
    assert!(complete(&v, 2, "OAM monitoring sink"));
    assert!(complete(&v, 1, "OAM policy sink"));
    let g = semantic::transform(&verified_parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 3);
    let sink = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_oam_sink")
        .unwrap();
    let link = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_oam_link")
        .unwrap();
    let policy = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_oam_sink_policy")
        .unwrap();
    assert_ne!(
        sink.entity.scope.as_ref().unwrap().account,
        link.entity.scope.as_ref().unwrap().account
    );
    assert!(sink.member_owner.is_none());
    assert!(link.member_owner.is_none());
    assert!(policy.member_owner.is_some());
    assert_eq!(sink.placement, crate::model::Placement::Spatial);
    let ordinary = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(
        !ordinary
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_oam_link")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "OAM monitoring sink" && r.complete)
    );
}
#[test]
fn sink_policy_preserves_lifecycle_selection_and_state_boundaries() {
    let mut v = fixture();
    v["resource_changes"].as_array_mut().unwrap().truncate(2);
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    super::super::service_references::tests::check_service_fixture(
        &v.to_string(),
        "OAM policy sink",
        "sink_identifier",
    );
}
#[test]
fn link_rejects_wrong_identity_computed_fallback_and_unresolved_lifecycle() {
    for case in 0..9 {
        let mut v = fixture();
        match case {
            1 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["sink_identifier"] =
                    json!({"references":["aws_oam_sink.monitor.sink_id"]})
            }
            2 => v["resource_changes"][0]["type"] = json!("aws_oam_sink_policy"),
            3 => v["resource_changes"][0]["deposed"] = json!("old"),
            4 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            5 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"sink_identifier":true})
            }
            6 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true}),
            7 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"] =
                    json!({"sink_arn":{"references":["aws_oam_sink.monitor.arn"]}})
            }
            8 => v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            _ => {}
        }
        assert_eq!(complete(&v, 2, "OAM monitoring sink"), case == 0, "{case}");
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][2]["expressions"]["sink_identifier"] =
        json!({"references":["aws_oam_sink.monitor.id"]});
    assert!(complete(&v, 2, "OAM monitoring sink"));
}
#[test]
fn link_requires_known_nonconflicting_accounts_and_equal_independent_regions() {
    for kind in ["aws_oam_link", "aws_oam_sink"] {
        for case in 0..5 {
            let mut p = verified_parse(FIXTURE).unwrap();
            let c = p
                .nodes
                .iter_mut()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .provider_configuration
                .as_mut()
                .unwrap();
            match case {
                1 => c.account = None,
                2 => c.account_conflict = true,
                3 => c.region = None,
                4 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                _ => {}
            }
            let g = semantic::transform(&p);
            let n = g
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_oam_link")
                .unwrap();
            assert_eq!(
                n.references
                    .iter()
                    .any(|r| r.label == "OAM monitoring sink" && r.complete),
                case == 0,
                "{kind} {case}"
            );
        }
    }
    let mut p = verified_parse(FIXTURE).unwrap();
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_oam_sink_policy")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("111111111111");
    let g = semantic::transform(&p);
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_oam_sink_policy")
            .unwrap()
            .member_owner
            .is_none()
    );
}
#[test]
fn filters_and_resource_type_labels_never_resolve_log_groups_or_expose_payloads() {
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
