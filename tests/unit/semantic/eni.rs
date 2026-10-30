use crate::model::{EdgeKind, Placement};
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

#[test]
fn explicit_eni_references_require_current_complete_known_evidence() {
    for (kind, field, target) in [
        ("aws_network_interface", "subnet_id", "aws_subnet"),
        (
            "aws_ec2_instance_connect_endpoint",
            "subnet_id",
            "aws_subnet",
        ),
        (
            "aws_network_interface_permission",
            "network_interface_id",
            "aws_network_interface",
        ),
    ] {
        let v = input(kind, field, target, "id");
        assert!(verified(&v, field), "{kind}");
        for case in 0..13 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][1]["change"]["after"][field] = "other".into(),
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
                2 => bad["resource_changes"][1]["change"]["after_sensitive"] = json!({field:true}),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{target}.endpoint.id"),"var.missing"]})
                }
                5 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => bad["resource_changes"][0]["deposed"] = "old".into(),
                7 => bad["resource_changes"][0]["mode"] = "data".into(),
                8 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                9 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{target}.endpoint.attachment_id")]})
                }
                10 => bad["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
                11 => bad["resource_changes"][0]["change"]["after_sensitive"] = json!({"id":true}),
                _ => {
                    bad["resource_changes"].as_array_mut().unwrap().remove(0);
                }
            }
            assert!(!verified(&bad, field), "{kind} case {case}");
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        if field == "subnet_id" {
            assert_eq!(g.nodes[1].placement, Placement::Spatial);
            assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
        } else {
            assert!(g.nodes[1].member_owner.is_some());
        }
        assert_eq!(g.coverage.selected.resources, 2);
    }
}

#[test]
fn security_groups_need_complete_collection_and_subnet_vpc() {
    for (kind, field) in [
        ("aws_network_interface", "security_groups"),
        ("aws_ec2_instance_connect_endpoint", "security_group_ids"),
    ] {
        let mut v = input(kind, "subnet_id", "aws_subnet", "id");
        v["resource_changes"][0]["change"]["after"]["vpc_id"] = "vpc-one".into();
        add(
            &mut v,
            "aws_security_group",
            "sg",
            json!({"id":"sg-one","vpc_id":"vpc-one"}),
            json!({}),
        );
        v["resource_changes"][1]["change"]["after"][field] = json!(["sg-one"]);
        v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
            json!({"references":["aws_security_group.sg.id"]});
        assert!(verified(&v, field), "{kind}");
        for case in 0..5 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][2]["change"]["after"]["vpc_id"] = "vpc-two".into(),
                1 => {
                    bad["resource_changes"][1]["change"]["after"][field] =
                        json!(["sg-one", "sg-two"])
                }
                2 => bad["resource_changes"][1]["change"]["after_unknown"] = json!({field:[true]}),
                3 => bad["resource_changes"][0]["change"]["after"]["vpc_id"] = Value::Null,
                _ => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["subnet_id"] =
                        json!({"constant_value":"endpoint"})
                }
            }
            assert!(!verified(&bad, field), "{kind} case {case}");
        }
    }
}

