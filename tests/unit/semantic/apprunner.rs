use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/apprunner-plan.json");
const EGRESS: &str = "network_configuration.egress_configuration.vpc_connector_arn";

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn complete(v: &Value, index: usize, field: &str) -> bool {
    let address = v["resource_changes"][index]["address"].as_str().unwrap();
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
                && Some(r.label) == super::label(address.split('.').next().unwrap(), field)
        })
}

fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].resource_type == "aws_apprunner_vpc_connector"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

fn egress<'a>(v: &'a mut Value, section: &str) -> &'a mut Value {
    &mut v["resource_changes"][5]["change"][section]["network_configuration"][0]["egress_configuration"]
        [0]
}

#[test]
fn apprunner_separates_shared_egress_and_ingress_from_service_placement() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(complete(&v, 5, EGRESS));
    assert!(complete(&v, 6, EGRESS));
    assert!(complete(&v, 7, "service_arn"));
    assert!(complete(&v, 4, "subnets"));
    assert!(complete(&v, 4, "security_groups"));
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 10);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for node in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_apprunner_"))
    {
        assert_eq!(
            node.placement,
            if node.resource_type == "aws_apprunner_service" {
                Placement::Spatial
            } else {
                Placement::Logical
            }
        );
    }
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && matches!(
            g.nodes[e.to].resource_type.as_str(),
            "aws_apprunner_service" | "aws_apprunner_vpc_ingress_connection"
        )));
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
        assert!(out.contains("Egress connector"));
        assert!(out.contains("Ingress service"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:apprunner"));
    }
}

#[test]
fn apprunner_references_require_exact_current_compatible_arn_evidence() {
    for (target, source, field) in [(5, 4, EGRESS), (7, 5, "service_arn")] {
        for case in [
            "literal",
            "wrong-output",
            "mismatch",
            "unknown",
            "source-unknown",
            "empty",
            "wrong-type",
            "deleted",
            "consumer-deleted",
            "deposed",
            "consumer-deposed",
            "data",
            "provider",
            "binding",
            "region",
            "account-conflict",
            "missing",
            "ambiguous",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let expression = if target == 5 {
                &mut v["configuration"]["root_module"]["resources"][target]["expressions"]["network_configuration"]
                    [0]["egress_configuration"][0]["vpc_connector_arn"]
            } else {
                &mut v["configuration"]["root_module"]["resources"][target]["expressions"][field]
            };
            match case {
                "literal" => *expression = json!({"constant_value":"literal"}),
                "wrong-output" => {
                    *expression = json!({"references":[if target==5 {"aws_apprunner_vpc_connector.shared.id"} else {"aws_apprunner_service.web.service_id"}]})
                }
                "ambiguous" => *expression = json!({"references":["var.dynamic"]}),
                "mismatch" | "empty" => {
                    let value = json!(if case == "empty" { "" } else { "different" });
                    if target == 5 {
                        egress(&mut v, "after")["vpc_connector_arn"] = value;
                    } else {
                        v["resource_changes"][target]["change"]["after"][field] = value;
                    }
                }
                "unknown" => v["resource_changes"][target]["change"]["after_unknown"] = json!(true),
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({"arn":true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_apprunner_connection")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-deleted" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => {
                    v["resource_changes"][target]["deposed"] = json!("old");
                    let g = graph(&v);
                    assert!(
                        !g.nodes
                            .iter()
                            .find(|n| n.address
                                == v["resource_changes"][target]["address"].as_str().unwrap())
                            .unwrap()
                            .references
                            .iter()
                            .any(|r| r.attribute == field
                                && r.complete
                                && Some(r.label)
                                    == super::label(
                                        v["resource_changes"][target]["type"].as_str().unwrap(),
                                        field
                                    ))
                    );
                    continue;
                }
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "binding" | "region" | "account-conflict" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region" {"us-west-2"} else {"us-east-1"}}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                    if case == "account-conflict" {
                        v["configuration"]["provider_config"]["other"]["expressions"]["allowed_account_ids"] =
                            json!({"constant_value":["999999999999"]});
                    }
                }
                "missing" => {
                    v["resource_changes"][source]["address"] =
                        json!("aws_apprunner_connection.unrelated");
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, field), "{target} {case}");
        }
    }
}

