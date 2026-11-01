use crate::{plan, semantic};
use serde_json::{Value, json};

fn pair(field: &str, endpoint: &str, output: &str, typ: &str) -> Value {
    json!({"format_version":"1.2", "resource_changes":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{output:"endpoint","availability_mode":"regional"}}},
        {"address":"aws_flow_log.main","type":"aws_flow_log","mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{field:"endpoint","log_destination_type":typ}}}
    ],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_config_key":"aws","expressions":{}},
        {"address":"aws_flow_log.main","type":"aws_flow_log","mode":"managed","provider_config_key":"aws","expressions":{field:{"references":[format!("{endpoint}.peer.{output}")]}}}
    ]}}})
}

fn verified(v: &Value, field: &str) -> bool {
    final_verified(&plan::parse(&v.to_string()).unwrap(), field)
}

fn final_verified(plan: &crate::model::TerraformPlan, field: &str) -> bool {
    let raw = semantic::Input::new(plan);
    raw.attributes
        .iter()
        .any(|r| r.attribute == field && super::super::service_references::verified(&raw, r))
}

const CASES: &[(&str, &str, &str, &str)] = &[
    ("eni_id", "aws_network_interface", "id", "cloud-watch-logs"),
    ("subnet_id", "aws_subnet", "id", "cloud-watch-logs"),
    ("vpc_id", "aws_vpc", "id", "cloud-watch-logs"),
    (
        "regional_nat_gateway_id",
        "aws_nat_gateway",
        "id",
        "cloud-watch-logs",
    ),
    (
        "transit_gateway_id",
        "aws_ec2_transit_gateway",
        "id",
        "cloud-watch-logs",
    ),
    (
        "transit_gateway_attachment_id",
        "aws_ec2_transit_gateway_vpc_attachment",
        "id",
        "cloud-watch-logs",
    ),
    (
        "log_destination",
        "aws_cloudwatch_log_group",
        "arn",
        "cloud-watch-logs",
    ),
    ("log_destination", "aws_s3_bucket", "arn", "s3"),
    (
        "log_destination",
        "aws_kinesis_firehose_delivery_stream",
        "arn",
        "kinesis-data-firehose",
    ),
    ("iam_role_arn", "aws_iam_role", "arn", "cloud-watch-logs"),
    (
        "iam_role_arn",
        "aws_iam_role",
        "arn",
        "kinesis-data-firehose",
    ),
];

#[test]
fn typed_references_require_explicit_complete_current_matching_evidence() {
    for &(field, kind, output, typ) in CASES {
        let v = pair(field, kind, output, typ);
        assert!(verified(&v, field), "{field} {kind}");
        for scenario in 0..19 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"][output] = json!("other"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"][output] = json!(true),
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
                9 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{kind}.missing.{output}")]})
                }
                10 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{kind}.peer.other")]})
                }
                11 => bad["resource_changes"][1]["change"]["after"][field] = json!(["endpoint"]),
                12 => bad["resource_changes"][1]["change"]["after_unknown"] = json!(true),
                13 => bad["resource_changes"][1]["mode"] = json!("data"),
                14 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{kind}.peer.{output}"), format!("{kind}.missing.{output}")]})
                }
                15 => {
                    let mut peer = bad["resource_changes"][0].clone();
                    peer["address"] = json!(format!("{kind}.other"));
                    bad["resource_changes"].as_array_mut().unwrap().push(peer);
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{kind}.peer.{output}"), format!("{kind}.other.{output}")]});
                }
                16 => bad["resource_changes"][0]["change"]["after_sensitive"][output] = json!(true),
                17 => bad["resource_changes"][1]["change"]["after_sensitive"][field] = json!(true),
                _ => bad["resource_changes"][1]["deposed"] = json!("old"),
            }
            assert!(!verified(&bad, field), "{field} {kind} scenario {scenario}");
        }
    }
}

