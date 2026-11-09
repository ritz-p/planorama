use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/vpclattice-resources-plan.json");
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
fn resource_configuration_references_preserve_exact_outputs_and_lifecycle() {
    for (s, t, path, label) in [
        (4, 6, "resource_gateway_identifier", "Configuration gateway"),
        (4, 8, "resource_gateway_identifier", "Configuration gateway"),
        (4, 9, "resource_gateway_identifier", "Configuration gateway"),
        (
            6,
            7,
            "resource_configuration_group_id",
            "Parent resource group",
        ),
        (
            5,
            8,
            "domain_verification_id",
            "Configured domain verification",
        ),
        (
            6,
            11,
            "resource_configuration_identifier",
            "Associated resource configuration",
        ),
        (
            10,
            11,
            "service_network_identifier",
            "Lattice service network",
        ),
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
fn resource_gateway_requires_complete_common_vpc_collections_for_containment() {
    for case in 0..12 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            2 => {
                v["resource_changes"][4]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "literal"])
            }
            3 => v["resource_changes"][4]["change"]["after_unknown"] = json!({"subnet_ids":true}),
            4 => v["resource_changes"][3]["change"]["after_sensitive"] = json!({"vpc_id":true}),
            5 => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-resources"})
            }
            6 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            7 => v["resource_changes"][2]["deposed"] = json!("old"),
            8 => v["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws"),
            9 => {
                v["resource_changes"][4]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "subnet-a"])
            }
            10 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"security_group_ids":true})
            }
            11 => {
                v["resource_changes"][4]["change"]["after"]["security_group_ids"] = json!([]);
                v["configuration"]["root_module"]["resources"][4]["expressions"]["security_group_ids"] =
                    json!({"constant_value":[]});
            }
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].resource_type == "aws_vpclattice_resource_gateway"),
            case == 0 || case == 11,
            "{case}"
        );
        assert!(
            !g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].resource_type == "aws_vpclattice_resource_configuration")
        );
        assert_eq!(g.coverage.selected.resources, 12);
    }
}

#[test]
fn configurations_require_known_variant_and_group_parent_type() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][7]["change"]["after"]["type"] = json!("SINGLE"),
            2 => v["resource_changes"][6]["change"]["after"]["type"] = json!("SINGLE"),
            3 => v["resource_changes"][6]["change"]["after_unknown"] = json!({"type":true}),
            4 => v["resource_changes"][7]["change"]["after_sensitive"] = json!({"type":true}),
            5 => {
                v["resource_changes"][7]["change"]["after"]["resource_configuration_group_id"] =
                    json!("other")
            }
            6 => v["resource_changes"][6]["type"] = json!("aws_vpclattice_resource_gateway"),
            _ => {}
        }
        assert_eq!(
            complete(&v, 7, "Parent resource group"),
            case == 0,
            "{case}"
        );
    }
    for mode in ["SINGLE", "GROUP", "ARN", "CHILD", "invalid"] {
        let mut v = fixture();
        v["resource_changes"][8]["change"]["after"]["type"] = json!(mode);
        assert_eq!(
            complete(&v, 8, "Configuration gateway"),
            matches!(mode, "SINGLE" | "GROUP" | "ARN"),
            "{mode}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["type"] = json!("CHILD");
    assert!(!complete(&v, 11, "Associated resource configuration"));
}

#[test]
fn resources_keep_scope_selection_actions_and_opaque_target_definitions() {
    let mut v = fixture();
    v["resource_changes"][7]["previous_address"] =
        json!("aws_vpclattice_resource_configuration.old");
    v["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    let p = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&p);
    let child = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_vpclattice_resource_configuration.child")
        .unwrap();
    assert_eq!(child.action, crate::model::Action::Replace);
    assert!(child.member_owner.is_some());
    let gateway = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_vpclattice_resource_gateway")
        .unwrap();
    assert_eq!(gateway.placement, crate::model::Placement::Spatial);
    assert!(gateway.member_owner.is_none());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_vpclattice_resource_configuration.child".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
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
        assert!(!out.contains("PRIVATE_RESOURCE_PAYLOAD"));
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_subnet" {
                "234567890123"
            } else {
                "123456789012"
            });
    }
    let g = semantic::transform(&p);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type == "aws_vpclattice_resource_gateway")
    );
}