#[test]
fn apprunner_egress_requires_one_known_vpc_configuration() {
    for case in [
        "default",
        "missing-type",
        "unknown-type",
        "unknown-block",
        "two-egress",
        "two-network",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "default" => egress(&mut v, "after")["egress_type"] = json!("DEFAULT"),
            "missing-type" => {
                egress(&mut v, "after")
                    .as_object_mut()
                    .unwrap()
                    .remove("egress_type");
            }
            "unknown-type" => {
                v["resource_changes"][5]["change"]["after_unknown"] = json!({"network_configuration":[{"egress_configuration":[{"egress_type":true}]}]})
            }
            "unknown-block" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"network_configuration":true})
            }
            "two-egress" => {
                let extra = egress(&mut v, "after").clone();
                v["resource_changes"][5]["change"]["after"]["network_configuration"][0]["egress_configuration"].as_array_mut().unwrap().push(extra);
            }
            "two-network" => {
                let extra =
                    v["resource_changes"][5]["change"]["after"]["network_configuration"][0].clone();
                v["resource_changes"][5]["change"]["after"]["network_configuration"]
                    .as_array_mut()
                    .unwrap()
                    .push(extra);
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 5, EGRESS), "{case}");
        assert!(
            complete(&v, 7, "service_arn"),
            "ingress remains independent: {case}"
        );
    }
}

#[test]
fn apprunner_connector_requires_complete_subnet_and_sg_common_vpc_proof() {
    for case in [
        "partial-subnets",
        "partial-sgs",
        "unknown-subnet",
        "unknown-sg",
        "duplicate",
        "wrong-output",
        "wrong-vpc",
        "vpc-unknown",
        "vpc-deleted",
        "subnet-deposed",
        "sg-deleted",
        "region",
        "literal-parent",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "partial-subnets" => {
                v["resource_changes"][4]["change"]["after"]["subnets"] =
                    json!(["subnet-a", "subnet-b", "literal"])
            }
            "partial-sgs" => {
                v["resource_changes"][4]["change"]["after"]["security_groups"] =
                    json!(["sg-connector", "literal"])
            }
            "unknown-subnet" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"subnets":[false,true]})
            }
            "unknown-sg" => {
                v["resource_changes"][3]["change"]["after_unknown"] = json!({"id":true})
            }
            "duplicate" => {
                v["resource_changes"][4]["change"]["after"]["subnets"] =
                    json!(["subnet-a", "subnet-a"])
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["subnets"] =
                    json!({"references":["aws_subnet.a.arn","aws_subnet.b.arn"]})
            }
            "wrong-vpc" => {
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other-vpc")
            }
            "vpc-unknown" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
            }
            "vpc-deleted" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "subnet-deposed" => v["resource_changes"][1]["deposed"] = json!("old"),
            "sg-deleted" => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            "region" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "literal-parent" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            _ => unreachable!(),
        }
        assert!(parent(&v).is_none(), "{case}");
        assert!(
            complete(&v, 5, EGRESS),
            "service reference independent of connector network: {case}"
        );
    }
}

#[test]
fn apprunner_filtering_lifecycle_and_named_states_preserve_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][5]["previous_address"] = json!("aws_apprunner_service.old");
    v["resource_changes"][5]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][5]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][5].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 11);
    for source in &parsed.nodes {
        let matched = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].metadata, source.metadata);
        assert_eq!(matched[0].action, source.action);
        assert_eq!(matched[0].previous_address, source.previous_address);
    }
    let selected = view::single(
        &graph(&serde_json::from_str(FIXTURE).unwrap()),
        &view::Options {
            focus: Some("aws_apprunner_service.web".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment)
    );
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
    assert!(!complete(&missing, 4, EGRESS));
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
fn apprunner_preserves_verified_account_scope_and_rejects_split_vpc_context() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for node in g.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_apprunner_service"
                | "aws_apprunner_vpc_connector"
                | "aws_apprunner_vpc_ingress_connection"
        )
    }) {
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
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_apprunner_vpc_connector.shared".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(selected.nodes.iter().all(|n| matches!(
        n.resource_type.as_str(),
        "aws_apprunner_vpc_connector" | "aws_vpc"
    )));
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment)
    );
    let mut second = v["resource_changes"][0].clone();
    second["address"] = json!("aws_vpc.other");
    second["change"]["after"]["id"] = json!("vpc-other");
    v["resource_changes"].as_array_mut().unwrap().push(second);
    let mut config = v["configuration"]["root_module"]["resources"][0].clone();
    config["address"] = json!("aws_vpc.other");
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
    v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
        json!({"references":["aws_vpc.other.id"]});
    assert!(complete(&v, 4, "subnets"));
    assert!(complete(&v, 4, "security_groups"));
    assert!(parent(&v).is_none());
}
