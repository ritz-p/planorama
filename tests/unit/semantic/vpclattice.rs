use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/vpclattice-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, target: usize, label: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let address = v["resource_changes"][target]["address"].as_str().unwrap();
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}

#[test]
fn lattice_scalar_endpoints_preserve_lifecycle_identity_and_output_contracts() {
    for (s, t, path, label) in [
        (0, 3, "certificate_arn", "Service certificate"),
        (3, 9, "service_identifier", "Lattice service"),
        (5, 8, "target_group_identifier", "Attachment target group"),
        (3, 11, "service_identifier", "Lattice service"),
        (
            4,
            11,
            "service_network_identifier",
            "Lattice service network",
        ),
        (
            4,
            12,
            "service_network_identifier",
            "Lattice service network",
        ),
        (3, 13, "resource_identifier", "Lattice policy resource"),
        (4, 14, "resource_arn", "Lattice policy resource"),
        (4, 16, "resource_identifier", "Lattice logging resource"),
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
fn lattice_forward_lists_require_complete_matching_rows_and_parent_agreement() {
    for t in [9, 10] {
        let action = if t == 9 { "default_action" } else { "action" };
        for case in 0..11 {
            let mut v = fixture();
            match case {
                1=>v["resource_changes"][t]["change"]["after"][action][0]["forward"][0]["target_groups"].as_array_mut().unwrap().swap(0,1),
                2=>v["configuration"]["root_module"]["resources"][t]["expressions"][action][0]["forward"][0]["target_groups"][1]=json!({"target_group_identifier":{"constant_value":"tg-second"}}),
                3=>v["resource_changes"][t]["change"]["after"][action][0]["fixed_response"]=json!([{"status_code":200}]),
                4=>v["resource_changes"][t]["change"]["after_unknown"]=json!({action:true}),
                5=>v["resource_changes"][6]["change"]["after_sensitive"]=json!({"id":true}),
                6=>v["resource_changes"][9]["change"]["after"]["service_arn"]=json!("different"),
                7=>v["resource_changes"][3]["change"]["actions"]=json!(["delete"]),
                8=>v["resource_changes"][6]["deposed"]=json!("old"),
                9=>v["resource_changes"][t]["change"]["after"][action][0]["forward"][0]["target_groups"].as_array_mut().unwrap().push(json!({"target_group_identifier":"literal"})),
                10=>v["configuration"]["root_module"]["resources"][t]["expressions"][action][0]["forward"][0]["target_groups"][1]=json!({"target_group_identifier":{"references":["aws_vpclattice_target_group.second.name"]}}),
                _=>{},
            }
            assert_eq!(
                complete(&v, t, "Configured target groups"),
                case == 0,
                "{t}: {case}"
            );
        }
    }
    for case in 0..4 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][10]["change"]["after"]["service_identifier"] = json!("other")
            }
            2 => {
                v["configuration"]["root_module"]["resources"][10]["expressions"]["listener_identifier"] =
                    json!({"references":["aws_vpclattice_listener.main.id"]})
            }
            3 => {
                v["resource_changes"][9]["change"]["after_unknown"] =
                    json!({"service_identifier":true})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 10, "Rule listener"),
            case == 0,
            "parent {case}"
        );
    }
}

