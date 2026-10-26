use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/redshift-serverless-plan.json");

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn parent(v: &Value, address: &str) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == address)
        .map(|e| g.nodes[e.from].address.clone())
}

fn complete(v: &Value, address: &str, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label) == super::label(address.split('.').next().unwrap(), field)
        })
}

#[test]
fn serverless_ownership_does_not_replace_independent_network_placement() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(
        parent(&v, "aws_redshiftserverless_workgroup.main").as_deref(),
        Some("aws_vpc.main")
    );
    assert_eq!(
        parent(&v, "aws_redshiftserverless_endpoint_access.main").as_deref(),
        Some("aws_vpc.other")
    );
    assert!(parent(&v, "aws_redshiftserverless_namespace.main").is_none());
    assert!(complete(
        &v,
        "aws_redshiftserverless_workgroup.main",
        "namespace_name"
    ));
    assert!(complete(
        &v,
        "aws_redshiftserverless_endpoint_access.main",
        "workgroup_name"
    ));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 11);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshiftserverless_namespace")
            .unwrap()
            .placement,
        Placement::Logical
    );
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshiftserverless_endpoint_access")
            .unwrap()
            .placement,
        Placement::Spatial
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
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Namespace owner"));
        assert!(out.contains("Workgroup owner"));
    }
}

#[test]
fn serverless_owner_references_require_exact_names_scope_and_current_identity() {
    for (target, source, field, address, wrong) in [
        (
            6,
            5,
            "namespace_name",
            "aws_redshiftserverless_workgroup.main",
            "aws_redshiftserverless_namespace.main.namespace_id",
        ),
        (
            9,
            6,
            "workgroup_name",
            "aws_redshiftserverless_endpoint_access.main",
            "aws_redshiftserverless_workgroup.main.workgroup_id",
        ),
    ] {
        for case in [
            "literal",
            "wrong-output",
            "mismatch",
            "unknown",
            "source-unknown",
            "wrong-type",
            "deleted",
            "deposed",
            "data",
            "provider",
            "region",
            "ambiguous",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"constant_value":"name"})
                }
                "wrong-output" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":[wrong]})
                }
                "mismatch" => {
                    v["resource_changes"][target]["change"]["after"][field] = json!("different")
                }
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({field:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({field:true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_redshift_cluster")
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "region" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                }
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] =
                        json!({"references":["var.dynamic"]})
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, address, field), "{field} {case}");
        }
    }
    for owner in [json!("999999999999"), json!(""), json!(null)] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][9]["change"]["after"]["owner_account"] = owner.clone();
        if owner.is_null() {
            v["resource_changes"][9]["change"]["after_unknown"] = json!({"owner_account":true});
        }
        assert!(!complete(
            &v,
            "aws_redshiftserverless_endpoint_access.main",
            "workgroup_name"
        ));
        assert_eq!(
            parent(&v, "aws_redshiftserverless_endpoint_access.main").as_deref(),
            Some("aws_vpc.other")
        );
    }
}

#[test]
fn serverless_network_requires_complete_local_subnet_and_security_group_sets() {
    for (target, subnet, sgfield, address) in [
        (
            6,
            2,
            "security_group_ids",
            "aws_redshiftserverless_workgroup.main",
        ),
        (
            9,
            7,
            "vpc_security_group_ids",
            "aws_redshiftserverless_endpoint_access.main",
        ),
    ] {
        for case in [
            "extra-subnet",
            "unknown-subnet",
            "extra-sg",
            "unknown-sg",
            "literal",
            "mixed-vpc",
            "deleted-subnet",
            "data-subnet",
            "unknown-vpc",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "extra-subnet" => v["resource_changes"][target]["change"]["after"]["subnet_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unproven")),
                "unknown-subnet" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({"subnet_ids":true})
                }
                "extra-sg" => v["resource_changes"][target]["change"]["after"][sgfield]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unproven")),
                "unknown-sg" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({sgfield:true})
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"]["subnet_ids"] =
                        json!({"constant_value":["literal"]})
                }
                "mixed-vpc" => {
                    let (id, reference) = if target == 6 {
                        ("vpc-other", "aws_vpc.other.id")
                    } else {
                        ("vpc-main", "aws_vpc.main.id")
                    };
                    v["resource_changes"][subnet]["change"]["after"]["vpc_id"] = json!(id);
                    v["configuration"]["root_module"]["resources"][subnet]["expressions"]["vpc_id"] =
                        json!({"references":[reference]});
                }
                "deleted-subnet" => {
                    v["resource_changes"][subnet]["change"]["actions"] = json!(["delete"])
                }
                "data-subnet" => v["resource_changes"][subnet]["mode"] = json!("data"),
                "unknown-vpc" => {
                    v["resource_changes"][if target == 6 { 0 } else { 1 }]["change"]["after_unknown"] =
                        json!({"id":true})
                }
                _ => unreachable!(),
            }
            assert!(parent(&v, address).is_none(), "{address} {case}");
        }
    }
}

#[test]
fn serverless_named_states_lifecycle_and_filtering_keep_exact_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][9]["previous_address"] =
        json!("aws_redshiftserverless_endpoint_access.old");
    v["resource_changes"][9]["change"]["actions"] = json!(["update"]);
    let mut old = v["resource_changes"][9].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for source in &parsed.nodes {
        let found = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].action, source.action);
        assert_eq!(found[0].metadata, source.metadata);
        assert_eq!(found[0].previous_address, source.previous_address);
    }
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_redshiftserverless_workgroup.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment),
        "{:?}",
        selected
            .edges
            .iter()
            .map(|e| (
                &selected.nodes[e.from].address,
                &selected.nodes[e.to].address,
                e.kind
            ))
            .collect::<Vec<_>>()
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(5);
    assert!(!complete(
        &missing,
        "aws_redshiftserverless_workgroup.main",
        "namespace_name"
    ));
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
            format: crate::OutputFormat::Html,
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

#[test]
fn serverless_account_scope_and_filtered_network_proof_are_preserved() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    value["resource_changes"][9]["change"]["after"]["owner_account"] = json!("123456789012");
    let parsed = crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
    assert!(
        parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "workgroup_name" && r.service_reference_verified)
    );
    let graph = semantic::transform(&parsed);
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.attribute == "workgroup_name" && r.complete)
    );
    let mut foreign = value.clone();
    foreign["resource_changes"][9]["change"]["after"]["owner_account"] = json!("210987654321");
    let foreign = semantic::transform(
        &crate::plan::evidence::tests::parse_fixture(&foreign.to_string()).unwrap(),
    );
    assert!(
        !foreign
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.attribute == "workgroup_name" && r.complete)
    );
    for node in &graph.nodes {
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012",
            "{}",
            node.address
        );
    }
    let output = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert!(
        doc.descendants()
            .any(|n| n.attribute("data-account") == Some("123456789012")
                && n.attribute("data-region") == Some("us-east-1"))
    );
    let groups = crate::view::scope::groups(&graph, &crate::layout::ContainmentTree::new(&graph));
    assert!(groups.keys().all(Option::is_some));
    for depth in [0, 1] {
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_redshiftserverless_workgroup.main".into()),
                depth: Some(depth),
                ..Default::default()
            },
        )
        .unwrap();
        let placement = selected.edges.iter().any(|e| {
            e.kind == EdgeKind::Containment
                && selected.nodes[e.to].address == "aws_redshiftserverless_workgroup.main"
        });
        assert_eq!(placement, depth == 1, "focus depth {depth}");
    }
}
