use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/datazone-environments-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn node<'a>(g: &'a crate::model::ArchitectureGraph, v: &Value, t: usize) -> &'a crate::model::Node {
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    node(&graph(v), v, t).references.iter().any(|r| {
        r.attribute == p
            && r.complete
            && Some(r.label) == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
    })
}
#[test]
fn datazone_environments_keep_control_scope_and_distinct_configured_targets() {
    let v = fixture();
    let g = graph(&v);
    for (t, p) in [
        (7, "domain_id"),
        (7, "manage_access_role_arn"),
        (7, "provisioning_role_arn"),
        (8, "domain_identifier"),
        (8, "project_identifier"),
        (9, "domain_identifier"),
        (9, "project_identifier"),
        (9, "profile_identifier"),
        (9, "glossary_terms"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    assert_eq!(g.coverage.selected.resources, 14);
    assert_eq!(g.nodes.len(), 14);
    for t in [7, 8, 9, 10, 12] {
        let n = node(&g, &v, t);
        let scope = n.entity.scope.as_ref().unwrap();
        assert_eq!(scope.account.as_ref().unwrap().as_str(), "123456789012");
        assert_eq!(scope.region.as_ref().unwrap().0, "us-east-1");
        assert_eq!(
            n.placement,
            if matches!(t, 9 | 10) {
                Placement::Spatial
            } else {
                Placement::Logical
            }
        );
        assert_eq!(n.member_owner.is_some(), t == 7);
    }
    for (t, account, region) in [
        (8, "234567890123", "us-west-2"),
        (9, "345678901234", "eu-west-1"),
    ] {
        let target = node(&g, &v, t).entity.deployment_target.as_ref().unwrap();
        assert_eq!(target.account.as_ref().unwrap().as_str(), account);
        assert_eq!(target.region.as_ref().unwrap().0, region);
    }
    for t in [7, 10, 12] {
        assert!(node(&g, &v, t).entity.deployment_target.is_none());
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment
                && super::resource(&g.nodes[e.to].resource_type))
    );
    assert!(!g.edges.iter().any(|e| g.nodes[e.from].address
        == "aws_datazone_environment_blueprint_configuration.main"
        && e.kind != EdgeKind::Dependency
        && matches!(
            g.nodes[e.to].resource_type.as_str(),
            super::ENVIRONMENT | super::PROFILE
        )));
    assert!(!verified(&v, 8, "environment_blueprint_identifier"));
    assert!(!verified(&v, 9, "blueprint_identifier"));
}
#[test]
fn datazone_environment_endpoints_require_exact_explicit_current_compatible_values() {
    for (t, p, s, out) in [
        (7, "domain_id", 2, "id"),
        (7, "manage_access_role_arn", 0, "arn"),
        (7, "provisioning_role_arn", 1, "arn"),
        (8, "project_identifier", 3, "id"),
        (9, "profile_identifier", 8, "id"),
        (9, "glossary_terms", 6, "id"),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "source-unknown",
            "mismatch",
            "wrong-output",
            "data",
            "delete",
            "deposed",
            "provider",
            "consumer-delete",
            "ambiguous",
        ] {
            let mut v = fixture();
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
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({out:true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("different"),
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"].as_str().unwrap();
                    let a = format!("{a}.name");
                    v["configuration"]["root_module"]["resources"][t]["expressions"][p] =
                        json!({"references":[a]});
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                "ambiguous" => {
                    let mut copy = v["resource_changes"][s].clone();
                    let a = copy["address"].as_str().unwrap().to_owned();
                    copy["address"] = json!(format!("{a}[1]"));
                    v["resource_changes"][s]["address"] = json!(format!("{a}[0]"));
                    v["resource_changes"].as_array_mut().unwrap().push(copy);
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn datazone_environment_domains_collections_and_parent_bindings_are_independent() {
    for case in [
        "foreign-profile",
        "same-id-foreign-domain",
        "unknown-profile-domain",
        "unknown-project-domain",
        "partial-terms",
        "duplicate-terms",
        "unknown-terms",
        "wrong-shape",
        "domain-account",
        "domain-region",
        "term-glossary-account",
    ] {
        let mut v = fixture();
        let (t, p) = match case {
            "foreign-profile" | "same-id-foreign-domain" => {
                v["resource_changes"][9]["change"]["after"]["profile_identifier"] =
                    json!("profile-foreign");
                v["configuration"]["root_module"]["resources"][9]["expressions"]["profile_identifier"] =
                    json!({"references":["aws_datazone_environment_profile.foreign.id"]});
                if case == "same-id-foreign-domain" {
                    v["resource_changes"][11]["change"]["after"]["id"] = json!("dzd-main");
                    v["resource_changes"][12]["change"]["after"]["domain_identifier"] =
                        json!("dzd-main");
                }
                (9, "profile_identifier")
            }
            "unknown-profile-domain" => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (9, "profile_identifier")
            }
            "unknown-project-domain" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (9, "project_identifier")
            }
            "partial-terms" | "duplicate-terms" | "wrong-shape" => {
                v["resource_changes"][9]["change"]["after"]["glossary_terms"] = match case {
                    "partial-terms" => json!(["term-main", "external"]),
                    "duplicate-terms" => json!(["term-main", "term-main"]),
                    _ => json!("term-main"),
                };
                (9, "glossary_terms")
            }
            "unknown-terms" => {
                v["resource_changes"][9]["change"]["after_unknown"] =
                    json!({"glossary_terms":[true]});
                (9, "glossary_terms")
            }
            _ => {
                let mut alias = v["configuration"]["provider_config"]["aws"].clone();
                alias["alias"] = json!("foreign");
                if case == "domain-region" {
                    alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
                } else {
                    alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                        json!("arn:aws:iam::999999999999:role/Test");
                }
                v["configuration"]["provider_config"]["aws.foreign"] = alias;
                v["configuration"]["root_module"]["resources"][if case == "term-glossary-account" {
                    5
                } else {
                    2
                }]["provider_config_key"] = json!("aws.foreign");
                (9, "glossary_terms")
            }
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
#[test]
fn datazone_deployment_targets_require_both_configured_known_bounded_fields() {
    for (t, account, region) in [
        (8, "aws_account_id", "aws_account_region"),
        (9, "account_identifier", "account_region"),
    ] {
        for case in [
            "unconfigured-account",
            "unconfigured-region",
            "unknown-account",
            "unknown-region",
            "sensitive-account",
            "sensitive-region",
            "invalid-account",
            "invalid-region",
            "object-region",
            "delete",
            "deposed",
            "data",
            "foreign-provider",
        ] {
            let mut v = fixture();
            match case {
                "unconfigured-account" | "unconfigured-region" => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"]
                        .as_object_mut()
                        .unwrap()
                        .remove(if case.ends_with("account") {
                            account
                        } else {
                            region
                        });
                }
                "unknown-account" | "unknown-region" | "sensitive-account" | "sensitive-region" => {
                    let path = if case.ends_with("account") {
                        account
                    } else {
                        region
                    };
                    v["resource_changes"][t]["change"][if case.starts_with("unknown") {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({path:true});
                }
                "invalid-account" => {
                    v["resource_changes"][t]["change"]["after"][account] = json!("123")
                }
                "invalid-region" => {
                    v["resource_changes"][t]["change"]["after"][region] = json!("PRIVATE_REGION")
                }
                "object-region" => {
                    v["resource_changes"][t]["change"]["after"][region] =
                        json!({"value":"us-west-2"})
                }
                "delete" => v["resource_changes"][t]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][t]["deposed"] = json!("old"),
                "data" => v["resource_changes"][t]["mode"] = json!("data"),
                "foreign-provider" => {
                    v["resource_changes"][t]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                _ => unreachable!(),
            }
            assert!(
                node(&graph(&v), &v, t).entity.deployment_target.is_none(),
                "{t} {case}"
            );
        }
    }
}
#[test]
fn datazone_environment_lifecycle_filter_and_named_state_identity_remain_separate() {
    for t in [7, 8, 9, 10, 12] {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"domain,environment"});
        v["resource_changes"][t]["previous_address"] = json!("aws_datazone_environment.old");
        assert_eq!(graph(&v).coverage.selected.resources, 14);
    }
    let mut v = fixture();
    v["resource_changes"][9]["mode"] = json!("data");
    assert_ne!(node(&graph(&v), &v, 9).placement, Placement::Spatial);
    let v = fixture();
    let g = graph(&v);
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_datazone_environment.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
    assert_eq!(selected.nodes.len(), 1);
    assert!(selected.nodes[0].entity.deployment_target.is_some());
    assert!(selected.nodes[0].member_owner.is_none());
    let inputs = plan::parse_inputs(
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(
        semantic::cross_state::resolve(&inputs, &[])
            .edges
            .is_empty()
    );
}
#[test]
fn datazone_target_scope_display_and_computed_payload_exclusions_match_svg_and_html() {
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let o = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(o.content.contains("Configured deployment target:"));
        assert!(o.content.contains("234567890123"));
        assert!(o.content.contains("345678901234"));
        assert!(!o.content.contains("456789012345"));
        assert!(!o.content.contains("PRIVATE_"));
    }
}
