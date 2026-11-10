use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/networkmanager-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn topology_references_preserve_independent_architecture_and_global_scope() {
    let v = fixture();
    for (t, l) in [
        (1, "Global network"),
        (2, "Global network"),
        (4, "Network Manager site"),
        (4, "Device AWS subnet context"),
        (6, "Network Manager site"),
        (7, "Network Manager device"),
        (7, "Connected device"),
        (7, "Network Manager link"),
        (7, "Connected link"),
        (8, "Network Manager device"),
        (12, "Associated customer gateway"),
        (13, "Associated transit gateway peer"),
        (14, "Registered transit gateway"),
    ] {
        assert!(complete(&v, t, l), "{t} {l}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 15);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
        if super::resource(&n.resource_type) {
            assert!(n.entity.scope.as_ref().unwrap().global);
            assert!(n.entity.scope.as_ref().unwrap().region.is_none());
        }
    }
    for kind in ["global_network", "core_network", "site", "device", "link"] {
        assert!(
            g.nodes
                .iter()
                .filter(|n| n.resource_type == format!("aws_networkmanager_{kind}"))
                .all(|n| n.placement == crate::model::Placement::Spatial)
        );
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
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
        assert!(!out.contains("PRIVATE_"));
    }
}
#[test]
fn network_id_lifecycle_and_named_state_boundaries_are_preserved() {
    let v = fixture();
    let mut pair = v.clone();
    pair["resource_changes"] = json!([v["resource_changes"][0], v["resource_changes"][1]]);
    pair["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][0],
        v["configuration"]["root_module"]["resources"][1]
    ]);
    super::super::service_references::tests::check_service_fixture(
        &pair.to_string(),
        "Global network",
        "global_network_id",
    );
}
#[test]
fn matching_ids_do_not_replace_explicit_same_global_network_proof() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][4]["change"]["after"]["global_network_id"] =
                    json!("global-other")
            }
            2 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["global_network_id"] =
                    json!({"constant_value":"global-main"})
            }
            3 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
            4 => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"global_network_id":true})
            }
            5 => v["resource_changes"][0]["deposed"] = json!("old"),
            6 => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        assert_eq!(
            complete(&v, 7, "Network Manager device"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn device_subnet_requires_exact_arn_and_single_known_location_without_zone() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["aws_location"]
                    [0]["subnet_arn"] = json!({"references":["aws_subnet.device.id"]})
            }
            2 => {
                v["resource_changes"][4]["change"]["after"]["aws_location"][0]["zone"] =
                    json!("us-west-2a")
            }
            3 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"aws_location":[{"zone":true}]})
            }
            4 => {
                v["resource_changes"][4]["change"]["after"]["aws_location"] =
                    json!([{"subnet_arn":"one"},{"subnet_arn":"two"}])
            }
            5 => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        assert_eq!(
            complete(&v, 4, "Device AWS subnet context"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn regional_endpoints_and_global_joins_recheck_independent_accounts() {
    for case in 0..4 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_subnet" {
                match case {
                    1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    2 => c.account_conflict = true,
                    3 => c.region = None,
                    _ => {}
                }
            }
        }
        let g = semantic::transform(&p);
        let n = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_networkmanager_device.one")
            .unwrap();
        assert_eq!(
            n.references
                .iter()
                .any(|r| r.label == "Device AWS subnet context" && r.complete),
            case == 0,
            "{case}"
        );
        assert!(n.entity.scope.as_ref().unwrap().region.is_none());
    }
}
