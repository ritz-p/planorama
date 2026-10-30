use crate::model::EdgeKind;
use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str, endpoint: &str, output: &str) -> Value {
    let mut v = json!({"format_version":"1.2", "resource_changes":[], "configuration":{
        "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
        "root_module":{"resources":[]}}});
    add(
        &mut v,
        endpoint,
        "endpoint",
        json!({output:"endpoint", "domain":"vpc", "availability_mode":"zonal", "connectivity_type":"public"}),
        json!({}),
    );
    add(
        &mut v,
        kind,
        "main",
        json!({field:"endpoint", "domain":"vpc", "availability_mode":"zonal", "connectivity_type":"public"}),
        json!({field:{"references":[format!("{endpoint}.endpoint.{output}")]}}),
    );
    v
}

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

fn mapping(kind: &str) -> Value {
    let mut v = input(kind, "name", "aws_subnet", "id");
    v["resource_changes"][0]["change"]["after"] = json!({"id":"subnet-a","vpc_id":"vpc-one"});
    v["resource_changes"][1]["change"]["after"] = json!({"load_balancer_type":"network","internal":false,"subnet_mapping":[{"subnet_id":"subnet-b","allocation_id":"eip-b"},{"subnet_id":"subnet-a","allocation_id":"eip-a"}]});
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({"subnet_mapping":[{"subnet_id":{"references":["aws_subnet.endpoint.id"]},"allocation_id":{"references":["aws_eip.a.allocation_id"]}},{"subnet_id":{"references":["aws_subnet.b.id"]},"allocation_id":{"references":["aws_eip.b.allocation_id"]}}]});
    add(
        &mut v,
        "aws_subnet",
        "b",
        json!({"id":"subnet-b","vpc_id":"vpc-one"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_eip",
        "a",
        json!({"allocation_id":"eip-a","domain":"vpc"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_eip",
        "b",
        json!({"allocation_id":"eip-b","domain":"vpc"}),
        json!({}),
    );
    v
}

#[test]
fn mapping_sets_match_each_configured_pair_even_when_reordered() {
    for kind in ["aws_lb", "aws_alb"] {
        let v = mapping(kind);
        assert!(verified(&v, "subnet_mapping.subnet_id"));
        assert!(verified(&v, "subnet_mapping.allocation_id"));
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Connection));
        assert_eq!(g.coverage.selected.resources, 5);
        let mut optional = v.clone();
        optional["resource_changes"][1]["change"]["after"]["subnet_mapping"][0]
            .as_object_mut()
            .unwrap()
            .remove("allocation_id");
        optional["configuration"]["root_module"]["resources"][1]["expressions"]["subnet_mapping"]
            [1]
        .as_object_mut()
        .unwrap()
        .remove("allocation_id");
        assert!(verified(&optional, "subnet_mapping.allocation_id"));
        let graph = semantic::transform(&plan::parse(&optional.to_string()).unwrap());
        assert!(graph.nodes.iter().any(|n| {
            n.references
                .iter()
                .any(|r| r.label == "Mapped Elastic IPs" && r.complete)
        }));
        optional["resource_changes"][1]["change"]["after_unknown"] =
            json!({"subnet_mapping":[{"allocation_id":true},{}]});
        assert!(!verified(&optional, "subnet_mapping.allocation_id"));
        for case in 0..9 {
            let mut bad = v.clone();
            match case {
                0 => {
                    bad["resource_changes"][1]["change"]["after"]["subnet_mapping"][0]["allocation_id"] =
                        "eip-a".into()
                }
                1 => {
                    bad["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"subnet_mapping":[{"allocation_id":true},{}]})
                }
                2 => {
                    bad["resource_changes"][4]["change"]["after_sensitive"] =
                        json!({"allocation_id":true})
                }
                3 => bad["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
                4 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["subnet_mapping"]
                        [0]["allocation_id"] = json!({"constant_value":"eip-a"})
                }
                5 => bad["resource_changes"][1]["change"]["after"]["internal"] = true.into(),
                6 => {
                    bad["resource_changes"][1]["change"]["after"]["load_balancer_type"] =
                        "application".into()
                }
                7 => bad["resource_changes"][1]["change"]["after"]["subnet_mapping"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"subnet_id":"unproven","allocation_id":"unknown"})),
                _ => bad["resource_changes"][4]["deposed"] = "old".into(),
            }
            assert!(
                !verified(&bad, "subnet_mapping.allocation_id"),
                "{kind} case {case}"
            );
        }
        for case in 0..5 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][2]["change"]["after"]["vpc_id"] = "other".into(),
                1 => {
                    bad["resource_changes"][1]["change"]["after_unknown"] =
                        json!({"subnet_mapping":[{"subnet_id":true},{}]})
                }
                2 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["subnets"] =
                        json!({"references":["aws_subnet.b.id"]})
                }
                3 => bad["resource_changes"][2]["mode"] = "data".into(),
                _ => bad["resource_changes"][2]["provider_name"] = "example.com/custom/aws".into(),
            }
            assert!(
                !verified(&bad, "subnet_mapping.subnet_id"),
                "{kind} case {case}"
            );
        }
    }
}

