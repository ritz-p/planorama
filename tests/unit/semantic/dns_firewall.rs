use serde_json::{Value, json};

fn fixture(association: bool) -> Value {
    let kind = if association {
        "aws_route53_resolver_firewall_rule_group_association"
    } else {
        "aws_route53_resolver_firewall_rule"
    };
    let endpoint = if association {
        "aws_vpc"
    } else {
        "aws_route53_resolver_firewall_domain_list"
    };
    let field = if association {
        "vpc_id"
    } else {
        "firewall_domain_list_id"
    };
    json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_route53_resolver_firewall_rule_group.main[0]","type":"aws_route53_resolver_firewall_rule_group","change":{"actions":["create"],"after":{"id":"group"}}},
        {"address":format!("{endpoint}.main[0]"),"type":endpoint,"change":{"actions":["create"],"after":{"id":"endpoint"}}},
        {"address":format!("{kind}.main"),"type":kind,"change":{"actions":["create"],"after":{"firewall_rule_group_id":"group",field:"endpoint"}}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":"aws_route53_resolver_firewall_rule_group.main","provider_config_key":"aws","expressions":{}},
        {"address":format!("{endpoint}.main"),"provider_config_key":"aws","expressions":{}},
        {"address":format!("{kind}.main"),"provider_config_key":"aws","expressions":{"firewall_rule_group_id":{"references":["aws_route53_resolver_firewall_rule_group.main[0].id"]},field:{"references":[format!("{endpoint}.main[0].id")]}}}
    ]}}})
}

fn links(v: &Value) -> usize {
    let g = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 3);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    g.nodes
        .iter()
        .flat_map(|n| &n.references)
        .filter(|r| r.label.starts_with("DNS Firewall") && r.complete)
        .count()
}

#[test]
fn firewall_rules_and_associations_use_independent_typed_ids() {
    for association in [false, true] {
        let v = fixture(association);
        assert_eq!(links(&v), 2);
        for case in 0..9 {
            let mut bad = v.clone();
            match case {
                0 => {
                    bad["resource_changes"][2]["change"]["after"]["firewall_rule_group_id"] =
                        json!("other")
                }
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
                2 => bad["resource_changes"][0]["change"]["after_sensitive"] = json!({"id":true}),
                3 => bad["resource_changes"][0]["mode"] = json!("data"),
                4 => bad["resource_changes"][0]["deposed"] = json!("old"),
                5 => bad["resource_changes"][0]["type"] = json!("aws_vpc"),
                6 => {
                    bad["configuration"]["root_module"]["resources"][2]["expressions"]["firewall_rule_group_id"] =
                        json!({"constant_value":"group"})
                }
                7 => bad["resource_changes"][0]["provider_name"] = json!("example.com/custom/aws"),
                _ => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            }
            assert_eq!(links(&bad), 1, "association {association}, case {case}");
        }
    }
}

#[test]
fn advanced_rule_never_acquires_a_domain_list_or_computed_endpoint() {
    for field in [
        "dns_threat_protection",
        "confidence_threshold",
        "firewall_threat_protection_id",
    ] {
        let mut v = fixture(false);
        v["resource_changes"][2]["change"]["after"][field] = json!("advanced");
        assert_eq!(links(&v), 1);
        v["resource_changes"][2]["change"]["after_unknown"] = json!({field:true});
        assert_eq!(links(&v), 1);
    }
}
