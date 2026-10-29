use crate::{plan, semantic};
use serde_json::{Value, json};

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
}

fn base() -> Value {
    json!({"format_version":"1.2", "resource_changes":[], "configuration":{
        "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
        "root_module":{"resources":[]}}})
}

fn input(kind: &str, field: &str, endpoint: &str, collection: bool) -> Value {
    let mut v = base();
    add(
        &mut v,
        endpoint,
        "peer",
        json!({"id":"endpoint"}),
        json!({}),
    );
    add(
        &mut v,
        kind,
        "main",
        json!({field:if collection {json!(["endpoint"])} else {json!("endpoint")}, "type":"ingress","ip_protocol":"tcp"}),
        json!({field:{"references":[format!("{endpoint}.peer.id")]}}),
    );
    v
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn owner_and_typed_peers_require_explicit_current_matching_endpoints() {
    for (kind, field, endpoint, collection) in [
        (
            "aws_security_group_rule",
            "security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_security_group_rule",
            "source_security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_security_group_rule",
            "prefix_list_ids",
            "aws_ec2_managed_prefix_list",
            true,
        ),
        (
            "aws_vpc_security_group_ingress_rule",
            "security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_vpc_security_group_egress_rule",
            "security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_vpc_security_group_ingress_rule",
            "referenced_security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_vpc_security_group_egress_rule",
            "referenced_security_group_id",
            "aws_security_group",
            false,
        ),
        (
            "aws_vpc_security_group_ingress_rule",
            "prefix_list_id",
            "aws_ec2_managed_prefix_list",
            false,
        ),
        (
            "aws_vpc_security_group_egress_rule",
            "prefix_list_id",
            "aws_ec2_managed_prefix_list",
            false,
        ),
    ] {
        let v = input(kind, field, endpoint, collection);
        assert!(verified(&v, field), "{kind} {field}");
        for scenario in 0..9 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"]["id"] = "different".into(),
                1 => bad["resource_changes"][0]["change"]["after_unknown"]["id"] = true.into(),
                2 => bad["resource_changes"][1]["change"]["after_unknown"][field] = true.into(),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                5 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                6 => bad["resource_changes"][0]["deposed"] = "old".into(),
                7 => {
                    bad["resource_changes"][0]["mode"] = "data".into();
                    bad["configuration"]["root_module"]["resources"][0]["mode"] = "data".into();
                }
                _ => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{endpoint}.peer.name")]})
                }
            }
            assert!(!verified(&bad, field), "{kind} {field} scenario {scenario}");
        }
        if field == "security_group_id" {
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            assert!(graph.nodes[1].member_owner.is_some(), "{kind}");
        }
    }
}

#[test]
fn inline_sets_require_complete_ids_and_do_not_accept_names() {
    for direction in ["ingress", "egress"] {
        for (field, endpoint) in [
            ("security_groups", "aws_security_group"),
            ("prefix_list_ids", "aws_ec2_managed_prefix_list"),
        ] {
            let mut v = input("aws_security_group", "unused", endpoint, true);
            v["resource_changes"][1]["change"]["after"] = json!({direction:[{field:["endpoint"]}]});
            v["configuration"]["root_module"]["resources"][1]["expressions"] =
                json!({direction:[{field:{"references":[format!("{endpoint}.peer.id")]}}]});
            let path = format!("{direction}.{field}");
            assert!(verified(&v, &path), "{path}");
            let mut bad = v.clone();
            bad["resource_changes"][1]["change"]["after"][direction][0][field] =
                json!(["endpoint", "literal"]);
            assert!(!verified(&bad, &path));
            bad = v.clone();
            bad["resource_changes"][1]["change"]["after_unknown"][direction] = true.into();
            assert!(!verified(&bad, &path));
        }
    }
}