#[test]
fn attachment_pairs_and_inline_set_do_not_embed_or_relocate_eni() {
    let mut v = input("aws_network_interface", "subnet_id", "aws_subnet", "id");
    v["resource_changes"][1]["change"]["after"]["id"] = "eni-one".into();
    add(
        &mut v,
        "aws_instance",
        "instance",
        json!({"id":"i-one"}),
        json!({}),
    );
    v["resource_changes"][1]["change"]["after"]["attachment"] =
        json!([{"instance":"i-one","device_index":1,"network_card_index":0}]);
    v["configuration"]["root_module"]["resources"][1]["expressions"]["attachment"] =
        json!([{"instance":{"references":["aws_instance.instance.id"]}}]);
    assert!(verified(&v, "attachment.instance"));
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(g.nodes[1].member_owner.is_none());
    assert!(
        g.edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Containment)
            .all(|e| g.nodes[e.from].address == "aws_subnet.endpoint"
                && g.nodes[e.to].address == "aws_network_interface.main")
    );
    for case in 0..4 {
        let mut bad = v.clone();
        match case {
            0 => {
                bad["resource_changes"][1]["change"]["after"]["attachment"][0]["device_index"] =
                    (-1).into()
            }
            1 => {
                bad["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"attachment":[{"device_index":true}]})
            }
            2 => bad["resource_changes"][1]["change"]["after"]["attachment"]
                .as_array_mut()
                .unwrap()
                .push(json!({"instance":"i-other","device_index":2})),
            _ => {
                bad["resource_changes"][1]["change"]["after"]["attachment"][0]["instance"] =
                    "i-other".into()
            }
        }
        assert!(!verified(&bad, "attachment.instance"), "case {case}");
    }
    v["resource_changes"][1]["change"]["after"]["attachment"] = Value::Null;
    v["configuration"]["root_module"]["resources"][1]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("attachment");
    add(
        &mut v,
        "aws_network_interface_attachment",
        "link",
        json!({"instance_id":"i-one", "network_interface_id":"eni-one","device_index":1}),
        json!({"instance_id":{"references":["aws_instance.instance.id"]},"network_interface_id":{"references":["aws_network_interface.main.id"]}}),
    );
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Association));
    assert_eq!(g.coverage.selected.resources, 4);
    v["resource_changes"][3]["change"]["after"]["instance_id"] = "different".into();
    assert!(!verified(&v, "network_interface_id"));
}

#[test]
fn eni_scope_filtering_and_named_rendering_remain_independent() {
    let mut v = input(
        "aws_network_interface_permission",
        "network_interface_id",
        "aws_network_interface",
        "id",
    );
    v["resource_changes"][1]["change"]["after"]["aws_account_id"] = "999999999999".into();
    v["resource_changes"][1]["change"]["after"]["permission"] =
        "<script>private-permission</script>".into();
    assert!(verified(&v, "network_interface_id"));
    let mut other = v.clone();
    other["configuration"]["provider_config"]["other"] =
        other["configuration"]["provider_config"]["aws"].clone();
    other["configuration"]["provider_config"]["other"]["expressions"]["region"]["constant_value"] =
        "us-west-2".into();
    other["configuration"]["root_module"]["resources"][0]["provider_config_key"] = "other".into();
    assert!(!verified(&other, "network_interface_id"));
    other["resource_changes"].as_array_mut().unwrap().remove(0);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: other.to_string(),
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
        assert!(!out.contains("private-permission"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

#[test]
fn separate_sg_attachment_checks_pair_and_vpc_without_inline_sg() {
    let mut v = input("aws_network_interface", "subnet_id", "aws_subnet", "id");
    v["resource_changes"][0]["change"]["after"]["vpc_id"] = "vpc-one".into();
    v["resource_changes"][1]["change"]["after"]["id"] = "eni-one".into();
    add(
        &mut v,
        "aws_security_group",
        "sg",
        json!({"id":"sg-one","vpc_id":"vpc-one"}),
        json!({}),
    );
    add(
        &mut v,
        "aws_network_interface_sg_attachment",
        "link",
        json!({"network_interface_id":"eni-one","security_group_id":"sg-one"}),
        json!({"network_interface_id":{"references":["aws_network_interface.main.id"]},"security_group_id":{"references":["aws_security_group.sg.id"]}}),
    );
    assert!(verified(&v, "network_interface_id"));
    assert!(verified(&v, "security_group_id"));
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Association));
    assert_eq!(g.coverage.selected.resources, 4);
    v["resource_changes"][2]["change"]["after"]["vpc_id"] = "vpc-two".into();
    assert!(!verified(&v, "network_interface_id"));
    assert!(!verified(&v, "security_group_id"));
}
