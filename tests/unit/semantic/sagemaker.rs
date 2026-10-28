use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/sagemaker-plan.json");

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn complete(v: &Value, target: usize, field: &str) -> bool {
    let resource = &v["resource_changes"][target];
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == resource["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label) == super::label(resource["type"].as_str().unwrap(), field)
        })
}

fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment && g.nodes[e.to].resource_type == "aws_sagemaker_model"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn shared_models_and_configurations_keep_identity_separate_from_network_context() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (target, field) in [
        (5, "execution_role_arn"),
        (5, super::SUBNETS),
        (5, super::GROUPS),
        (6, "execution_role_arn"),
        (7, super::MODELS),
        (8, "endpoint_config_name"),
        (9, "endpoint_config_name"),
        (10, super::MODELS),
    ] {
        assert!(complete(&v, target, field), "{target} {field}");
    }
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 11);
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_sagemaker_"))
    {
        assert!(n.member_owner.is_none());
        assert_eq!(
            n.placement,
            if n.resource_type == "aws_sagemaker_endpoint" {
                Placement::Spatial
            } else {
                Placement::Logical
            }
        );
    }
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && matches!(
            g.nodes[e.to].resource_type.as_str(),
            "aws_sagemaker_endpoint" | "aws_sagemaker_endpoint_configuration"
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
        assert!(out.contains("Endpoint configuration"));
        assert!(out.contains("Production models"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:sagemaker"));
    }
}

#[test]
fn references_reject_unproven_values_identity_and_scope() {
    for (target, source, field, output) in [
        (8, 7, "endpoint_config_name", "name"),
        (5, 4, "execution_role_arn", "arn"),
    ] {
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
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"constant_value":"literal"})
                }
                "wrong-output" => {
                    let address = v["resource_changes"][source]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":[format!("{address}.id")]});
                }
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":["var.dynamic"]})
                }
                "mismatch" | "empty" => {
                    v["resource_changes"][target]["change"]["after"][field] =
                        json!(if case == "empty" { "" } else { "wrong" })
                }
                "unknown" => v["resource_changes"][target]["change"]["after_unknown"] = json!(true),
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_sagemaker_training_job")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-deleted" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
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
                        json!("aws_sagemaker_model.unrelated")
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, target, field), "{target} {case}");
        }
    }
}

#[test]
fn production_variants_require_complete_known_model_names() {
    for case in [
        "unknown",
        "unknown-block",
        "mask-length",
        "missing-model",
        "literal-member",
        "wrong-output",
        "mismatch",
        "duplicate-identity",
        "empty",
        "data",
        "deleted",
        "region",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"production_variants":[{}, {"model_name":true}]})
            }
            "unknown-block" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"production_variants":true})
            }
            "mask-length" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"production_variants":[{}]})
            }
            "missing-model" => {
                v["resource_changes"][7]["change"]["after"]["production_variants"][1] =
                    json!({"variant_name":"components"})
            }
            "literal-member" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["production_variants"]
                    [1]["model_name"] = json!({"constant_value":"other"})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["production_variants"]
                    [1]["model_name"] = json!({"references":["aws_sagemaker_model.other.arn"]})
            }
            "mismatch" => v["resource_changes"][6]["change"]["after"]["name"] = json!("different"),
            "duplicate-identity" => {
                v["resource_changes"][6]["change"]["after"]["name"] = json!("shared")
            }
            "empty" => {
                v["resource_changes"][7]["change"]["after"]["production_variants"] = json!([])
            }
            "data" => v["resource_changes"][6]["mode"] = json!("data"),
            "deleted" => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            "region" => {
                v["configuration"]["provider_config"]["west"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
                    json!("west");
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, 7, super::MODELS), "{case}");
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][7]["change"]["after"]["production_variants"][1]["model_name"] =
        json!("shared");
    v["configuration"]["root_module"]["resources"][7]["expressions"]["production_variants"][1]["model_name"] =
        json!({"references":["aws_sagemaker_model.shared.name"]});
    assert!(complete(&v, 7, super::MODELS));
}

#[test]
fn model_network_requires_complete_sets_and_one_verified_vpc() {
    for case in [
        "unknown-subnet",
        "unknown-group",
        "duplicate-subnet",
        "literal-subnet",
        "missing-group",
        "two-blocks",
        "split-vpc",
        "parent-mismatch",
        "parent-deleted",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown-subnet" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"vpc_config":[{"subnets":[false,true]}]})
            }
            "unknown-group" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"vpc_config":[{"security_group_ids":[true]}]})
            }
            "duplicate-subnet" => {
                v["resource_changes"][5]["change"]["after"]["vpc_config"][0]["subnets"] =
                    json!(["subnet-a", "subnet-a"])
            }
            "literal-subnet" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["vpc_config"][0]
                    ["subnets"] = json!({"references":["aws_subnet.a.id"]})
            }
            "missing-group" => {
                v["resource_changes"][5]["change"]["after"]["vpc_config"][0]["security_group_ids"] =
                    json!([])
            }
            "two-blocks" => {
                let block = v["resource_changes"][5]["change"]["after"]["vpc_config"][0].clone();
                v["resource_changes"][5]["change"]["after"]["vpc_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            }
            "parent-mismatch" => {
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-unknown")
            }
            "parent-deleted" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "split-vpc" => {
                let mut other = v["resource_changes"][0].clone();
                other["address"] = json!("aws_vpc.other");
                other["change"]["after"]["id"] = json!("vpc-other");
                v["resource_changes"].as_array_mut().unwrap().push(other);
                let mut config = v["configuration"]["root_module"]["resources"][0].clone();
                config["address"] = json!("aws_vpc.other");
                v["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(config);
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
                assert!(complete(&v, 5, super::SUBNETS));
                assert!(complete(&v, 5, super::GROUPS));
            }
            _ => unreachable!(),
        }
        assert!(parent(&v).is_none(), "{case}");
    }
}

#[test]
fn selection_named_states_and_deposed_resources_preserve_coverage() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut old = v["resource_changes"][5].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 12);
    for source in &parsed.nodes {
        let nodes = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].previous_address, source.previous_address);
    }
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_sagemaker_endpoint.first".into()),
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
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(7);
    assert!(!complete(&missing, 7, "endpoint_config_name"));
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
    }
}

#[test]
fn verified_provider_account_is_retained_without_changing_unrelated_defaults() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for node in g
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_sagemaker_"))
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
    assert!(
        graph(&serde_json::from_str(FIXTURE).unwrap())
            .nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_sagemaker_"))
            .all(|n| n.entity.scope.as_ref().unwrap().account.is_none())
    );
}
