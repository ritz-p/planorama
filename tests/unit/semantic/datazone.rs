use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/datazone-plan.json");
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
fn set(v: &mut Value, p: &str, x: Value) {
    *v.pointer_mut(&format!("/{}", p.replace('.', "/"))).unwrap() = x;
}
#[test]
fn datazone_domain_cards_and_qualified_metadata_keep_independent_identity() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (3, "domain_execution_role"),
        (3, "service_role"),
        (3, "kms_key_identifier"),
        (4, "domain_identifier"),
        (5, "domain_identifier"),
        (5, "glossary_terms"),
        (6, "owning_project_identifier"),
        (7, "owning_project_identifier"),
        (8, "glossary_identifier"),
        (9, "glossary_identifier"),
        (9, "term_relations.classifies"),
        (9, "term_relations.is_a"),
        (10, "owning_project_identifier"),
        (11, "owning_project_identifier"),
        (12, "owning_project_identifier"),
        (12, "forms_input.type_identifier"),
        (12, "forms_input.type_revision"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 17);
    assert_eq!(g.nodes.len(), 17);
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(
            n.placement,
            if n.resource_type == super::DOMAIN {
                Placement::Spatial
            } else {
                Placement::Logical
            }
        );
        assert_eq!(n.member_owner.is_some(), n.resource_type != super::DOMAIN);
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment
                && super::resource(&g.nodes[e.to].resource_type))
    );
    let user = g
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_iam_user")
        .unwrap();
    assert!(!g.edges.iter().any(|e| e.from == user || e.to == user));
}
#[test]
fn datazone_references_require_exact_known_current_outputs_and_explicit_provenance() {
    for (t, path, p, s, out, wrong) in [
        (
            3,
            "domain_execution_role",
            "domain_execution_role",
            0,
            "arn",
            "id",
        ),
        (4, "domain_identifier", "domain_identifier", 3, "id", "arn"),
        (5, "glossary_terms", "glossary_terms", 8, "id", "name"),
        (
            8,
            "glossary_identifier",
            "glossary_identifier",
            6,
            "id",
            "name",
        ),
        (
            9,
            "term_relations.0.classifies",
            "term_relations.classifies",
            8,
            "id",
            "name",
        ),
        (
            12,
            "forms_input.0.type_identifier",
            "forms_input.type_identifier",
            10,
            "name",
            "id",
        ),
        (
            12,
            "forms_input.1.type_revision",
            "forms_input.type_revision",
            11,
            "revision",
            "name",
        ),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "wrong-output",
            "mismatch",
            "data",
            "delete",
            "deposed",
            "provider",
            "source-sensitive",
            "consumer-delete",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v,
                    &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                    json!({"constant_value":"literal"}),
                ),
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({path.split('.').next().unwrap():true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.{wrong}")]}),
                    );
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn datazone_form_revision_pairs_and_domain_qualification_do_not_use_names_alone() {
    for case in [
        "foreign-form",
        "same-name-foreign-domain",
        "stale-revision",
        "unknown-revision",
        "different-revision-source",
        "partial-forms",
        "duplicate-key",
        "unknown-key",
        "object",
        "foreign-project",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let p = match case {
            "foreign-form" | "same-name-foreign-domain" => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["forms_input"]
                    [0] = json!({"type_identifier":{"references":["aws_datazone_form_type.foreign.name"]},"type_revision":{"references":["aws_datazone_form_type.foreign.revision"]}});
                if case == "same-name-foreign-domain" {
                    v["resource_changes"][13]["change"]["after"]["id"] = json!("dzd-main");
                    v["resource_changes"][15]["change"]["after"]["domain_identifier"] =
                        json!("dzd-main");
                }
                "forms_input.type_identifier"
            }
            "stale-revision" => {
                v["resource_changes"][10]["change"]["after"]["revision"] = json!("3");
                "forms_input.type_identifier"
            }
            "unknown-revision" => {
                v["resource_changes"][10]["change"]["after_unknown"] = json!({"revision":true});
                "forms_input.type_identifier"
            }
            "different-revision-source" => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["forms_input"]
                    [0]["type_revision"] =
                    json!({"references":["aws_datazone_form_type.foreign.revision"]});
                "forms_input.type_identifier"
            }
            "partial-forms" => {
                v["resource_changes"][12]["change"]["after"]["forms_input"].as_array_mut().unwrap().push(json!({"map_block_key":"third","type_identifier":"external","type_revision":"1"}));
                "forms_input.type_identifier"
            }
            "duplicate-key" => {
                v["resource_changes"][12]["change"]["after"]["forms_input"][1]["map_block_key"] =
                    json!("local-a");
                "forms_input.type_identifier"
            }
            "unknown-key" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"forms_input":[{"map_block_key":true},{}]});
                "forms_input.type_identifier"
            }
            "object" => {
                let row = v["resource_changes"][12]["change"]["after"]["forms_input"][0].clone();
                v["resource_changes"][12]["change"]["after"]["forms_input"] = row;
                "forms_input.type_identifier"
            }
            "foreign-project" => {
                v["resource_changes"][12]["change"]["after"]["owning_project_identifier"] =
                    json!("project-foreign");
                v["configuration"]["root_module"]["resources"][12]["expressions"]["owning_project_identifier"] =
                    json!({"references":["aws_datazone_project.foreign.id"]});
                "owning_project_identifier"
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, 12, p), "{case}");
        assert!(verified(&v, 12, "domain_identifier"));
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["root_module"]["resources"][12]["expressions"]["forms_input"][0]["type_revision"] =
        json!({"constant_value":"1"});
    assert!(verified(&v, 12, "forms_input.type_identifier"));
}
#[test]
fn datazone_optional_term_domain_and_complete_taxonomy_sets_are_qualified_by_glossary() {
    for case in [
        "omitted",
        "literal-match",
        "mismatch",
        "unknown",
        "glossary-domain",
        "partial-terms",
        "taxonomy-object",
        "taxonomy-duplicate",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "omitted" => (8, "glossary_identifier"),
            "literal-match" => {
                v["resource_changes"][8]["change"]["after"]["domain_identifier"] =
                    json!("dzd-main");
                v["configuration"]["root_module"]["resources"][8]["expressions"]["domain_identifier"] =
                    json!({"constant_value":"dzd-main"});
                (8, "glossary_identifier")
            }
            "mismatch" => {
                v["resource_changes"][8]["change"]["after"]["domain_identifier"] =
                    json!("dzd-foreign");
                (8, "glossary_identifier")
            }
            "unknown" => {
                v["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (8, "glossary_identifier")
            }
            "glossary-domain" => {
                v["resource_changes"][6]["change"]["after"]["domain_identifier"] =
                    json!("dzd-foreign");
                v["configuration"]["root_module"]["resources"][6]["expressions"]["domain_identifier"] =
                    json!({"references":["aws_datazone_domain.foreign.id"]});
                (5, "glossary_terms")
            }
            "partial-terms" => {
                v["resource_changes"][5]["change"]["after"]["glossary_terms"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("external"));
                (5, "glossary_terms")
            }
            "taxonomy-object" => {
                let a = v["resource_changes"][9]["change"]["after"]["term_relations"][0].clone();
                v["resource_changes"][9]["change"]["after"]["term_relations"] = a;
                (9, "term_relations.classifies")
            }
            "taxonomy-duplicate" => {
                v["resource_changes"][9]["change"]["after"]["term_relations"][0]["classifies"] =
                    json!(["term-first", "term-first"]);
                (9, "term_relations.classifies")
            }
            _ => unreachable!(),
        };
        assert_eq!(
            verified(&v, t, p),
            matches!(case, "omitted" | "literal-match"),
            "{case}"
        );
    }
}
#[test]
fn datazone_version_account_region_and_parent_bindings_are_independent_constraints() {
    for case in [
        "v1",
        "unknown-version",
        "foreign-domain-account",
        "foreign-domain-region",
        "foreign-glossary-account",
        "unknown-form-domain",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "v1" => {
                v["resource_changes"][3]["change"]["after"]["domain_version"] = json!("V1");
                (3, "service_role")
            }
            "unknown-version" => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"domain_version":true});
                (3, "service_role")
            }
            "unknown-form-domain" => {
                v["resource_changes"][10]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (12, "forms_input.type_identifier")
            }
            _ => {
                let mut alias = v["configuration"]["provider_config"]["aws"].clone();
                alias["alias"] = json!("foreign");
                if case == "foreign-domain-region" {
                    alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
                } else {
                    alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                        json!("arn:aws:iam::999999999999:role/Test");
                }
                v["configuration"]["provider_config"]["aws.foreign"] = alias;
                v["configuration"]["root_module"]["resources"][if case == "foreign-glossary-account"
                {
                    6
                } else {
                    3
                }]["provider_config_key"] = json!("aws.foreign");
                (5, "glossary_terms")
            }
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
#[test]
fn datazone_lifecycle_filters_data_separation_named_states_and_payload_privacy_remain_exact() {
    for t in 3..=15 {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"domain,metadata"});
        v["resource_changes"][t]["previous_address"] = json!("aws_datazone_project.old");
        assert_eq!(graph(&v).coverage.selected.resources, 17);
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][3]["mode"] = json!("data");
    let g = graph(&v);
    let d = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_datazone_domain.main")
        .unwrap();
    assert_ne!(d.placement, Placement::Spatial);
    assert!(!verified(&v, 4, "domain_identifier"));
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_datazone_glossary_term.second".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
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
        let o = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!o.content.contains("PRIVATE_"));
    }
}
#[test]
fn datazone_form_qualification_never_matches_two_unknown_or_sensitive_values() {
    for mask in ["after_unknown", "after_sensitive"] {
        for (input, output) in [("type_identifier", "name"), ("type_revision", "revision")] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            v["resource_changes"][12]["change"][mask] = json!({"forms_input":[{input:true},{}]});
            v["resource_changes"][10]["change"][mask] = json!({output:true});
            assert!(
                !verified(&v, 12, "forms_input.type_identifier"),
                "{mask} {input}"
            );
            assert!(
                !verified(&v, 12, "forms_input.type_revision"),
                "{mask} {input}"
            );
        }
    }
}
