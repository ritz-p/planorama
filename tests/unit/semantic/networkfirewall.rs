use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/networkfirewall-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, target: usize, field: &str) -> bool {
    let address = v["resource_changes"][target]["address"].as_str().unwrap();
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address && n.deposed_key.is_none())
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        field,
                    )
        })
}
fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == "aws_networkfirewall_firewall.main"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn firewall_network_policy_shared_groups_and_logging_are_separate() {
    let v = fixture();
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    for (target, field) in [
        (6, "vpc_id"),
        (6, super::SUBNETS),
        (6, "firewall_policy_arn"),
        (7, "firewall_policy_arn"),
        (5, super::STATEFUL),
        (5, super::STATELESS),
        (8, "firewall_arn"),
    ] {
        assert!(complete(&v, target, field), "{target} {field}");
    }
    for (i, key) in ["bucketName", "logGroup", "deliveryStream"]
        .iter()
        .enumerate()
    {
        assert!(
            complete(
                &v,
                8,
                &format!(
                    "logging_configuration.0.log_destination_config.{i}.log_destination.{key}"
                )
            ),
            "{key}"
        );
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 12);
    for n in &g.nodes {
        if matches!(
            n.resource_type.as_str(),
            "aws_networkfirewall_firewall" | "aws_networkfirewall_logging_configuration"
        ) {
            assert_eq!(n.placement, Placement::Spatial);
        } else if n.resource_type.starts_with("aws_networkfirewall_") {
            assert_eq!(n.placement, Placement::Logical);
        }
    }
    assert!(
        g.nodes
            .iter()
            .find(|n| n.address == "aws_networkfirewall_logging_configuration.main")
            .unwrap()
            .member_owner
            .is_some()
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_networkfirewall_firewall_policy" | "aws_networkfirewall_rule_group"
            ))
            .all(|n| n.member_owner.is_none())
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
        assert!(out.contains("Firewall policy"));
        assert!(out.contains("Logging firewall"));
        assert!(out.contains(">Logging<"));
        assert!(!out.contains("References: unresolved"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:network-firewall"));
    }
}

#[test]
fn firewall_links_require_explicit_current_unique_compatible_evidence() {
    for case in [
        "literal",
        "wrong-output",
        "mismatch",
        "unknown",
        "source-unknown",
        "empty",
        "wrong-type",
        "provider",
        "data",
        "deleted",
        "consumer-deleted",
        "deposed",
        "consumer-deposed",
        "binding",
        "region",
        "missing",
        "ambiguous",
    ] {
        let mut v = fixture();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["firewall_policy_arn"] =
                    json!({"constant_value":"literal"})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["firewall_policy_arn"] =
                    json!({"references":["aws_networkfirewall_firewall_policy.shared.id"]})
            }
            "ambiguous" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["firewall_policy_arn"] =
                    json!({"references":["var.unknown"]})
            }
            "mismatch" | "empty" => {
                v["resource_changes"][6]["change"]["after"]["firewall_policy_arn"] =
                    json!(if case == "empty" { "" } else { "other" })
            }
            "unknown" => v["resource_changes"][6]["change"]["after_unknown"] = json!(true),
            "source-unknown" => {
                v["resource_changes"][5]["change"]["after_unknown"] = json!({"arn":true})
            }
            "wrong-type" => {
                v["resource_changes"][5]["type"] = json!("aws_networkfirewall_rule_group")
            }
            "provider" => {
                v["resource_changes"][5]["provider_name"] = json!("example.test/other/aws")
            }
            "data" => v["resource_changes"][5]["mode"] = json!("data"),
            "deleted" => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            "consumer-deleted" => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][5]["deposed"] = json!("old"),
            "consumer-deposed" => {
                v["resource_changes"][6]["deposed"] = json!("old");
                assert!(
                    graph(&v)
                        .nodes
                        .iter()
                        .find(|n| n.address == "aws_networkfirewall_firewall.main")
                        .unwrap()
                        .references
                        .iter()
                        .all(|r| !r.complete)
                );
                continue;
            }
            "binding" => {
                v["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                    json!("aws.missing")
            }
            "region" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "missing" => {
                v["resource_changes"][5]["address"] =
                    json!("aws_networkfirewall_firewall_policy.other")
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 6, "firewall_policy_arn"), "{case}");
    }
}

