use crate::{
    model::{EdgeKind, Graph},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/redshift-endpoints-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
fn parent(g: &Graph, address: &str) -> Option<String> {
    let t = g.nodes.iter().position(|n| n.address == address).unwrap();
    g.edges
        .iter()
        .find(|e| e.to == t && e.kind == EdgeKind::Containment)
        .map(|e| g.nodes[e.from].address.clone())
}
#[test]
fn redshift_endpoint_network_is_independent_of_accessed_cluster() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (12, "cluster_identifier"),
        (12, "subnet_group_name"),
        (12, "vpc_security_group_ids"),
        (13, "cluster_identifier"),
        (13, "vpc_ids"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(
        parent(&g, "aws_redshift_cluster.main").as_deref(),
        Some("aws_vpc.main")
    );
    assert_eq!(
        parent(&g, "aws_redshift_endpoint_access.client").as_deref(),
        Some("aws_vpc.endpoint")
    );
    assert_eq!(g.coverage.selected.resources, 14);
    assert_eq!(g.nodes.len(), 14);
    for kind in [
        "aws_vpc",
        "aws_subnet",
        "aws_security_group",
        "aws_redshift_subnet_group",
        "aws_redshift_endpoint_access",
    ] {
        assert!(
            g.nodes
                .iter()
                .filter(|n| n.resource_type == kind)
                .all(|n| n.member_owner.is_none())
        );
    }
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshift_endpoint_authorization")
            .unwrap()
            .member_owner
            .is_some()
    );
    assert_eq!(
        parent(&g, "aws_redshift_endpoint_authorization.client"),
        None
    );
}
#[test]
fn redshift_endpoint_fields_require_complete_current_known_provenance() {
    for (t, s, p, out) in [
        (12, 6, "cluster_identifier", "cluster_identifier"),
        (12, 10, "subnet_group_name", "name"),
        (12, 11, "vpc_security_group_ids", "id"),
        (13, 6, "cluster_identifier", "cluster_identifier"),
        (13, 7, "vpc_ids", "id"),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "source-unknown",
            "source-sensitive",
            "mismatch",
            "output",
            "shape",
            "data",
            "delete",
            "deposed",
            "provider",
            "region",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"constant_value":"literal"})
                }
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({p:true})
                }
                "source-unknown" | "source-sensitive" => {
                    v["resource_changes"][s]["change"][if case == "source-unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({out:true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "output" => {
                    let a = v["resource_changes"][s]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"references":[format!("{a}.unsupported")]});
                }
                "shape" => {
                    let a = v["resource_changes"][t]["change"]["after"][p].clone();
                    v["resource_changes"][t]["change"]["after"][p] = if a.is_array() {
                        a[0].clone()
                    } else {
                        json!([a])
                    };
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => v["resource_changes"][s]["provider_name"] = json!("foreign/aws"),
                "region" => {
                    let mut other = v["configuration"]["provider_config"]["aws"].clone();
                    other["alias"] = json!("other");
                    other["expressions"]["region"] = json!({"constant_value":"us-west-2"});
                    v["configuration"]["provider_config"]["aws.other"] = other;
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn redshift_endpoint_network_and_cluster_fallback_independently() {
    for case in [
        "known",
        "unknown-sg",
        "partial-sg",
        "foreign-sg",
        "partial-subnets",
        "unknown-subnet",
        "literal-group",
        "unknown-owner",
        "unknown-cluster",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown-sg" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"vpc_security_group_ids":true})
            }
            "partial-sg" => {
                v["resource_changes"][12]["change"]["after"]["vpc_security_group_ids"] =
                    json!(["sg-endpoint", "external"])
            }
            "foreign-sg" => {
                v["resource_changes"][11]["change"]["after"]["vpc_id"] = json!("vpc-fixture");
                v["configuration"]["root_module"]["resources"][11]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.main.id"]});
            }
            "partial-subnets" => {
                v["resource_changes"][10]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-endpoint-a", "subnet-endpoint-b", "external"])
            }
            "unknown-subnet" => {
                v["resource_changes"][8]["change"]["after_unknown"] = json!({"id":true})
            }
            "literal-group" => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["subnet_group_name"] =
                    json!({"constant_value":"endpoint-subnets"})
            }
            "unknown-owner" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"resource_owner":true})
            }
            "unknown-cluster" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"cluster_identifier":true})
            }
            _ => (),
        }
        let cluster = !matches!(case, "unknown-owner" | "unknown-cluster");
        assert_eq!(verified(&v, 12, "cluster_identifier"), cluster, "{case}");
        assert_eq!(
            parent(&graph(&v), "aws_redshift_endpoint_access.client").is_some(),
            matches!(case, "known" | "unknown-owner" | "unknown-cluster"),
            "{case}"
        );
    }
}
fn foreign(v: &mut Value, indices: &[usize]) {
    let mut other = v["configuration"]["provider_config"]["aws"].clone();
    other["alias"] = json!("foreign");
    other["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::999999999999:role/Test");
    v["configuration"]["provider_config"]["aws.foreign"] = other;
    for &i in indices {
        v["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
            json!("aws.foreign");
    }
}
#[test]
fn redshift_endpoint_owner_qualifies_cluster_without_relocating_endpoint() {
    for qualified in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        foreign(&mut v, &[0, 1, 2, 3, 4, 5, 6]);
        if qualified {
            v["resource_changes"][12]["change"]["after"]["resource_owner"] = json!("999999999999");
        }
        assert_eq!(verified(&v, 12, "cluster_identifier"), qualified);
        let g = graph(&v);
        assert_eq!(
            parent(&g, "aws_redshift_endpoint_access.client").as_deref(),
            Some("aws_vpc.endpoint")
        );
        assert_eq!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == "aws_redshift_endpoint_access")
                .unwrap()
                .entity
                .scope
                .as_ref()
                .unwrap()
                .account,
            crate::model::AwsAccountScope::new("123456789012")
        );
        let raw = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(
            !raw.nodes
                .iter()
                .find(|n| n.resource_type == "aws_redshift_endpoint_access")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "Accessed cluster" && r.complete)
        );
    }
}
#[test]
fn redshift_authorization_account_qualifies_only_explicit_complete_vpcs() {
    for case in [
        "qualified",
        "wrong-account",
        "unknown-account",
        "sensitive-account",
        "invalid-account",
        "empty",
        "omitted",
        "partial",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        foreign(&mut v, &[7, 8, 9, 10, 11, 12]);
        v["resource_changes"][13]["change"]["after"]["account"] = json!("999999999999");
        match case {
            "wrong-account" => {
                v["resource_changes"][13]["change"]["after"]["account"] = json!("123456789012")
            }
            "unknown-account" | "sensitive-account" => {
                v["resource_changes"][13]["change"][if case == "unknown-account" {
                    "after_unknown"
                } else {
                    "after_sensitive"
                }] = json!({"account":true})
            }
            "invalid-account" => {
                v["resource_changes"][13]["change"]["after"]["account"] = json!("bad")
            }
            "empty" => v["resource_changes"][13]["change"]["after"]["vpc_ids"] = json!([]),
            "omitted" => {
                v["resource_changes"][13]["change"]["after"]
                    .as_object_mut()
                    .unwrap()
                    .remove("vpc_ids");
                v["configuration"]["root_module"]["resources"][13]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("vpc_ids");
            }
            "partial" => {
                v["resource_changes"][13]["change"]["after"]["vpc_ids"] =
                    json!(["vpc-endpoint", "external"])
            }
            _ => (),
        }
        assert_eq!(verified(&v, 13, "vpc_ids"), case == "qualified", "{case}");
        assert!(verified(&v, 13, "cluster_identifier"), "{case}");
        let g = graph(&v);
        let n = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_redshift_endpoint_authorization")
            .unwrap();
        assert_eq!(
            n.entity.scope.as_ref().unwrap().account,
            crate::model::AwsAccountScope::new("123456789012")
        );
        assert!(n.member_owner.is_some());
    }
}
#[test]
fn redshift_endpoint_filter_and_named_states_preserve_proof_and_coverage() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][8]["change"]["actions"] = json!(["no-op"]);
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        parent(&selected, "aws_redshift_endpoint_access.client"),
        None
    );
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_redshift_endpoint_access.client".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let plans = plan::parse_inputs(
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
}