#[test]
fn exclusive_sets_keep_rules_owned_by_the_group_and_require_both_complete_directions() {
    let mut v = base();
    add(
        &mut v,
        "aws_security_group",
        "owner",
        json!({"id":"sg-owner"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_vpc_security_group_ingress_rule",
        "rule",
        json!({"id":"sgr-one","security_group_rule_id":"sgr-one","security_group_id":"sg-owner"}),
        json!({"security_group_id":{"references":["aws_security_group.owner.id"]}}),
    );
    add(
        &mut v,
        "aws_vpc_security_group_rules_exclusive",
        "main",
        json!({"security_group_id":"sg-owner", "ingress_rule_ids":["sgr-one"], "egress_rule_ids":[]}),
        json!({"security_group_id":{"references":["aws_security_group.owner.id"]},"ingress_rule_ids":{"references":["aws_vpc_security_group_ingress_rule.rule.id"]},"egress_rule_ids":{"constant_value":[]}}),
    );
    assert!(verified(&v, "ingress_rule_ids"));
    let mut alternate = v.clone();
    alternate["configuration"]["root_module"]["resources"][2]["expressions"]["ingress_rule_ids"] =
        json!({"references":["aws_vpc_security_group_ingress_rule.rule.security_group_rule_id"]});
    assert!(verified(&alternate, "ingress_rule_ids"));
    alternate["resource_changes"][1]["change"]["after"]["security_group_rule_id"] =
        "different".into();
    assert!(!verified(&alternate, "ingress_rule_ids"));
    let mut wrong_direction = v.clone();
    wrong_direction["resource_changes"][2]["change"]["after"]["ingress_rule_ids"] = json!([]);
    wrong_direction["resource_changes"][2]["change"]["after"]["egress_rule_ids"] =
        json!(["sgr-one"]);
    wrong_direction["configuration"]["root_module"]["resources"][2]["expressions"]["ingress_rule_ids"] =
        json!({"constant_value":[]});
    wrong_direction["configuration"]["root_module"]["resources"][2]["expressions"]["egress_rule_ids"] =
        json!({"references":["aws_vpc_security_group_ingress_rule.rule.id"]});
    assert!(!verified(&wrong_direction, "egress_rule_ids"));
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(
        graph.nodes[1].member_owner.as_ref().unwrap().target,
        graph.nodes[2].member_owner.as_ref().unwrap().target
    );
    for scenario in 0..4 {
        let mut bad = v.clone();
        match scenario {
            0 => {
                bad["resource_changes"][1]["change"]["after"]["security_group_id"] = "other".into()
            }
            1 => {
                bad["resource_changes"][2]["change"]["after"]["egress_rule_ids"] =
                    json!(["literal"])
            }
            2 => {
                bad["resource_changes"][2]["change"]["after_unknown"]["egress_rule_ids"] =
                    true.into()
            }
            _ => bad["resource_changes"][2]["change"]["after"]["egress_rule_ids"] = Value::Null,
        }
        assert!(!verified(&bad, "ingress_rule_ids"), "{scenario}");
        let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
        assert!(graph.nodes[2].member_owner.is_none());
    }
    v["resource_changes"][2]["change"]["after"]["ingress_rule_ids"] = json!([]);
    v["configuration"]["root_module"]["resources"][2]["expressions"]["ingress_rule_ids"] =
        json!({"constant_value":[]});
    assert!(
        semantic::transform(&plan::parse(&v.to_string()).unwrap()).nodes[2]
            .member_owner
            .is_some()
    );
}

#[test]
fn additional_vpc_association_preserves_original_containment_and_coverage() {
    let mut v = base();
    add(
        &mut v,
        "aws_vpc",
        "original",
        json!({"id":"vpc-original"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_vpc",
        "additional",
        json!({"id":"vpc-additional"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_security_group",
        "owner",
        json!({"id":"sg-owner","vpc_id":"vpc-original"}),
        json!({"vpc_id":{"references":["aws_vpc.original.id"]}}),
    );
    add(
        &mut v,
        "aws_vpc_security_group_vpc_association",
        "main",
        json!({"security_group_id":"sg-owner","vpc_id":"vpc-additional","state":"PRIVATE_RUNTIME_STATE"}),
        json!({"security_group_id":{"references":["aws_security_group.owner.id"]},"vpc_id":{"references":["aws_vpc.additional.id"]}}),
    );
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.coverage.selected.resources, 4);
    assert!(graph.edges.iter().any(|e| {
        e.kind == crate::model::EdgeKind::Association
            && [
                graph.nodes[e.from].address.as_str(),
                graph.nodes[e.to].address.as_str(),
            ]
            .contains(&"aws_security_group.owner")
            && [
                graph.nodes[e.from].address.as_str(),
                graph.nodes[e.to].address.as_str(),
            ]
            .contains(&"aws_vpc.additional")
    }));
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && graph.nodes[e.from].address == "aws_vpc.original"
                && graph.nodes[e.to].address == "aws_security_group.owner")
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && graph.nodes[e.from].address == "aws_vpc.additional"
                && graph.nodes[e.to].address == "aws_security_group.owner")
    );
    for scenario in 0..3 {
        let mut bad = v.clone();
        match scenario {
            0 => bad["resource_changes"][1]["change"]["after"]["id"] = "mismatch".into(),
            1 => bad["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            _ => {
                bad["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-additional"})
            }
        }
        let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.coverage.selected.resources, 4);
    }
}

#[test]
fn discriminators_ambiguity_and_incompatible_scopes_are_conservative() {
    let v = input(
        "aws_security_group_rule",
        "source_security_group_id",
        "aws_security_group",
        false,
    );
    for typ in [json!("invalid"), Value::Null] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"]["type"] = typ;
        assert!(!verified(&bad, "source_security_group_id"));
    }
    for field in ["self", "cidr_blocks", "ipv6_cidr_blocks"] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"][field] = if field == "self" {
            json!(true)
        } else {
            json!(["10.0.0.0/8"])
        };
        assert!(!verified(&bad, "source_security_group_id"));
    }
    let v = input(
        "aws_vpc_security_group_ingress_rule",
        "referenced_security_group_id",
        "aws_security_group",
        false,
    );
    for field in ["prefix_list_id", "cidr_ipv4", "cidr_ipv6"] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"][field] = "literal".into();
        assert!(!verified(&bad, "referenced_security_group_id"));
    }
    for region in ["us-east-1", "us-west-2"] {
        let mut bad = v.clone();
        bad["configuration"]["provider_config"]["other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":region}}});
        bad["configuration"]["root_module"]["resources"][0]["provider_config_key"] = "other".into();
        assert!(!verified(&bad, "referenced_security_group_id"));
    }
    let mut bad = v.clone();
    add(
        &mut bad,
        "aws_security_group",
        "other",
        json!({"id":"endpoint"}),
        json!({}),
    );
    bad["configuration"]["root_module"]["resources"][1]["expressions"]["referenced_security_group_id"] =
        json!({"references":["aws_security_group.peer.id","aws_security_group.other.id"]});
    assert!(!verified(&bad, "referenced_security_group_id"));
}

#[test]
fn named_states_filtering_and_formats_preserve_identity_without_payloads() {
    let mut v = input(
        "aws_security_group_rule",
        "security_group_id",
        "aws_security_group",
        false,
    );
    v["resource_changes"][1]["change"]["after"]["description"] = "PRIVATE_PAYLOAD".into();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!verified(&filtered, "security_group_id"));
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