#[test]
fn monitored_owner_requires_exactly_one_known_input_and_regional_nat_mode() {
    for &(field, kind, output, typ) in &CASES[..6] {
        let v = pair(field, kind, output, typ);
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let flow = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_flow_log")
            .unwrap();
        assert!(flow.member_owner.is_some(), "{field}");
        let owner = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == kind)
            .unwrap();
        assert_eq!(flow.placement, owner.placement, "{field}");
        assert_eq!(flow.action, crate::model::Action::Create);
        for other in super::MONITORED.iter().filter(|&&f| f != field) {
            for unknown in [false, true] {
                let mut bad = v.clone();
                if unknown {
                    bad["resource_changes"][1]["change"]["after_unknown"][other] = json!(true);
                } else {
                    bad["resource_changes"][1]["change"]["after"][other] = json!("another");
                }
                assert!(!verified(&bad, field), "{field} {other}");
                let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
                assert!(
                    graph
                        .nodes
                        .iter()
                        .find(|n| n.resource_type == "aws_flow_log")
                        .unwrap()
                        .member_owner
                        .is_none()
                );
            }
        }
    }
    let mut v = pair("regional_nat_gateway_id", "aws_nat_gateway", "id", "s3");
    for mode in [json!("zonal"), Value::Null] {
        v["resource_changes"][0]["change"]["after"]["availability_mode"] = mode;
        assert!(!verified(&v, "regional_nat_gateway_id"));
    }
}

#[test]
fn destination_type_and_exact_bucket_identity_are_required() {
    for &(field, kind, output, typ) in &CASES[6..] {
        let v = pair(field, kind, output, typ);
        for replacement in [json!("invalid"), Value::Null] {
            let mut bad = v.clone();
            bad["resource_changes"][1]["change"]["after"]["log_destination_type"] = replacement;
            assert!(!verified(&bad, field));
        }
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after_unknown"]["log_destination_type"] = json!(true);
        assert!(!verified(&bad, field));
        if field == "iam_role_arn" {
            bad = v.clone();
            bad["resource_changes"][1]["change"]["after"]["log_destination_type"] = json!("s3");
            assert!(!verified(&bad, field));
        }
    }
    let mut v = pair("log_destination", "aws_s3_bucket", "arn", "s3");
    v["resource_changes"][0]["change"]["after"]["arn"] = json!("arn:aws:s3:::logs");
    v["resource_changes"][1]["change"]["after"]["log_destination"] =
        json!("arn:aws:s3:::logs/prefix");
    assert!(!verified(&v, "log_destination"));
    v["resource_changes"][1]["change"]["after"]["log_destination"] = json!("arn:aws:s3:::logs");
    assert!(verified(&v, "log_destination"));
    v["resource_changes"][1]["change"]["after"]["log_destination_type"] = json!("cloud-watch-logs");
    assert!(!verified(&v, "log_destination"));
}

fn account(v: &mut Value, key: &str, id: &str, region: &str) {
    v["configuration"]["provider_config"][key] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{
        "region":{"constant_value":region},
        "assume_role":[{"role_arn":{"constant_value":format!("arn:aws:iam::{id}:role/Test")}}]
    }});
}

fn evidence_verified(v: &Value, field: &str) -> bool {
    final_verified(
        &crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap(),
        field,
    )
}

