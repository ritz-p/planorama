use crate::model::EdgeKind;
use crate::semantic;
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
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = crate::semantic::Input::new(&parsed);
    raw.attributes
        .iter()
        .any(|r| r.attribute == field && super::super::service_references::verified(&raw, r))
}

fn peering() -> Value {
    let mut v = input("aws_vpc_peering_connection", "vpc_id", "aws_vpc", "id");
    v["resource_changes"][1]["change"]["after"]["id"] = "pcx-one".into();
    v["resource_changes"][1]["change"]["after"]["peer_vpc_id"] = "vpc-peer".into();
    v["configuration"]["root_module"]["resources"][1]["expressions"]["peer_vpc_id"] =
        json!({"references":["aws_vpc.peer.id"]});
    add(
        &mut v,
        "aws_vpc",
        "peer",
        json!({"id":"vpc-peer"}),
        json!({}),
    );
    v
}

#[test]
fn requester_and_peer_proof_preserves_noncontainment() {
    let v = peering();
    assert!(verified(&v, "vpc_id"));
    assert!(verified(&v, "peer_vpc_id"));
    let g =
        semantic::transform(&crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for case in 0..15 {
        let mut bad = v.clone();
        match case {
            0 => bad["resource_changes"][1]["change"]["after"]["peer_vpc_id"] = "wrong".into(),
            1 => bad["resource_changes"][2]["change"]["after_unknown"] = json!({"id":true}),
            2 => {
                bad["resource_changes"][1]["change"]["after_sensitive"] =
                    json!({"peer_vpc_id":true})
            }
            3 => {
                bad["resource_changes"][1]["change"]["after_unknown"] = json!({"peer_region":true})
            }
            4 => bad["resource_changes"][1]["change"]["after"]["peer_region"] = "us-west-2".into(),
            5 => {
                bad["resource_changes"][1]["change"]["after"]["peer_owner_id"] =
                    "222222222222".into()
            }
            6 => bad["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            7 => bad["resource_changes"][2]["deposed"] = "old".into(),
            9 => bad["resource_changes"][2]["mode"] = "data".into(),
            10 => bad["resource_changes"][2]["provider_name"] = "example.com/custom/aws".into(),
            11 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            12 => bad["resource_changes"][2]["change"]["after_sensitive"] = json!({"id":true}),
            13 => {
                bad["resource_changes"].as_array_mut().unwrap().remove(2);
            }
            8 => {
                bad["configuration"]["root_module"]["resources"][1]["expressions"]["peer_vpc_id"] =
                    json!({"constant_value":"vpc-peer"})
            }
            _ => {
                bad["configuration"]["root_module"]["resources"][1]["expressions"]["peer_vpc_id"] =
                    json!({"references":["aws_vpc.peer.id","var.missing"]})
            }
        }
        assert!(!verified(&bad, "peer_vpc_id"), "case {case}");
    }
}

#[test]
fn scoped_cross_account_peer_and_accepter_orientation() {
    let mut v = peering();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["peer"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["peer"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    v["configuration"]["provider_config"]["peer"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][2]["provider_config_key"] = "peer".into();
    v["resource_changes"][1]["change"]["after"]["peer_owner_id"] = "222222222222".into();
    v["resource_changes"][1]["change"]["after"]["peer_region"] = "us-west-2".into();
    assert!(verified(&v, "peer_vpc_id"));
    let json_only = crate::plan::parse(&v.to_string()).unwrap();
    let raw = crate::semantic::Input::new(&json_only);
    assert!(!raw.attributes.iter().any(
        |r| r.attribute == "peer_vpc_id" && super::super::service_references::verified(&raw, r)
    ));
    add(
        &mut v,
        "aws_vpc_peering_connection_accepter",
        "accept",
        json!({"vpc_peering_connection_id":"pcx-one","vpc_id":"vpc-peer","peer_vpc_id":"endpoint","peer_owner_id":"111111111111","peer_region":"us-west-2"}),
        json!({"vpc_peering_connection_id":{"references":["aws_vpc_peering_connection.main.id"]}}),
    );
    v["configuration"]["root_module"]["resources"][3]["provider_config_key"] = "peer".into();
    assert!(verified(&v, "vpc_peering_connection_id"));
    for case in 0..4 {
        let mut bad = v.clone();
        match case {
            0 => bad["resource_changes"][3]["change"]["after"]["peer_region"] = "us-east-1".into(),
            1 => bad["resource_changes"][3]["change"]["after"]["vpc_id"] = "endpoint".into(),
            2 => {
                bad["resource_changes"][3]["change"]["after"]["peer_owner_id"] =
                    "222222222222".into()
            }
            _ => {
                bad["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
                    "aws".into()
            }
        }
        assert!(!verified(&bad, "vpc_peering_connection_id"), "case {case}");
    }
}

#[test]
fn options_have_side_scopes_and_unique_owner() {
    let mut v = peering();
    add(
        &mut v,
        "aws_vpc_peering_connection_options",
        "options",
        json!({"vpc_peering_connection_id":"pcx-one","requester":[{"allow_remote_vpc_dns_resolution":true}]}),
        json!({"vpc_peering_connection_id":{"references":["aws_vpc_peering_connection.main.id"]},"requester":[{"allow_remote_vpc_dns_resolution":{"constant_value":true}}]}),
    );
    assert!(verified(&v, "vpc_peering_connection_id"));
    let g =
        semantic::transform(&crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_vpc_peering_connection_options")
            .unwrap()
            .member_owner
            .is_some()
    );
    v["resource_changes"][3]["change"]["after_unknown"] =
        json!({"requester":[{"allow_remote_vpc_dns_resolution":true}]});
    assert!(!verified(&v, "vpc_peering_connection_id"));
}

#[test]
fn peering_named_states_and_filtered_helpers_preserve_coverage() {
    let mut v = peering();
    add(
        &mut v,
        "aws_vpc_peering_connection_accepter",
        "accept",
        json!({"vpc_peering_connection_id":"pcx-one"}),
        json!({"vpc_peering_connection_id":{"references":["aws_vpc_peering_connection.main.id"]}}),
    );
    assert!(verified(&v, "vpc_peering_connection_id"));
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(!verified(&filtered, "vpc_peering_connection_id"));
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

#[test]
fn cross_region_options_validate_only_the_configured_side() {
    let mut v = peering();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["peer"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["peer"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][2]["provider_config_key"] = "peer".into();
    v["resource_changes"][1]["change"]["after"]["peer_region"] = "us-west-2".into();
    add(
        &mut v,
        "aws_vpc_peering_connection_options",
        "options",
        json!({"vpc_peering_connection_id":"pcx-one","accepter":[{"allow_remote_vpc_dns_resolution":true}],"requester":[{"allow_remote_vpc_dns_resolution":false}]}),
        json!({"vpc_peering_connection_id":{"references":["aws_vpc_peering_connection.main.id"]},"accepter":[{"allow_remote_vpc_dns_resolution":{"constant_value":true}}]}),
    );
    v["configuration"]["root_module"]["resources"][3]["provider_config_key"] = "peer".into();
    assert!(verified(&v, "vpc_peering_connection_id"));
    v["configuration"]["root_module"]["resources"][3]["expressions"]["requester"] =
        json!([{"allow_remote_vpc_dns_resolution":{"constant_value":false}}]);
    assert!(!verified(&v, "vpc_peering_connection_id"));
}
