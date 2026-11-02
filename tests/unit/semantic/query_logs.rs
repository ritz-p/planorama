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
fn query_logs_require_current_explicit_typed_known_endpoints() {
    for (kind, field, target, output) in [
        (super::PUBLIC, "zone_id", "aws_route53_zone", "zone_id"),
        (
            super::PUBLIC,
            "cloudwatch_log_group_arn",
            "aws_cloudwatch_log_group",
            "arn",
        ),
        (super::CONFIG, "destination_arn", "aws_s3_bucket", "arn"),
        (
            super::CONFIG,
            "destination_arn",
            "aws_cloudwatch_log_group",
            "arn",
        ),
        (
            super::CONFIG,
            "destination_arn",
            "aws_kinesis_firehose_delivery_stream",
            "arn",
        ),
        (
            super::ASSOCIATION,
            "resolver_query_log_config_id",
            super::CONFIG,
            "id",
        ),
        (super::ASSOCIATION, "resource_id", "aws_vpc", "id"),
    ] {
        let original = fixture(kind, field, target, output);
        assert!(proven(&original, field), "{kind}.{field} -> {target}");
        for case in 0..12 {
            let mut v = original.clone();
            match case {
                0 => v["resource_changes"][1]["change"]["after"][field] = json!("mismatch"),
                1 => v["resource_changes"][1]["change"]["after_unknown"] = json!({field:true}),
                2 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true}),
                3 => v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({}),
                4 => v["resource_changes"][0]["mode"] = json!("data"),
                5 => v["resource_changes"][0]["deposed"] = json!("old"),
                6 => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                7 => v["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
                8 => v["resource_changes"][1]["provider_name"] = json!("example.com/custom/aws"),
                9 => {
                    v["resource_changes"].as_array_mut().unwrap().remove(0);
                }
                10 => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{target}.main[0].{output}"),format!("{target}.main[1].{output}")]})
                }
                _ => {
                    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.missing")
                }
            }
            assert!(!proven(&v, field), "{kind}.{field}: {case}");
        }
    }
}

#[test]
fn public_query_logging_preserves_us_east_1_destination_contract() {
    let mut v = fixture(
        super::PUBLIC,
        "cloudwatch_log_group_arn",
        "aws_cloudwatch_log_group",
        "arn",
    );
    v["configuration"]["provider_config"]["aws"]["expressions"]["region"]["constant_value"] =
        json!("us-west-2");
    assert!(!proven(&v, "cloudwatch_log_group_arn"));
}

#[test]
fn resolver_query_logs_preserve_shared_config_account_and_region() {
    let field = "resolver_query_log_config_id";
    let mut v = fixture(super::ASSOCIATION, field, super::CONFIG, "id");
    v["resource_changes"][0]["change"]["after"]["owner_id"] = json!("111111111111");
    v["resource_changes"][0]["change"]["after"]["share_status"] = json!("SHARED_BY_ME");
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["aws.other"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws.other");
    let verified = |v: &Value| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let g = crate::semantic::transform(&parsed);
        assert_eq!(g.coverage.selected.resources, 2);
        assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
        assert!(
            !g.edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment)
        );
        g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Query log configuration" && r.complete)
    };
    assert!(verified(&v));
    for case in 0..4 {
        let mut invalid = v.clone();
        match case {
            0 => {
                invalid["resource_changes"][0]["change"]["after"]["owner_id"] =
                    json!("333333333333")
            }
            1 => {
                invalid["resource_changes"][0]["change"]["after"]["share_status"] =
                    json!("NOT_SHARED")
            }
            2 => {
                invalid["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"owner_id":true})
            }
            _ => {
                invalid["configuration"]["provider_config"]["aws.other"]["expressions"]["region"]
                    ["constant_value"] = json!("us-west-2")
            }
        }
        assert!(!verified(&invalid), "case {case}");
    }
}
