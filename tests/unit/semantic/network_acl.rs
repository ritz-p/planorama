use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str) -> Value {
    let collection = field == "subnet_ids";
    let endpoint = if field == "network_acl_id" {
        "aws_network_acl"
    } else {
        "aws_subnet"
    };
    json!({"format_version":"1.2", "resource_changes":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"id":"endpoint","vpc_id":"vpc-one"}}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{field:if collection {json!(["endpoint"])} else {json!("endpoint")},"vpc_id":"vpc-one","rule_number":100,"egress":false,"protocol":"tcp","rule_action":"allow"}}}
    ],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_config_key":"aws","expressions":{}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_config_key":"aws","expressions":{field:{"references":[format!("{endpoint}.peer.id")]}}}
    ]}}})
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn association_lowers_only_a_complete_compatible_pair() {
    let mut v = input("aws_network_acl_association", "network_acl_id");
    v["resource_changes"].as_array_mut().unwrap().push(json!({
        "address":"aws_subnet.subnet","type":"aws_subnet","mode":"managed",
        "provider_name":"registry.terraform.io/hashicorp/aws",
        "change":{"actions":["create"],"after":{"id":"subnet-one","vpc_id":"vpc-one"}}
    }));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({
        "address":"aws_subnet.subnet","type":"aws_subnet","mode":"managed","provider_config_key":"aws","expressions":{}
    }));
    v["resource_changes"][1]["change"]["after"]["subnet_id"] = json!("subnet-one");
    v["configuration"]["root_module"]["resources"][1]["expressions"]["subnet_id"] =
        json!({"references":["aws_subnet.subnet.id"]});
    assert!(verified(&v, "network_acl_id"));
    assert!(verified(&v, "subnet_id"));
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.edges.iter().any(|e| {
        e.kind == crate::model::EdgeKind::Association
            && e.change
                .as_ref()
                .is_some_and(|c| c.address == "aws_network_acl_association.main")
    }));
    for scenario in 0..4 {
        let mut bad = v.clone();
        match scenario {
            0 => bad["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            1 => bad["resource_changes"][0]["change"]["after_unknown"]["vpc_id"] = json!(true),
            2 => bad["resource_changes"][2]["change"]["after"]["id"] = json!("other"),
            _ => {
                bad["configuration"]["root_module"]["resources"][1]["expressions"]["subnet_id"] =
                    json!({"constant_value":"subnet-one"})
            }
        }
        assert!(!verified(&bad, "network_acl_id"));
        let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
        assert_eq!(graph.nodes.len(), 3);
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Association)
        );
    }
}

#[test]
fn acl_references_require_complete_current_matching_id_evidence() {
    for (kind, field) in [
        ("aws_network_acl", "subnet_ids"),
        ("aws_network_acl_rule", "network_acl_id"),
    ] {
        let v = input(kind, field);
        assert!(verified(&v, field));
        for scenario in 0..10 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"]["id"] = json!("other"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"]["id"] = json!(true),
                2 => bad["resource_changes"][1]["change"]["after_unknown"][field] = json!(true),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => bad["resource_changes"][0]["mode"] = json!("data"),
                5 => bad["resource_changes"][0]["deposed"] = json!("old"),
                6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                7 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                8 => {
                    bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.other")
                }
                _ => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":["aws_subnet.missing.id"]})
                }
            }
            assert!(!verified(&bad, field), "{kind}: {scenario}");
        }
    }
}

#[test]
fn subnet_collections_require_complete_values_and_matching_vpc() {
    let v = input("aws_network_acl", "subnet_ids");
    for replacement in [json!(["endpoint", "missing"]), json!("endpoint"), json!([])] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"]["subnet_ids"] = replacement;
        assert!(!verified(&bad, "subnet_ids"));
    }
    let mut bad = v;
    bad["resource_changes"][0]["change"]["after"]["vpc_id"] = json!("other");
    assert!(!verified(&bad, "subnet_ids"));
}

#[test]
fn rule_owner_keeps_identity_and_action_with_conservative_fallback() {
    let v = input("aws_network_acl_rule", "network_acl_id");
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let rule = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_network_acl_rule")
        .unwrap();
    assert!(rule.member_owner.is_some());
    assert_eq!(rule.action, crate::model::Action::Create);
    for (key, value) in [
        ("rule_number", json!(0)),
        ("egress", json!("false")),
        ("protocol", json!("<script>")),
        ("rule_action", json!("invalid")),
    ] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"][key] = value;
        assert!(!verified(&bad, "network_acl_id"));
        let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
        assert!(
            graph
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_network_acl_rule")
                .unwrap()
                .member_owner
                .is_none()
        );
    }
}

#[test]
fn acl_named_states_filtering_and_formats_keep_independent_coverage() {
    let mut v = input("aws_network_acl_rule", "network_acl_id");
    v["resource_changes"][1]["change"]["after"]["cidr_block"] = json!("PRIVATE_PAYLOAD");
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!verified(&filtered, "network_acl_id"));
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 2);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: filtered.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_PAYLOAD"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}

#[test]
fn acl_ambiguity_scope_and_unknown_rule_identity_are_rejected() {
    let v = input("aws_network_acl_rule", "network_acl_id");
    for key in ["rule_number", "egress", "protocol", "rule_action"] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after_unknown"][key] = json!(true);
        assert!(!verified(&bad, "network_acl_id"));
    }
    for region in ["us-east-1", "us-west-2"] {
        let mut bad = v.clone();
        bad["configuration"]["provider_config"]["other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":region}}});
        bad["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("other");
        assert!(!verified(&bad, "network_acl_id"));
    }
    let mut bad = v.clone();
    let mut endpoint = bad["resource_changes"][0].clone();
    endpoint["address"] = json!("aws_network_acl.other");
    bad["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(endpoint);
    let mut config = bad["configuration"]["root_module"]["resources"][0].clone();
    config["address"] = json!("aws_network_acl.other");
    bad["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    bad["configuration"]["root_module"]["resources"][1]["expressions"]["network_acl_id"] =
        json!({"references":["aws_network_acl.peer.id","aws_network_acl.other.id"]});
    assert!(!verified(&bad, "network_acl_id"));
}

#[test]
fn acl_rule_keeps_verified_account_and_rechecks_owner_scope() {
    let mut v = input("aws_network_acl_rule", "network_acl_id");
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let mut parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let rule = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_network_acl_rule")
        .unwrap();
    assert!(rule.member_owner.is_some());
    assert_eq!(
        rule.entity
            .scope
            .as_ref()
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .as_str(),
        "123456789012"
    );
    parsed
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_network_acl")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account_conflict = true;
    let graph = semantic::transform(&parsed);
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_network_acl_rule")
            .unwrap()
            .member_owner
            .is_none()
    );
}
