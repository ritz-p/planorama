use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/opensearch-serverless-plan.json");
const ENDPOINT: &str = "aws_opensearchserverless_vpc_endpoint.shared";

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn complete(v: &Value, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == ENDPOINT)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label) == super::label("aws_opensearchserverless_vpc_endpoint", field)
        })
}

fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == ENDPOINT)
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn serverless_endpoint_proves_network_without_claiming_collection_ownership() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for field in ["vpc_id", "subnet_ids", "security_group_ids"] {
        assert!(complete(&v, field), "{field}");
    }
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 9);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
        if n.resource_type == "aws_opensearchserverless_collection" {
            assert_eq!(n.placement, Placement::Spatial);
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == n.address)
            );
        }
    }
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.address == ENDPOINT)
            .unwrap()
            .placement,
        Placement::Logical
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
        assert!(out.contains("Endpoint VPC"));
        assert!(out.contains("Configured subnets"));
        assert!(out.contains("Configured security groups"));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:aoss"));
    }
}

#[test]
fn serverless_references_require_exact_complete_current_compatible_id_evidence() {
    for (field, source) in [("vpc_id", 0), ("subnet_ids", 1), ("security_group_ids", 3)] {
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
            "consumer-data",
            "provider",
            "consumer-provider",
            "binding",
            "region",
            "account-conflict",
            "missing",
            "ambiguous",
            "partial",
            "duplicate",
        ] {
            if field == "vpc_id" && matches!(case, "partial" | "duplicate") {
                continue;
            }
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let address = v["resource_changes"][source]["address"]
                .as_str()
                .unwrap()
                .to_string();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][4]["expressions"][field] =
                        json!({"constant_value":"literal"})
                }
                "wrong-output" => {
                    v["configuration"]["root_module"]["resources"][4]["expressions"][field] =
                        json!({"references":[format!("{address}.arn")]})
                }
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][4]["expressions"][field] =
                        json!({"references":["var.dynamic"]})
                }
                "mismatch" | "empty" => {
                    v["resource_changes"][4]["change"]["after"][field] = if field == "vpc_id" {
                        json!(if case == "empty" { "" } else { "different" })
                    } else if case == "empty" {
                        json!([])
                    } else {
                        json!(["different"])
                    }
                }
                "unknown" => {
                    v["resource_changes"][4]["change"]["after_unknown"] = json!({field:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({"id":true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_opensearch_domain")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-deleted" => {
                    v["resource_changes"][4]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => v["resource_changes"][4]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "consumer-data" => v["resource_changes"][4]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "consumer-provider" => {
                    v["resource_changes"][4]["provider_name"] = json!("example.org/foreign/aws")
                }
                "binding" | "region" | "account-conflict" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                    if case == "account-conflict" {
                        v["configuration"]["provider_config"]["other"]["expressions"]["allowed_account_ids"] =
                            json!({"constant_value":["999999999999"]});
                    }
                }
                "missing" => v["resource_changes"][source]["address"] = json!("aws_vpc.unrelated"),
                "partial" => v["resource_changes"][4]["change"]["after"][field]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("literal")),
                "duplicate" => {
                    let a = v["resource_changes"][4]["change"]["after"][field][0].clone();
                    v["resource_changes"][4]["change"]["after"][field]
                        .as_array_mut()
                        .unwrap()
                        .push(a);
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, field), "{field} {case}");
            assert!(parent(&v).is_none(), "{field} {case}");
        }
    }
}

#[test]
fn serverless_network_requires_agreement_and_never_invents_default_security_groups() {
    for case in [
        "sg-absent",
        "sg-null",
        "sg-unknown",
        "subnet-mask",
        "parent-literal",
        "parent-mismatch",
        "parent-unknown",
        "split-vpc",
        "explicit-vpc-mismatch",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "sg-absent" => {
                v["resource_changes"][4]["change"]["after"]
                    .as_object_mut()
                    .unwrap()
                    .remove("security_group_ids");
            }
            "sg-null" => {
                v["resource_changes"][4]["change"]["after"]["security_group_ids"] = Value::Null
            }
            "sg-unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"security_group_ids":[true]})
            }
            "subnet-mask" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"subnet_ids":[false,true]})
            }
            "parent-literal" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["vpc_id"] =
                    json!({"constant_value":"vpc-main"})
            }
            "parent-mismatch" => {
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("different")
            }
            "parent-unknown" => {
                v["resource_changes"][1]["change"]["after_unknown"] = json!({"vpc_id":true})
            }
            "split-vpc" | "explicit-vpc-mismatch" => {
                let mut node = v["resource_changes"][0].clone();
                node["address"] = json!("aws_vpc.other");
                node["change"]["after"]["id"] = json!("vpc-other");
                v["resource_changes"].as_array_mut().unwrap().push(node);
                let mut config = v["configuration"]["root_module"]["resources"][0].clone();
                config["address"] = json!("aws_vpc.other");
                v["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(config);
                let source = if case == "split-vpc" { 3 } else { 4 };
                v["resource_changes"][source]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][source]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
                assert!(complete(&v, "vpc_id"));
                assert!(complete(&v, "subnet_ids"));
                assert!(complete(&v, "security_group_ids"));
            }
            _ => unreachable!(),
        }
        assert!(parent(&v).is_none(), "{case}");
    }
}

#[test]
fn serverless_preserves_scope_lifecycle_filtering_and_named_state_boundaries() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_opensearchserverless_collection" | "aws_opensearchserverless_vpc_endpoint"
        )
    }) {
        assert_eq!(
            n.entity
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
            focus: Some(ENDPOINT.into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment)
    );
    v["resource_changes"][4]["previous_address"] =
        json!("aws_opensearchserverless_vpc_endpoint.old");
    v["resource_changes"][4]["change"]["actions"] = json!(["delete", "create"]);
    v["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][4].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 10);
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
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].deposed_key.is_some())
    );
    let mut missing: Value = serde_json::from_str(FIXTURE).unwrap();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(!complete(&missing, "subnet_ids"));
    assert!(parent(&missing).is_none());
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