#[test]
fn firewall_placement_requires_full_vpc_subnet_proof_and_vpc_variant() {
    for case in [
        "partial",
        "duplicate",
        "unknown",
        "wrong-vpc",
        "literal-parent",
        "missing-vpc",
        "unknown-vpc",
        "tgw",
        "unknown-tgw",
        "az-variant",
        "deleted-subnet",
    ] {
        let mut v = fixture();
        match case {
            "partial" => v["resource_changes"][6]["change"]["after"]["subnet_mapping"]
                .as_array_mut()
                .unwrap()
                .push(json!({"subnet_id":"unreferenced"})),
            "duplicate" => {
                v["resource_changes"][6]["change"]["after"]["subnet_mapping"][1]["subnet_id"] =
                    json!("subnet-a")
            }
            "unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"subnet_mapping":[false,{"subnet_id":true}]})
            }
            "wrong-vpc" => {
                v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other")
            }
            "literal-parent" => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            "missing-vpc" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["vpc_id"] =
                    Value::Null
            }
            "unknown-vpc" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
            }
            "tgw" => {
                v["resource_changes"][6]["change"]["after"]["transit_gateway_id"] = json!("tgw-1")
            }
            "unknown-tgw" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"transit_gateway_id":true})
            }
            "az-variant" => {
                v["resource_changes"][6]["change"]["after"]["availability_zone_mapping"] =
                    json!([{"availability_zone_id":"use1-az1"}])
            }
            "deleted-subnet" => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => unreachable!(),
        }
        assert!(parent(&v).is_none(), "{case}");
        assert!(
            complete(&v, 6, "firewall_policy_arn"),
            "policy is independent: {case}"
        );
        let p = plan::parse(&v.to_string()).unwrap();
        assert!(
            crate::provider::aws::diagnostics::collect(&p)
                .iter()
                .any(|d| d.address == "aws_networkfirewall_firewall.main"
                    && d.attribute == super::SUBNETS)
        );
    }
}

#[test]
fn firewall_rule_groups_require_complete_typed_collections() {
    for case in [
        "wrong-type",
        "unknown-type",
        "unknown-reference",
        "duplicate",
        "literal-extra",
    ] {
        let mut v = fixture();
        match case {
            "wrong-type" => {
                v["resource_changes"][3]["change"]["after"]["type"] = json!("STATELESS")
            }
            "unknown-type" => {
                v["resource_changes"][3]["change"]["after_unknown"] = json!({"type":true})
            }
            "unknown-reference" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"firewall_policy":[{"stateful_rule_group_reference":true}]})
            }
            "duplicate" | "literal-extra" => {
                let mut extra=v["resource_changes"][5]["change"]["after"]["firewall_policy"][0]["stateful_rule_group_reference"][0].clone();
                if case == "literal-extra" {
                    extra["resource_arn"] = json!("literal");
                }
                v["resource_changes"][5]["change"]["after"]["firewall_policy"][0]["stateful_rule_group_reference"].as_array_mut().unwrap().push(extra);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 5, super::STATEFUL), "{case}");
        assert!(complete(&v, 5, super::STATELESS));
    }
}

