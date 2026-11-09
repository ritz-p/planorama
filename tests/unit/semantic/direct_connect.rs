use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/direct-connect-plan.json");
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
fn dedicated_dx_references_preserve_exact_identity_and_lifecycle_boundaries() {
    for (s, t, path, label) in [
        (0, 1, "connection_id", "Dedicated connection"),
        (1, 5, "connection_id", "Virtual interface connection"),
        (0, 6, "connection_id", "Virtual interface connection"),
        (0, 7, "connection_id", "Virtual interface connection"),
        (2, 5, "dx_gateway_id", "Direct Connect gateway"),
        (2, 7, "dx_gateway_id", "Direct Connect gateway"),
        (0, 8, "connection_id", "Dedicated connection"),
        (1, 8, "lag_id", "Connection LAG"),
        (5, 9, "virtual_interface_id", "Peer virtual interface"),
        (0, 10, "connection_id", "Dedicated connection"),
        (4, 10, "secret_arn", "Configured MACsec secret"),
        (3, 11, "vpn_gateway_id", "Virtual private gateway"),
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
fn private_vif_requires_one_known_gateway_variant() {
    for (t, other, label) in [
        (5, "vpn_gateway_id", "Direct Connect gateway"),
        (11, "dx_gateway_id", "Virtual private gateway"),
    ] {
        for case in 0..5 {
            let mut v = fixture();
            match case {
                1 => v["resource_changes"][t]["change"]["after"][other] = json!("other"),
                2 => v["resource_changes"][t]["change"]["after_unknown"] = json!({other:true}),
                3 => v["resource_changes"][t]["change"]["after_sensitive"] = json!({other:true}),
                4 => v["resource_changes"][t]["change"]["after"][other] = json!(""),
                _ => {}
            }
            assert_eq!(
                complete(&v, t, label),
                case == 0 || case == 4,
                "{t}: {case}"
            );
        }
    }
}
#[test]
fn dx_endpoint_allowlists_do_not_conflate_ids_arns_or_hosted_connections() {
    for (source, valid) in [
        ("aws_dx_connection", true),
        ("aws_dx_lag", true),
        ("aws_dx_hosted_connection", false),
        ("aws_instance", false),
    ] {
        let mut v = fixture();
        v["resource_changes"][0]["type"] = json!(source);
        assert_eq!(
            complete(&v, 6, "Virtual interface connection"),
            valid,
            "{source}"
        );
        assert_eq!(
            complete(&v, 10, "Dedicated connection"),
            source == "aws_dx_connection",
            "{source}"
        );
    }
    for source in [
        "aws_dx_private_virtual_interface",
        "aws_dx_public_virtual_interface",
        "aws_dx_transit_virtual_interface",
    ] {
        let mut v = fixture();
        v["resource_changes"][5]["type"] = json!(source);
        assert!(complete(&v, 9, "Peer virtual interface"));
    }
    for (t, path, label, source) in [
        (
            6,
            "connection_id",
            "Virtual interface connection",
            "aws_dx_connection.main",
        ),
        (
            10,
            "secret_arn",
            "Configured MACsec secret",
            "aws_secretsmanager_secret.macsec",
        ),
    ] {
        let mut v = fixture();
        let output = if path == "connection_id" { "arn" } else { "id" };
        v["configuration"]["root_module"]["resources"][t]["expressions"][path] =
            json!({"references":[format!("{source}.{output}")]});
        assert!(!complete(&v, t, label));
    }
}
#[test]
fn dx_global_gateway_does_not_inherit_connection_region_or_account() {
    for case in 0..4 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("123456789012");
            if n.resource_type == "aws_dx_gateway" {
                if case == 1 {
                    c.account = crate::model::AwsAccountScope::new("234567890123");
                }
                if case == 2 {
                    c.account_conflict = true;
                }
                if case == 3 {
                    c.region = Some(crate::model::RegionScope("us-west-2".into()));
                }
            }
        }
        let g = semantic::transform(&p);
        let gateway = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_dx_gateway")
            .unwrap();
        assert!(gateway.entity.scope.as_ref().unwrap().global);
        assert!(gateway.entity.scope.as_ref().unwrap().region.is_none());
        let vif = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_dx_private_virtual_interface.main")
            .unwrap();
        assert_eq!(
            vif.references
                .iter()
                .any(|r| r.label == "Direct Connect gateway" && r.complete),
            case == 0 || case == 3,
            "{case}"
        );
    }
}
#[test]
fn dx_shared_cards_and_unmarked_secret_payloads_remain_safe() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 12);
    for kind in ["aws_dx_connection", "aws_dx_lag", "aws_dx_gateway"] {
        assert!(
            g.nodes
                .iter()
                .filter(|n| n.resource_type == kind)
                .all(
                    |n| n.placement == crate::model::Placement::Spatial && n.member_owner.is_none()
                )
        );
    }
    assert!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_dx_"))
            .all(|n| n.member_owner.is_none())
    );
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
        assert!(!out.contains("PRIVATE_DX_PAYLOAD"));
    }
}
