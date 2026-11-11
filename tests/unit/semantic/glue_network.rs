use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/glue-network-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (8, 1, "physical_connection_requirements.subnet_id", "id"),
    (
        8,
        3,
        "physical_connection_requirements.security_group_id_list",
        "id",
    ),
    (8, 5, "authentication_configuration.kms_key_arn", "arn"),
    (8, 6, "authentication_configuration.secret_arn", "arn"),
    (9, 4, "role_arn", "arn"),
    (9, 7, "security_configuration", "name"),
    (9, 1, "subnet_id", "id"),
    (9, 3, "security_group_ids", "id"),
];
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, target: usize, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        path,
                    )
        })
}
fn parent(v: &Value, target: usize) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == v["resource_changes"][target]["address"]
        })
        .map(|e| g.nodes[e.from].address.clone())
}
fn set(v: &mut Value, path: &str, value: Value) {
    if let Some((block, leaf)) = path.split_once('.') {
        v[block][0][leaf] = value;
    } else {
        v[path] = value;
    }
}
#[test]
fn glue_network_links_exact_outputs_without_relocating_consumers() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{t} {p}");
    }
    for t in [8, 9] {
        assert_eq!(parent(&v, t).as_deref(), Some("aws_vpc.main"));
    }
    assert_eq!(parent(&v, 10), None);
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 11);
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_glue_dev_endpoint")
            .unwrap()
            .placement,
        Placement::Spatial
    );
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
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
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Authentication secret"));
    }
}
#[test]
fn glue_network_rejects_unproven_outputs_bindings_and_lifecycles() {
    for &(t, s, p, output) in REFERENCES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "wrong-type",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "consumer-delete",
            "consumer-deposed",
            "consumer-data",
            "provider",
            "region",
            "account",
            "ambiguous",
        ] {
            let mut v = fixture();
            match case {
                "literal" | "ambiguous" => set(
                    &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                    p,
                    if case == "literal" {
                        json!({"constant_value":"literal"})
                    } else {
                        json!({"references":["var.dynamic"]})
                    },
                ),
                "unknown" => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({p.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                        p,
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "wrong-type" => v["resource_changes"][s]["type"] = json!("aws_glue_job"),
                "mismatch" => {
                    v["resource_changes"][s]["change"]["after"][output] = json!("different")
                }
                "delete" | "consumer-delete" => {
                    v["resource_changes"][if case == "delete" { s } else { t }]["change"]["actions"] =
                        json!(["delete"])
                }
                "deposed" | "consumer-deposed" => {
                    v["resource_changes"][if case == "deposed" { s } else { t }]["deposed"] =
                        json!("old")
                }
                "data" | "consumer-data" => {
                    v["resource_changes"][if case == "data" { s } else { t }]["mode"] =
                        json!("data")
                }
                "provider" => {
                    v["resource_changes"][s]["provider_name"] = json!("example.org/foreign/aws")
                }
                "region" | "account" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}},"allowed_account_ids":{"constant_value":["999999999999"]}}});
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("other");
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn glue_network_needs_complete_common_vpc_evidence() {
    for t in [8, 9] {
        let (subnet, groups) = super::network(if t == 8 {
            "aws_glue_connection"
        } else {
            "aws_glue_dev_endpoint"
        })
        .unwrap();
        for case in [
            "partial",
            "duplicate",
            "empty",
            "unknown-member",
            "scalar-groups",
            "list-subnet",
            "literal-vpc",
            "split-vpc",
            "unknown-vpc",
        ] {
            let mut v = fixture();
            let after = &mut v["resource_changes"][t]["change"]["after"];
            match case {
                "partial" => set(after, groups, json!(["sg-connector", "literal"])),
                "duplicate" => set(after, groups, json!(["sg-connector", "sg-connector"])),
                "empty" => set(after, groups, json!([])),
                "scalar-groups" => set(after, groups, json!("sg-connector")),
                "list-subnet" => set(after, subnet, json!(["subnet-a"])),
                "unknown-member" => {
                    if t == 8 {
                        v["resource_changes"][t]["change"]["after_unknown"] = json!({"physical_connection_requirements":[{"security_group_id_list":[true]}]});
                    } else {
                        v["resource_changes"][t]["change"]["after_unknown"] =
                            json!({"security_group_ids":[true]});
                    }
                }
                "literal-vpc" => {
                    v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
                        json!({"constant_value":"vpc-main"})
                }
                "split-vpc" => {
                    v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other")
                }
                "unknown-vpc" => {
                    v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
                }
                _ => unreachable!(),
            }
            assert_eq!(parent(&v, t), None, "{t} {case}");
            assert_eq!(graph(&v).coverage.selected.resources, 11);
        }
    }
}
#[test]
fn glue_network_shapes_authentication_and_corroboration_are_separate() {
    for block in [
        "physical_connection_requirements",
        "authentication_configuration",
    ] {
        for multiple in [false, true] {
            let mut v = fixture();
            let row = v["resource_changes"][8]["change"]["after"][block][0].clone();
            v["resource_changes"][8]["change"]["after"][block] = if multiple {
                json!([row.clone(), row])
            } else {
                row
            };
            for &(t, _, p, _) in REFERENCES
                .iter()
                .filter(|(t, _, p, _)| *t == 8 && p.starts_with(block))
            {
                assert!(!complete(&v, t, p));
            }
        }
    }
    for mode in [json!(null), json!("FUTURE")] {
        let mut v = fixture();
        v["resource_changes"][8]["change"]["after"]["authentication_configuration"][0]["authentication_type"] =
            mode;
        assert!(!complete(&v, 8, "authentication_configuration.secret_arn"));
        assert_eq!(parent(&v, 8).as_deref(), Some("aws_vpc.main"));
    }
    for case in [
        "mismatched-zone",
        "unknown-zone",
        "mismatched-vpc",
        "unknown-vpc",
    ] {
        let mut v = fixture();
        let t = if case.ends_with("zone") { 8 } else { 9 };
        match case {
            "mismatched-zone" => {
                v["resource_changes"][8]["change"]["after"]["physical_connection_requirements"][0]
                    ["availability_zone"] = json!("us-east-1b")
            }
            "unknown-zone" => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"physical_connection_requirements":[{"availability_zone":true}]})
            }
            "mismatched-vpc" => {
                v["resource_changes"][9]["change"]["after"]["vpc_id"] = json!("other")
            }
            "unknown-vpc" => {
                v["resource_changes"][9]["change"]["after_unknown"] = json!({"vpc_id":true})
            }
            _ => unreachable!(),
        }
        assert_eq!(parent(&v, t), None, "{case}");
        let p = if t == 8 {
            "physical_connection_requirements.subnet_id"
        } else {
            "subnet_id"
        };
        assert!(complete(&v, t, p));
    }
    let mut v = fixture();
    v["resource_changes"][8]["change"]["after"]["physical_connection_requirements"][0]["availability_zone"] =
        Value::Null;
    v["resource_changes"][9]["change"]["after"]["vpc_id"] = Value::Null;
    assert!(parent(&v, 8).is_some());
    assert!(parent(&v, 9).is_some());
}
#[test]
fn glue_network_preserves_filter_and_named_state_coverage() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let g = semantic::transform(&parsed);
    for address in [
        "aws_glue_connection.shared",
        "aws_glue_dev_endpoint.development",
    ] {
        let selected = view::single(
            &g,
            &view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Containment)
        );
    }
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: FIXTURE.into(),
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
    }
}
