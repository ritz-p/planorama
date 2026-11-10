use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/classic-elb-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .is_some_and(|n| n.references.iter().any(|r| r.label == label && r.complete))
}
#[test]
fn classic_elb_has_independent_card_network_and_complete_nested_references() {
    let v = fixture();
    for label in [
        "Classic ELB subnets",
        "Classic ELB security groups",
        "Classic ELB instances",
        "Classic listener certificate",
        "Classic ELB log bucket",
    ] {
        assert!(complete(&v, 6, label), "{label}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let elb = g
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_elb")
        .unwrap();
    assert_eq!(g.nodes[elb].placement, crate::model::Placement::Spatial);
    assert!(
        g.edges
            .iter()
            .any(|e| e.to == elb && e.kind == crate::model::EdgeKind::Containment)
    );
    assert!(g.edges.iter().any(|e| {
        e.kind == crate::model::EdgeKind::Association
            && e.change
                .as_ref()
                .is_some_and(|c| c.address == "aws_elb_attachment.app")
    }));
    assert_eq!(g.coverage.selected.resources, 15);
    for t in 8..15 {
        assert!(complete(&v, t, "Classic load balancer"), "{t}");
    }
    for t in 13..15 {
        assert!(complete(&v, t, "Qualified Classic ELB policies"), "{t}");
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
fn classic_certificates_and_logs_require_correlated_enabled_protocols() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][6]["change"]["after"]["listener"][1]["lb_protocol"] =
                    json!("TCP")
            }
            2 => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"listener":[{}, {"lb_protocol":true}]})
            }
            3 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["listener"] =
                    json!([{ "ssl_certificate_id":{"references":["aws_acm_certificate.tls.arn"]}}])
            }
            4 => {
                v["resource_changes"][6]["change"]["after"]["access_logs"][0]["enabled"] =
                    json!(false)
            }
            5 => {
                v["resource_changes"][6]["change"]["after_sensitive"] =
                    json!({"access_logs":[{"enabled":true}]})
            }
            6 => {
                v["resource_changes"][6]["change"]["after"]["access_logs"] = json!([{ "enabled":true,"bucket":"classic-logs"},{"enabled":true,"bucket":"classic-logs"}])
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 6, "Classic listener certificate"),
            !matches!(case, 1..=3),
            "cert {case}"
        );
        assert_eq!(
            complete(&v, 6, "Classic ELB log bucket"),
            case < 4,
            "log {case}"
        );
    }
}
#[test]
fn policy_names_and_ports_are_qualified_by_the_same_explicit_classic_elb() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][13]["change"]["after"]["load_balancer_port"] = json!(444),
            2 => {
                v["resource_changes"][12]["change"]["after"]["load_balancer_name"] =
                    json!("different")
            }
            3 => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["load_balancer_name"] =
                    json!({"constant_value":"classic"})
            }
            4 => {
                v["resource_changes"][13]["change"]["after"]["policy_names"] =
                    json!(["custom", "AWS-predefined"])
            }
            5 => v["resource_changes"][6]["type"] = json!("aws_lb"),
            6 => v["resource_changes"][6]["change"]["after_unknown"] = json!({"listener":true}),
            _ => {}
        }
        assert_eq!(
            complete(&v, 13, "Qualified Classic ELB policies"),
            case == 0,
            "{case}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after"]["instance_ports"] = json!(["8080"]);
    assert!(!complete(&v, 11, "Classic load balancer"));
}
#[test]
fn shared_backend_ports_preserve_policy_ownership_and_check_every_protocol() {
    for case in 0..6 {
        let mut v = fixture();
        v["resource_changes"][6]["change"]["after"]["listener"][0]["instance_protocol"] =
            json!("TCP");
        v["resource_changes"][6]["change"]["after"]["listener"][1]["instance_port"] = json!(8080);
        v["resource_changes"][11]["change"]["after"]["instance_ports"] = json!(["8080"]);
        match case {
            1 => {
                v["resource_changes"][6]["change"]["after"]["listener"][1]["instance_protocol"] =
                    json!("HTTP")
            }
            2 => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"listener":[{}, {"instance_protocol":true}]})
            }
            3 => {
                v["resource_changes"][6]["change"]["after_sensitive"] =
                    json!({"listener":[{}, {"instance_protocol":true}]})
            }
            4 => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"listener":[{}, {"instance_port":true}]})
            }
            5 => v["resource_changes"][6]["change"]["after"]["listener"][1]["lb_port"] = json!(443),
            _ => {}
        }
        assert_eq!(
            complete(&v, 11, "Classic load balancer"),
            matches!(case, 0 | 5),
            "proxy {case}"
        );
        assert_eq!(
            complete(&v, 14, "Qualified Classic ELB policies"),
            case != 4,
            "backend {case}"
        );
        assert_eq!(
            complete(&v, 13, "Classic load balancer"),
            case != 5,
            "listener {case}"
        );
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        for (kind, expected) in [
            ("aws_proxy_protocol_policy", matches!(case, 0 | 5)),
            ("aws_load_balancer_backend_server_policy", case != 4),
        ] {
            assert_eq!(
                g.nodes
                    .iter()
                    .find(|n| n.resource_type == kind)
                    .unwrap()
                    .member_owner
                    .is_some(),
                expected,
                "{kind} {case}"
            );
        }
    }
}
#[test]
fn elb_reference_lifecycle_and_filter_boundaries_are_preserved() {
    for (s, t, path, label) in [
        (1, 6, "subnets", "Classic ELB subnets"),
        (6, 12, "load_balancer_name", "Classic load balancer"),
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
fn classic_network_and_policy_context_recheck_scope() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_elb" {
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
        let policy = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_load_balancer_listener_policy")
            .unwrap();
        assert_eq!(
            policy
                .references
                .iter()
                .any(|r| r.label == "Qualified Classic ELB policies" && r.complete),
            case == 0,
            "{case}"
        );
    }
}

#[test]
fn incomplete_networks_and_ambiguous_listeners_do_not_establish_containment_or_ownership() {
    for case in 0..4 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][6]["change"]["after"]["subnets"] =
                    json!(["subnet-one", "literal"])
            }
            2 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            3 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"vpc_id":true}),
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let elb = g
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_elb")
            .unwrap();
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.to == elb && e.kind == crate::model::EdgeKind::Containment),
            case == 0,
            "{case}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["listener"][1]["lb_port"] = json!(443);
    assert!(!complete(&v, 8, "Classic load balancer"));
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after"]["elb"] =
        json!("arn:aws:elasticloadbalancing:us-east-1:111111111111:loadbalancer/classic");
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        g.nodes
            .iter()
            .any(|n| n.resource_type == "aws_elb_attachment")
    );
}
