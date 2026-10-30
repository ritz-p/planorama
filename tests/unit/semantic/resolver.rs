use crate::plan;
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":"aws_route53_resolver_endpoint.main","type":"aws_route53_resolver_endpoint","change":{"actions":["create"],"after":{"id":"rslvr-out-main","direction":"OUTBOUND"}}},
            {"address":"aws_route53_resolver_rule.main","type":"aws_route53_resolver_rule","change":{"actions":["create"],"after":{"id":"rslvr-rr-main","rule_type":"FORWARD","resolver_endpoint_id":"rslvr-out-main"}}}
        ],
        "configuration":{
            "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
            "root_module":{"resources":[
                {"address":"aws_route53_resolver_endpoint.main","provider_config_key":"aws","expressions":{}},
                {"address":"aws_route53_resolver_rule.main","provider_config_key":"aws","expressions":{"resolver_endpoint_id":{"references":["aws_route53_resolver_endpoint.main.id"]}}}
            ]}
        }
    })
}

fn verified(v: &Value) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == "resolver_endpoint_id" && r.service_reference_verified)
}

fn network_fixture() -> Value {
    let mut v = fixture();
    for (kind, name, values, expressions) in [
        ("aws_vpc", "main", json!({"id":"vpc-main"}), json!({})),
        (
            "aws_subnet",
            "a",
            json!({"id":"subnet-a","vpc_id":"vpc-main"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
        (
            "aws_subnet",
            "b",
            json!({"id":"subnet-b","vpc_id":"vpc-main"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
        (
            "aws_security_group",
            "main",
            json!({"id":"sg-main","vpc_id":"vpc-main"}),
            json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        ),
    ] {
        let address = format!("{kind}.{name}");
        v["resource_changes"].as_array_mut().unwrap().push(
            json!({"address":address,"type":kind,"change":{"actions":["create"],"after":values}}),
        );
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":address,"provider_config_key":"aws","expressions":expressions}));
    }
    v["resource_changes"][0]["change"]["after"] = json!({"id":"rslvr-out-main","direction":"OUTBOUND","host_vpc_id":"vpc-main","ip_address":[{"subnet_id":"subnet-b"},{"subnet_id":"subnet-a"}],"security_group_ids":["sg-main"]});
    v["configuration"]["root_module"]["resources"][0]["expressions"] = json!({"ip_address":[{"subnet_id":{"references":["aws_subnet.a.id"]}},{"subnet_id":{"references":["aws_subnet.b.id"]}}],"security_group_ids":{"references":["aws_security_group.main.id"]}});
    v
}

#[test]
fn endpoint_requires_complete_subnets_and_groups_for_vpc_placement() {
    let mut v = network_fixture();
    let p = plan::parse(&v.to_string()).unwrap();
    for field in ["ip_address.subnet_id", "security_group_ids"] {
        assert!(
            p.attributes
                .iter()
                .any(|r| r.attribute == field && r.service_reference_verified),
            "{field}"
        );
    }
    let g = crate::semantic::transform(&p);
    assert!(
        g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && g.nodes[e.from].address == "aws_vpc.main"
                && g.nodes[e.to].address == "aws_route53_resolver_endpoint.main")
    );
    v["resource_changes"][0]["change"]["after"]["host_vpc_id"] = json!("wrong");
    let p = plan::parse(&v.to_string()).unwrap();
    assert!(
        !p.attributes
            .iter()
            .any(|r| r.attribute == "ip_address.subnet_id" && r.service_reference_verified)
    );
}

#[test]
fn forward_rule_requires_current_outbound_endpoint_and_matching_id() {
    assert!(verified(&fixture()));
    for case in 0..8 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][0]["change"]["after"]["direction"] = json!("INBOUND"),
            1 => v["resource_changes"][1]["change"]["after"]["rule_type"] = json!("SYSTEM"),
            2 => {
                v["resource_changes"][1]["change"]["after"]["resolver_endpoint_id"] = json!("other")
            }
            3 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"direction":true}),
            4 => {
                v["resource_changes"][1]["change"]["after_sensitive"] =
                    json!({"resolver_endpoint_id":true})
            }
            5 => v["resource_changes"][0]["deposed"] = json!("old"),
            6 => v["resource_changes"][0]["mode"] = json!("data"),
            _ => v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({}),
        }
        assert!(!verified(&v), "case {case}");
    }
}