#[test]
fn lattice_targets_require_group_type_and_never_reconstruct_ip_targets() {
    for (kind, source, output, valid) in [
        ("INSTANCE", "aws_instance", "id", true),
        ("LAMBDA", "aws_lambda_function", "arn", true),
        ("ALB", "aws_lb", "arn", true),
        ("IP", "aws_instance", "id", false),
        ("LAMBDA", "aws_instance", "id", false),
        ("INSTANCE", "aws_lambda_function", "arn", false),
    ] {
        let mut v = fixture();
        v["resource_changes"][5]["change"]["after"]["type"] = json!(kind);
        v["resource_changes"][7]["type"] = json!(source);
        v["resource_changes"][7]["change"]["after"] =
            json!({output:"endpoint", "load_balancer_type":"application"});
        v["resource_changes"][8]["change"]["after"]["target"][0]["id"] = json!("endpoint");
        v["configuration"]["root_module"]["resources"][8]["expressions"]["target"][0]["id"] =
            json!({"references":[format!("aws_instance.main.{output}")]});
        assert_eq!(
            complete(&v, 8, "Configured Lattice target"),
            valid,
            "{kind}: {source}"
        );
        if kind == "ALB" {
            v["resource_changes"][7]["change"]["after"]["load_balancer_type"] = json!("network");
            assert!(!complete(&v, 8, "Configured Lattice target"));
        }
    }
    for case in 0..5 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][5]["change"]["after_unknown"] = json!({"type":true}),
            2 => {
                v["resource_changes"][8]["change"]["after_sensitive"] =
                    json!({"target":[{"id":true}]})
            }
            3 => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            4 => v["resource_changes"][7]["provider_name"] = json!("example.com/foreign/aws"),
            _ => {}
        }
        assert_eq!(
            complete(&v, 8, "Configured Lattice target"),
            case == 0,
            "{case}"
        );
    }
}

#[test]
fn lattice_vpc_and_group_proof_requires_complete_common_vpc() {
    for case in 0..9 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            2 => {
                v["resource_changes"][12]["change"]["after"]["security_group_ids"] =
                    json!(["sg-main", "literal"])
            }
            3 => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"security_group_ids":true})
            }
            4 => v["resource_changes"][2]["change"]["after_sensitive"] = json!({"id":true}),
            5 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            6 => v["resource_changes"][1]["deposed"] = json!("old"),
            7 => {
                v["resource_changes"][12]["change"]["after"]["security_group_ids"] = json!([]);
                v["configuration"]["root_module"]["resources"][12]["expressions"]["security_group_ids"] =
                    json!({"constant_value":[]});
            }
            8 => v["resource_changes"][12]["change"]["after"]["security_group_ids"] = json!([]),
            _ => {}
        }
        assert_eq!(
            complete(&v, 12, "Associated VPC"),
            case == 0 || case == 7,
            "{case}"
        );
        assert_eq!(
            complete(&v, 12, "Association security groups"),
            case == 0,
            "{case}"
        );
    }
    let mut v = fixture();
    assert!(complete(&v, 5, "Target group VPC"));
    v["resource_changes"][5]["change"]["after"]["type"] = json!("LAMBDA");
    assert!(!complete(&v, 5, "Target group VPC"));
}

#[test]
fn lattice_logs_and_policies_respect_variants_and_output_types() {
    for case in 0..7 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][16]["change"]["after_unknown"] =
                    json!({"service_network_log_type":true})
            }
            2 => {
                v["resource_changes"][16]["change"]["after"]["service_network_log_type"] =
                    json!("invalid")
            }
            3 => v["resource_changes"][15]["type"] = json!("aws_kinesis_stream"),
            4 => v["resource_changes"][15]["change"]["after_sensitive"] = json!({"arn":true}),
            5 => {
                v["configuration"]["root_module"]["resources"][16]["expressions"]["destination_arn"] =
                    json!({"references":["aws_cloudwatch_log_group.main.name"]})
            }
            6 => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        assert_eq!(
            complete(&v, 16, "Configured log destination"),
            case == 0,
            "{case}"
        );
    }
    for t in [13, 14] {
        let mut v = fixture();
        let field = if t == 13 {
            "resource_identifier"
        } else {
            "resource_arn"
        };
        let source = if t == 13 {
            "aws_vpclattice_service.main.id"
        } else {
            "aws_vpclattice_service_network.main.id"
        };
        v["configuration"]["root_module"]["resources"][t]["expressions"][field] =
            json!({"references":[source]});
        v["resource_changes"][t]["change"]["after"][field] =
            json!(if t == 13 { "svc-main" } else { "sn-main" });
        assert!(!complete(&v, t, "Lattice policy resource"));
    }
}