#[test]
fn firewall_log_destinations_require_typed_map_key_provenance() {
    let path = "logging_configuration.0.log_destination_config.0.log_destination.bucketName";
    for case in [
        "type",
        "unknown",
        "wrong-key",
        "map-only",
        "prefix-reference",
        "literal",
        "wrong-output",
        "mismatch",
        "duplicate",
    ] {
        let mut v = fixture();
        match case {
            "type" => {
                v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]["log_destination_config"]
                    [0]["log_destination_type"] = json!("CloudWatchLogs")
            }
            "unknown" => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"logging_configuration":true})
            }
            "wrong-key" | "prefix-reference" => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"]
                    [0]["log_destination_config"][0]["log_destination"] =
                    json!({"prefix":{"references":["aws_s3_bucket.logs.bucket"]}})
            }
            "map-only" => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"]
                    [0]["log_destination_config"][0]["log_destination"] =
                    json!({"references":["aws_s3_bucket.logs.bucket"]})
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"]
                    [0]["log_destination_config"][0]["log_destination"]["bucketName"] =
                    json!({"constant_value":"PRIVATE_BUCKET"})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"]
                    [0]["log_destination_config"][0]["log_destination"]["bucketName"] =
                    json!({"references":["aws_s3_bucket.logs.arn"]})
            }
            "mismatch" => {
                v["resource_changes"][9]["change"]["after"]["bucket"] = json!("different")
            }
            "duplicate" => {
                let extra = v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]
                    ["log_destination_config"][0]
                    .clone();
                v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]["log_destination_config"].as_array_mut().unwrap().push(extra);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 8, path), "{case}");
        assert!(complete(&v, 8, "firewall_arn"));
    }
    let mut v = fixture();
    v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]["log_destination_config"].as_array_mut().unwrap().reverse();
    assert!(
        complete(&v, 8, path),
        "set ordering must not determine identity"
    );
    let mut v = fixture();
    let config = v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]["log_destination_config"][0].clone();
    v["resource_changes"][8]["change"]["after"]["logging_configuration"][0]["log_destination_config"].as_array_mut().unwrap().push(config);
    let expr =
        v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"]
            [0]["log_destination_config"][0]
            .clone();
    v["configuration"]["root_module"]["resources"][8]["expressions"]["logging_configuration"][0]["log_destination_config"].as_array_mut().unwrap().push(expr);
    v["resource_changes"][8]["change"]["after_unknown"] = json!({"logging_configuration":[{"log_destination_config":[false,false,false,{"log_destination":{"bucketName":true}}]}]});
    assert!(
        !complete(&v, 8, path),
        "unknown competing destination cannot establish uniqueness"
    );
}

#[test]
fn firewall_selection_lifecycle_and_named_states_keep_identity() {
    let mut v = fixture();
    v["resource_changes"][6]["previous_address"] = json!("aws_networkfirewall_firewall.old");
    v["resource_changes"][6]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][6]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][6].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let p = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&p);
    assert_eq!(g.coverage.selected.resources, 13);
    for source in &p.nodes {
        let matches = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].metadata, source.metadata);
        assert_eq!(matches[0].action, source.action);
        assert_eq!(matches[0].previous_address, source.previous_address);
    }
    let selected = view::single(
        &graph(&fixture()),
        &view::Options {
            focus: Some("aws_networkfirewall_firewall.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        selected
            .nodes
            .iter()
            .all(|n| n.resource_type != "aws_networkfirewall_firewall_policy")
    );
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(5);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}

#[test]
fn firewall_retains_verified_account_and_rejects_conflicting_bindings() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for node in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_networkfirewall_"))
    {
        assert_eq!(
            node.entity
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
    v["configuration"]["provider_config"]["other"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["other"]["expressions"]["allowed_account_ids"] =
        json!({"constant_value":["999999999999"]});
    v["configuration"]["root_module"]["resources"][5]["provider_config_key"] = json!("other");
    assert!(!complete(&v, 6, "firewall_policy_arn"));
    let mut missing = fixture();
    missing["configuration"]["root_module"]["resources"][8]["expressions"]["firewall_arn"] =
        json!({"constant_value":"literal"});
    assert!(
        graph(&missing)
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_networkfirewall_logging_configuration")
            .unwrap()
            .member_owner
            .is_none()
    );
}