#[test]
fn cross_account_role_is_tied_to_verified_firehose_destination_account() {
    let mut v = pair(
        "log_destination",
        "aws_kinesis_firehose_delivery_stream",
        "arn",
        "kinesis-data-firehose",
    );
    account(&mut v, "aws", "111111111111", "us-east-1");
    account(&mut v, "dst", "222222222222", "us-east-1");
    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("dst");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_iam_role.delivery","type":"aws_iam_role","mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"arn":"arn:aws:iam::222222222222:role/Delivery"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_iam_role.delivery","type":"aws_iam_role","mode":"managed","provider_config_key":"dst","expressions":{}}));
    v["resource_changes"][1]["change"]["after"]["deliver_cross_account_role"] =
        json!("arn:aws:iam::222222222222:role/Delivery");
    v["configuration"]["root_module"]["resources"][1]["expressions"]["deliver_cross_account_role"] =
        json!({"references":["aws_iam_role.delivery.arn"]});
    assert!(evidence_verified(&v, "log_destination"));
    assert!(evidence_verified(&v, "deliver_cross_account_role"));
    for scenario in 0..8 {
        let mut bad = v.clone();
        match scenario {
            0 => {
                bad["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws")
            }
            1 => {
                account(&mut bad, "third", "333333333333", "us-east-1");
                bad["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("third");
            }
            2 => account(&mut bad, "dst", "222222222222", "us-west-2"),
            3 => {
                bad["resource_changes"][1]["change"]["after"]["log_destination_type"] = json!("s3")
            }
            4 => {
                bad["configuration"]["root_module"]["resources"][1]["expressions"]["log_destination"] =
                    json!({"constant_value":"endpoint"})
            }
            5 => bad["resource_changes"][0]["change"]["after_unknown"]["arn"] = json!(true),
            6 => bad["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => {
                bad["configuration"]["provider_config"]["dst"]["expressions"]["assume_role"] =
                    Value::Null
            }
        }
        assert!(
            !evidence_verified(&bad, "deliver_cross_account_role"),
            "{scenario}"
        );
    }
}

#[test]
fn scope_is_independent_and_rechecked_after_annotation() {
    for &(field, kind, output, typ) in CASES {
        let mut v = pair(field, kind, output, typ);
        account(&mut v, "aws", "111111111111", "us-east-1");
        let mut parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .any(|r| r.attribute == field && r.service_reference_verified)
        );
        parsed
            .nodes
            .iter_mut()
            .find(|n| n.resource_type == kind)
            .unwrap()
            .provider_configuration
            .as_mut()
            .unwrap()
            .account_conflict = true;
        let graph = semantic::transform(&parsed);
        let flow = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_flow_log")
            .unwrap();
        assert!(flow.member_owner.is_none());
        assert!(!flow.references.iter().any(|r| r.attribute == field
            && r.complete
            && r.label == super::label("aws_flow_log", field).unwrap()));
        assert_eq!(
            flow.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "111111111111"
        );
        account(&mut v, "other", "222222222222", "us-east-1");
        v["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("other");
        let cross = field == "log_destination"
            && matches!(
                kind,
                "aws_s3_bucket" | "aws_kinesis_firehose_delivery_stream"
            );
        assert_eq!(evidence_verified(&v, field), cross, "{field} {kind}");
        account(&mut v, "other", "111111111111", "us-west-2");
        assert_eq!(
            evidence_verified(&v, field),
            matches!(kind, "aws_iam_role" | "aws_s3_bucket"),
            "{field} {kind}"
        );
    }
}

#[test]
fn named_states_filtering_and_formats_preserve_coverage_without_payloads() {
    let mut v = pair("vpc_id", "aws_vpc", "id", "s3");
    v["resource_changes"][1]["change"]["after"]["log_format"] =
        json!("PRIVATE_PACKET_PAYLOAD<script>");
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!verified(&filtered, "vpc_id"));
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
        assert!(!out.contains("PRIVATE_PACKET_PAYLOAD"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}

#[test]
fn shared_delivery_services_remain_independent_of_monitored_network_ownership() {
    let mut v = pair("vpc_id", "aws_vpc", "id", "cloud-watch-logs");
    for (kind, field) in [
        ("aws_cloudwatch_log_group", "log_destination"),
        ("aws_iam_role", "iam_role_arn"),
    ] {
        let p = pair(field, kind, "arn", "cloud-watch-logs");
        v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(p["resource_changes"][0].clone());
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(p["configuration"]["root_module"]["resources"][0].clone());
        v["resource_changes"][1]["change"]["after"][field] = json!("endpoint");
        v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
            p["configuration"]["root_module"]["resources"][1]["expressions"][field].clone();
    }
    let mut second = v["resource_changes"][1].clone();
    second["address"] = json!("aws_flow_log.second");
    v["resource_changes"].as_array_mut().unwrap().push(second);
    let mut config = v["configuration"]["root_module"]["resources"][1].clone();
    config["address"] = json!("aws_flow_log.second");
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 5);
    assert_eq!(graph.nodes.len(), 5);
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for node in &graph.nodes {
        assert_eq!(
            node.member_owner.is_some(),
            node.resource_type == "aws_flow_log"
        );
        if node.resource_type == "aws_flow_log" {
            for field in ["vpc_id", "log_destination", "iam_role_arn"] {
                assert!(
                    node.references
                        .iter()
                        .any(|r| r.attribute == field && r.complete && r.targets.len() == 1)
                );
            }
        }
    }
}