#[test]
fn lattice_preserves_shared_identity_and_private_payloads() {
    let p = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 17);
    for kind in ["aws_vpclattice_service", "aws_vpclattice_service_network"] {
        assert!(
            g.nodes
                .iter()
                .filter(|n| n.resource_type == kind)
                .all(
                    |n| n.placement == crate::model::Placement::Spatial && n.member_owner.is_none()
                )
        );
    }
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.member_owner.is_some() && n.resource_type.starts_with("aws_vpclattice_"))
            .count(),
        2
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.to].resource_type.starts_with("aws_vpclattice_"))
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
        assert!(!out.contains("PRIVATE_LATTICE_PAYLOAD"));
    }
}

#[test]
fn lattice_scope_proof_is_rechecked_without_inheriting_accounts() {
    for case in 0..6 {
        let mut v = fixture();
        if case == 1 || case == 2 {
            v["resource_changes"][11]["change"]["after"]["service_network_identifier"] =
                v["resource_changes"][4]["change"]["after"]["arn"].clone();
            v["configuration"]["root_module"]["resources"][11]["expressions"]["service_network_identifier"] =
                json!({"references":["aws_vpclattice_service_network.main.arn"]});
        }
        if case == 4 {
            v["resource_changes"][4]["change"]["after"]["region"] = json!("us-west-2");
            v["configuration"]["root_module"]["resources"][4]["expressions"]["region"] =
                json!({"constant_value":"us-west-2"});
        }
        let mut p = plan::parse(&v.to_string()).unwrap();
        for n in &mut p.nodes {
            let config = n.provider_configuration.as_mut().unwrap();
            config.account = crate::model::AwsAccountScope::new("123456789012");
            if n.address == "aws_vpclattice_service_network.main" {
                let config = n.provider_configuration.as_mut().unwrap();
                if matches!(case, 1..=3) {
                    config.account = crate::model::AwsAccountScope::new("234567890123");
                }
                if case == 2 {
                    config.account_conflict = true;
                }
                if case == 5 {
                    config.account = None;
                }
            }
        }
        let g = semantic::transform(&p);
        let association = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_vpclattice_service_network_service_association.main")
            .unwrap();
        assert_eq!(
            association
                .references
                .iter()
                .any(|r| r.label == "Lattice service network" && r.complete),
            matches!(case, 0 | 1),
            "{case}"
        );
        assert_eq!(
            association
                .entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012"
        );
    }
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.address == "aws_vpclattice_service.main" {
                "234567890123"
            } else {
                "123456789012"
            });
    }
    let g = semantic::transform(&p);
    let rule = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_vpclattice_listener_rule.main")
        .unwrap();
    assert!(
        !rule
            .references
            .iter()
            .any(|r| r.label == "Rule listener" && r.complete)
    );
    assert!(rule.member_owner.is_none());
}

#[test]
fn lattice_logging_owner_variants_and_destination_allowlist_are_exact() {
    for (owner, mode, valid) in [
        ("aws_vpclattice_service_network", "SERVICE", true),
        ("aws_vpclattice_service_network", "RESOURCE", true),
        ("aws_vpclattice_service", "SERVICE", true),
        ("aws_vpclattice_service", "RESOURCE", false),
        ("aws_vpclattice_resource_configuration", "RESOURCE", true),
        ("aws_vpclattice_resource_configuration", "SERVICE", false),
    ] {
        for destination in [
            "aws_cloudwatch_log_group",
            "aws_s3_bucket",
            "aws_kinesis_firehose_delivery_stream",
        ] {
            let mut v = fixture();
            v["resource_changes"][4]["type"] = json!(owner);
            v["resource_changes"][15]["type"] = json!(destination);
            v["resource_changes"][16]["change"]["after"]["service_network_log_type"] = json!(mode);
            assert_eq!(
                complete(&v, 16, "Configured log destination"),
                valid,
                "{owner} {mode} {destination}"
            );
        }
    }
}