#[test]
fn endpoint_rejects_incomplete_ambiguous_or_incompatible_network_evidence() {
    for case in 0..12 {
        let mut v = network_fixture();
        match case {
            0 => {
                v["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"ip_address":[{}, {"subnet_id":true}]})
            }
            1 => {
                v["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"security_group_ids":true})
            }
            2 => {
                v["resource_changes"][0]["change"]["after"]["ip_address"][1]["subnet_id"] =
                    json!("missing")
            }
            3 => {
                v["configuration"]["root_module"]["resources"][0]["expressions"]["ip_address"][1] =
                    json!({})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][0]["expressions"]["ip_address"][1]
                    ["subnet_id"] = json!({"references":["aws_subnet.a.id", "aws_subnet.b.id"]})
            }
            5 => v["resource_changes"][5]["change"]["after"]["vpc_id"] = json!("other"),
            6 => v["resource_changes"][4]["deposed"] = json!("old"),
            7 => v["resource_changes"][3]["mode"] = json!("data"),
            8 => v["resource_changes"][0]["provider_name"] = json!("example.com/custom/aws"),
            9 => {
                v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws", "full_name":"registry.terraform.io/hashicorp/aws", "expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
                    json!("aws.other");
            }
            10 => v["configuration"]["root_module"]["resources"][5]["expressions"] = json!({}),
            _ => {
                v["resource_changes"].as_array_mut().unwrap().remove(4);
            }
        }
        let p = plan::parse(&v.to_string()).unwrap();
        let g = crate::semantic::transform(&p);
        assert!(
            !g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && g.nodes[e.to].address == "aws_route53_resolver_endpoint.main"),
            "case {case}"
        );
    }
}

#[test]
fn association_preserves_rule_owner_and_vpc_scope() {
    let mut v = network_fixture();
    let address = "aws_route53_resolver_rule_association.main";
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":"aws_route53_resolver_rule_association","change":{"actions":["create"],"after":{"resolver_rule_id":"rslvr-rr-main","vpc_id":"vpc-main"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"provider_config_key":"aws","expressions":{"resolver_rule_id":{"references":["aws_route53_resolver_rule.main.id"]},"vpc_id":{"references":["aws_vpc.main.id"]}}}));
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["resource_changes"][1]["change"]["after"]["owner_id"] = json!("111111111111");
    v["resource_changes"][1]["change"]["after"]["share_status"] = json!("SHARED_BY_ME");
    let proven = |v: &Value| {
        let p = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let raw = crate::semantic::Input::new(&p);
        raw.attributes.iter().any(|r| {
            r.attribute == "resolver_rule_id" && super::super::service_references::verified(&raw, r)
        })
    };
    assert!(proven(&v));
    for case in 0..4 {
        let mut bad = v.clone();
        match case {
            0 => bad["resource_changes"][1]["change"]["after"]["owner_id"] = json!("222222222222"),
            1 => {
                bad["resource_changes"][1]["change"]["after"]["share_status"] =
                    json!("SHARED_WITH_ME")
            }
            2 => bad["resource_changes"][1]["change"]["after_unknown"] = json!({"owner_id":true}),
            _ => bad["resource_changes"][6]["change"]["after"]["resolver_rule_id"] = json!("other"),
        }
        assert!(!proven(&bad), "case {case}");
    }
}

#[test]
fn indexed_endpoints_and_named_states_keep_local_identity() {
    let v = network_fixture();
    let mut indexed: Value =
        serde_json::from_str(&v.to_string().replace("aws_subnet.b", "aws_subnet.b[0]")).unwrap();
    indexed["configuration"]["root_module"]["resources"][4]["address"] = json!("aws_subnet.b");
    let p = plan::parse(&indexed.to_string()).unwrap();
    assert!(
        p.attributes
            .iter()
            .any(|r| r.attribute == "ip_address.subnet_id" && r.service_reference_verified)
    );
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
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
            format: crate::OutputFormat::Html,
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(out.contains("data-state-id=\"one\""));
    assert!(out.contains("data-state-id=\"two\""));
    assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
}