#[test]
fn pool_and_enabled_log_buckets_have_exact_outputs_and_type_gates() {
    for kind in ["aws_lb", "aws_alb"] {
        for (block, field, target, output) in [
            ("ipam_pools", "ipv4_ipam_pool_id", "aws_vpc_ipam_pool", "id"),
            ("access_logs", "bucket", "aws_s3_bucket", "bucket"),
            ("connection_logs", "bucket", "aws_s3_bucket", "id"),
            ("health_check_logs", "bucket", "aws_s3_bucket", "bucket"),
        ] {
            let mut v = input(kind, "name", target, output);
            v["resource_changes"][1]["change"]["after"] = json!({"load_balancer_type":"application",block:[{field:"endpoint","enabled":true}]});
            v["configuration"]["root_module"]["resources"][1]["expressions"] =
                json!({block:[{field:{"references":[format!("{target}.endpoint.{output}")]}}]});
            let path = format!("{block}.{field}");
            assert!(verified(&v, &path), "{kind} {path}");
            for case in 0..8 {
                let mut bad = v.clone();
                match case {
                    0 => {
                        bad["resource_changes"][1]["change"]["after"][block][0][field] =
                            "mismatch".into()
                    }
                    1 => {
                        bad["resource_changes"][1]["change"]["after_unknown"] =
                            json!({block:[{field:true}]})
                    }
                    2 => {
                        bad["resource_changes"][0]["change"]["after_sensitive"] =
                            json!({output:true})
                    }
                    3 => {
                        bad["resource_changes"][1]["change"]["after"]["load_balancer_type"] =
                            "gateway".into()
                    }
                    4 => bad["resource_changes"][1]["change"]["after"][block]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({field:"endpoint","enabled":true})),
                    5 => {
                        bad["configuration"]["root_module"]["resources"][1]["expressions"][block] =
                            json!([{field:{"constant_value":"endpoint"}}])
                    }
                    6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                    _ => bad["resource_changes"][0]["deposed"] = "old".into(),
                }
                assert!(!verified(&bad, &path), "{kind} {path} case {case}");
            }
            if block != "ipam_pools" {
                v["resource_changes"][1]["change"]["after"][block][0]["enabled"] = false.into();
                assert!(!verified(&v, &path));
            }
        }
    }
}

#[test]
fn mapping_scope_and_named_filtering_preserve_independent_resources() {
    let v = mapping("aws_lb");
    let mut foreign = v.clone();
    foreign["configuration"]["provider_config"]["other"] =
        foreign["configuration"]["provider_config"]["aws"].clone();
    foreign["configuration"]["provider_config"]["other"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    foreign["configuration"]["root_module"]["resources"][2]["provider_config_key"] = "other".into();
    assert!(!verified(&foreign, "subnet_mapping.subnet_id"));
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
    assert!(!verified(&filtered, "subnet_mapping.allocation_id"));
    assert!(verified(&filtered, "subnet_mapping.subnet_id"));
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
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
