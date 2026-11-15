use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/cloudwan-plan.json");
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
fn attachments_use_typed_explicit_endpoints_and_never_own_or_relocate_global_cores() {
    let v = fixture();
    for (t, l) in [
        (3, "Cloud WAN attachment VPC"),
        (3, "Cloud WAN attachment subnets"),
        (4, "Connect transport attachment"),
        (5, "Peer Connect attachment"),
        (5, "Tunnel-less peer subnet"),
        (7, "Cloud WAN prefix list"),
        (9, "Cloud WAN Direct Connect gateway"),
        (11, "Cloud WAN VPN connection"),
        (13, "Cloud WAN transit gateway"),
        (15, "Route table peering"),
        (15, "Peering transit gateway route table"),
        (16, "Cloud WAN core network"),
        (17, "Routing label attachment"),
    ] {
        assert!(complete(&v, t, l), "{t} {l}");
    }
    for t in 18..23 {
        assert!(complete(&v, t, "Accepted attachment"), "{t}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 23);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_networkmanager_"))
    {
        assert!(n.entity.scope.as_ref().unwrap().global);
        assert!(n.entity.scope.as_ref().unwrap().region.is_none());
    }
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
fn core_reference_lifecycle_filter_and_state_boundaries_remain_conservative() {
    let v = fixture();
    let mut pair = v.clone();
    pair["resource_changes"] = json!([v["resource_changes"][0], v["resource_changes"][16]]);
    pair["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][0],
        v["configuration"]["root_module"]["resources"][16]
    ]);
    super::super::service_references::tests::check_service_fixture(
        &pair.to_string(),
        "Cloud WAN core network",
        "core_network_id",
    );
}
#[test]
fn vpc_attachment_requires_complete_explicit_common_vpc_proof() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][3]["change"]["after"]["subnet_arns"] = json!([
                    v["resource_changes"][2]["change"]["after"]["arn"],
                    "literal"
                ])
            }
            2 => v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            3 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            4 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"vpc_id":true}),
            5 => v["resource_changes"][2]["deposed"] = json!("old"),
            _ => {}
        }
        assert_eq!(
            complete(&v, 3, "Cloud WAN attachment VPC"),
            case == 0,
            "{case}"
        );
        assert_eq!(
            complete(&v, 3, "Cloud WAN attachment subnets"),
            case == 0,
            "{case}"
        );
        assert_eq!(
            complete(&v, 4, "Connect transport attachment"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn connect_transport_and_peer_require_core_edge_region_and_protocol_proof() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][4]["change"]["after"]["core_network_id"] = json!("other"),
            2 => v["resource_changes"][4]["change"]["after"]["edge_location"] = json!("us-west-2"),
            3 => {
                v["resource_changes"][4]["change"]["after"]["options"][0]["protocol"] = json!("GRE")
            }
            4 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"options":[{"protocol":true}]})
            }
            5 => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"edge_location":true})
            }
            6 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["transport_attachment_id"] =
                    json!({"references":["aws_networkmanager_vpc_attachment.main.arn"]})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 4, "Connect transport attachment"),
            matches!(case, 0 | 3),
            "{case}"
        );
        assert_eq!(
            complete(&v, 5, "Tunnel-less peer subnet"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn accepter_type_and_attachment_output_are_exact_and_never_decode_composite_ids() {
    for case in 0..4 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][18]["change"]["after"]["attachment_type"] = json!("CONNECT"),
            2 => {
                v["configuration"]["root_module"]["resources"][18]["expressions"]["attachment_id"] =
                    json!({"references":["aws_networkmanager_vpc_attachment.main.attachment_id"]})
            }
            3 => {
                v["resource_changes"][18]["change"]["after_unknown"] =
                    json!({"attachment_type":true})
            }
            _ => {}
        }
        assert_eq!(complete(&v, 18, "Accepted attachment"), case == 0, "{case}");
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][19]["expressions"]["attachment_id"] =
        json!({"references":["aws_networkmanager_connect_attachment.main.attachment_id"]});
    assert!(complete(&v, 19, "Accepted attachment"));
}
#[test]
fn route_table_and_label_are_qualified_and_dx_edges_must_be_complete() {
    for case in 0..4 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][14]["change"]["after"]["transit_gateway_id"] = json!("other")
            }
            2 => {
                v["configuration"]["root_module"]["resources"][14]["expressions"]["transit_gateway_id"] =
                    json!({"constant_value":"tgw-main"})
            }
            3 => v["resource_changes"][12]["change"]["after_unknown"] = json!({"id":true}),
            _ => {}
        }
        assert_eq!(
            complete(&v, 15, "Peering transit gateway route table"),
            case == 0,
            "{case}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][3]["change"]["after"]["core_network_id"] = json!("other");
    assert!(!complete(&v, 17, "Routing label attachment"));
    for mask in [json!(true), json!([false, true]), json!([false])] {
        let mut v = fixture();
        v["resource_changes"][9]["change"]["after_unknown"] = json!({"edge_locations":mask});
        assert!(!complete(&v, 9, "Cloud WAN Direct Connect gateway"));
    }
}
#[test]
fn endpoint_account_and_region_are_independent_of_global_control_scope() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_subnet" {
                match case {
                    1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    2 => c.account_conflict = true,
                    3 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                    4 => c.region = None,
                    _ => {}
                }
            }
        }
        let g = semantic::transform(&p);
        let n = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_networkmanager_connect_peer")
            .unwrap();
        assert_eq!(
            n.references
                .iter()
                .any(|r| r.label == "Tunnel-less peer subnet" && r.complete),
            case == 0,
            "{case}"
        );
    }
}

#[test]
fn indirect_core_proof_rechecks_the_peering_account_for_labels_and_accepters() {
    for case in 0..4 {
        let mut v = fixture();
        v["resource_changes"][17]["change"]["after"]["attachment_id"] = json!("attachment-table");
        v["configuration"]["root_module"]["resources"][17]["expressions"]["attachment_id"] = json!({"references":["aws_networkmanager_transit_gateway_route_table_attachment.main.id"]});
        let mut p = plan::parse(&v.to_string()).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_networkmanager_transit_gateway_peering" {
                match case {
                    1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    2 => c.account_conflict = true,
                    3 => c.account = None,
                    _ => {}
                }
            }
        }
        let g = semantic::transform(&p);
        for (t, label) in [
            (17, "Routing label attachment"),
            (22, "Accepted attachment"),
        ] {
            let n = g
                .nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
                .unwrap();
            assert_eq!(
                n.references.iter().any(|r| r.label == label && r.complete),
                case == 0,
                "{label} {case}"
            );
        }
    }
}
