use serde_json::{Value, json};

fn fixture(kind: &str, field: &str, target: &str, output: &str) -> Value {
    let address = format!("{kind}.main");
    let source = format!("{target}.main[0]");
    json!({"format_version":"1.2", "resource_changes":[
        {"address":source,"type":target,"change":{"actions":["create"],"after":{output:"known"}}},
        {"address":address,"type":kind,"change":{"actions":["create"],"after":{field:"known"}}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{target}.main"),"provider_config_key":"aws","expressions":{}},
        {"address":address,"provider_config_key":"aws","expressions":{field:{"references":[format!("{source}.{output}")]}}}
    ]}}})
}

fn proven(v: &Value, field: &str) -> bool {
    let graph = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    graph.nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            Some(r.label) == super::label(&n.resource_type, field)
                && r.attribute == field
                && r.complete
                && !r.dependency
                && !r.targets.is_empty()
        })
    })
}

#[test]
fn profiles_require_exact_current_typed_endpoint_outputs() {
    for (kind, field, target, output) in [
        (
            super::VPC,
            "profile_id",
            "aws_route53profiles_profile",
            "id",
        ),
        (
            super::RESOURCE,
            "profile_id",
            "aws_route53profiles_profile",
            "id",
        ),
        (super::VPC, "resource_id", "aws_vpc", "id"),
        (super::RESOURCE, "resource_arn", "aws_route53_zone", "arn"),
        (
            super::RESOURCE,
            "resource_arn",
            "aws_route53_resolver_rule",
            "arn",
        ),
        (
            super::RESOURCE,
            "resource_arn",
            "aws_route53_resolver_firewall_rule_group",
            "arn",
        ),
    ] {
        let mut original = fixture(kind, field, target, output);
        if target == "aws_route53_zone" {
            original["resource_changes"][0]["change"]["after"]["vpc"] =
                json!([{"vpc_id":"vpc-private"}]);
        }
        assert!(proven(&original, field), "{kind}.{field} -> {target}");
        for case in 0..9 {
            let mut v = original.clone();
            match case {
                0 => v["resource_changes"][1]["change"]["after"][field] = json!("wrong"),
                1 => v["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                2 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true}),
                3 => v["resource_changes"][0]["mode"] = json!("data"),
                4 => v["resource_changes"][0]["deposed"] = json!("old"),
                5 => {
                    v["resource_changes"][0]["type"] =
                        json!("aws_route53_resolver_query_log_config")
                }
                6 => v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({}),
                7 => v["resource_changes"][1]["provider_name"] = json!("example.com/custom/aws"),
                _ => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            }
            assert!(!proven(&v, field), "{target} case {case}");
        }
    }
}

#[test]
fn public_zones_and_opaque_properties_do_not_prove_profile_resources() {
    let mut v = fixture(super::RESOURCE, "resource_arn", "aws_route53_zone", "arn");
    v["resource_changes"][1]["change"]["after"]["resource_properties"] =
        json!("{\"vpc\":\"vpc-private\"}");
    assert!(!proven(&v, "resource_arn"));
    v["resource_changes"][0]["change"]["after"]["vpc"] = json!([]);
    assert!(!proven(&v, "resource_arn"));
    v["resource_changes"][0]["change"]["after"]["vpc"] = json!([{"vpc_id":"vpc-private"}]);
    v["resource_changes"][0]["change"]["after_unknown"] = json!({"vpc":true});
    assert!(!proven(&v, "resource_arn"));
}
