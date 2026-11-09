use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/traffic-mirror-plan.json");

fn verified(value: &Value, target: usize, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let raw = semantic::Input::new(&parsed);
    let address = value["resource_changes"][target]["address"]
        .as_str()
        .unwrap();
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == address
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn exact_mirror_endpoints_preserve_lifecycle_scope_and_selection() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, output, label) in [
        (
            0,
            1,
            "traffic_mirror_filter_id",
            "id",
            "Traffic mirror filter",
        ),
        (
            0,
            9,
            "traffic_mirror_filter_id",
            "id",
            "Traffic mirror filter",
        ),
        (
            7,
            9,
            "traffic_mirror_target_id",
            "id",
            "Traffic mirror target",
        ),
        (
            3,
            9,
            "network_interface_id",
            "id",
            "Mirror network interface",
        ),
        (
            3,
            6,
            "network_interface_id",
            "id",
            "Mirror network interface",
        ),
        (
            4,
            7,
            "network_load_balancer_arn",
            "arn",
            "Mirror network load balancer",
        ),
        (
            5,
            8,
            "gateway_load_balancer_endpoint_id",
            "id",
            "Mirror gateway endpoint",
        ),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_current_scalar_fixture(
            &pair.to_string(),
            label,
            path,
            output,
        );
    }
}

#[test]
fn target_modes_and_rule_identity_fail_closed() {
    for case in [
        "alb",
        "interface",
        "conflict",
        "unknown-alternative",
        "sensitive-type",
        "direction",
        "unknown-direction",
        "rule-number",
        "rule-action",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (target, path) = match case {
            "alb" => {
                v["resource_changes"][4]["change"]["after"]["load_balancer_type"] =
                    json!("application");
                (7, "network_load_balancer_arn")
            }
            "interface" => {
                v["resource_changes"][5]["change"]["after"]["vpc_endpoint_type"] =
                    json!("Interface");
                (8, "gateway_load_balancer_endpoint_id")
            }
            "conflict" => {
                v["resource_changes"][7]["change"]["after"]["network_interface_id"] =
                    json!("eni-1");
                (7, "network_load_balancer_arn")
            }
            "unknown-alternative" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"network_interface_id":true});
                (7, "network_load_balancer_arn")
            }
            "sensitive-type" => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"load_balancer_type":true});
                (7, "network_load_balancer_arn")
            }
            "direction" => {
                v["resource_changes"][1]["change"]["after"]["traffic_direction"] = json!("both");
                (1, "traffic_mirror_filter_id")
            }
            "unknown-direction" => {
                v["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"traffic_direction":true});
                (1, "traffic_mirror_filter_id")
            }
            "rule-number" => {
                v["resource_changes"][1]["change"]["after_sensitive"] = json!({"rule_number":true});
                (1, "traffic_mirror_filter_id")
            }
            _ => {
                v["resource_changes"][1]["change"]["after"]["rule_action"] = json!("allow");
                (1, "traffic_mirror_filter_id")
            }
        };
        assert!(!verified(&v, target, path), "{case}");
    }
}

#[test]
fn filter_owns_distinct_rules_without_containing_mirror_endpoints() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 10);
    assert_eq!(
        g.nodes.iter().filter(|n| n.member_owner.is_some()).count(),
        2
    );
    for node in g
        .nodes
        .iter()
        .filter(|n| n.resource_type == "aws_ec2_traffic_mirror_filter_rule")
    {
        let owner = node.member_owner.as_ref().unwrap();
        assert_eq!(owner.section, "Mirror rules");
        assert_eq!(owner.attribute, "traffic_mirror_filter_id");
    }
    assert!(!g.edges.iter().any(|e| {
        e.kind == crate::model::EdgeKind::Containment
            && (g.nodes[e.from]
                .resource_type
                .starts_with("aws_ec2_traffic_mirror_")
                || g.nodes[e.to]
                    .resource_type
                    .starts_with("aws_ec2_traffic_mirror_"))
    }));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            output
                .content
                .contains("aws_ec2_traffic_mirror_filter_rule.ingress")
        );
        assert!(
            output
                .content
                .contains("aws_ec2_traffic_mirror_filter_rule.egress")
        );
    }
}

#[test]
fn late_account_evidence_removes_foreign_filter_ownership() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_ec2_traffic_mirror_filter")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Traffic mirror filter" && r.complete)
    );
    assert_eq!(g.coverage.selected.resources, 10);
}
