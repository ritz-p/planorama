use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/datazone-access-plan.json");
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
fn set(v: &mut Value, p: &str, x: Value) {
    *v.pointer_mut(&format!("/{}", p.replace('.', "/"))).unwrap() = x;
}
#[test]
fn datazone_access_keeps_typed_domain_qualified_helpers_and_independent_grants() {
    let v = fixture();
    for (t, p) in [
        (7, "domain_identifier"),
        (7, "user_identifier"),
        (8, "user_identifier"),
        (9, "domain_identifier"),
        (10, "entity_identifier"),
        (11, "entity_identifier"),
        (12, "entity_identifier"),
        (10, "principal.project.project_identifier"),
        (11, "principal.user.user_identifier"),
        (12, "principal.user.user_identifier"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 22);
    assert_eq!(g.nodes.len(), 22);
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(n.placement, Placement::Logical);
        assert_eq!(n.member_owner.is_some(), n.resource_type == super::USER);
        let s = n.entity.scope.as_ref().unwrap();
        assert_eq!(s.account.as_ref().unwrap().as_str(), "123456789012");
        assert_eq!(s.region.as_ref().unwrap().0, "us-east-1");
        assert!(
            n.references
                .iter()
                .all(|r| !super::datazone::indexed(&n.resource_type, &r.attribute))
        );
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment
                && super::resource(&g.nodes[e.to].resource_type))
    );
    assert!(!verified(&v, 9, "user_identifier"));
    assert!(!verified(&v, 13, "entity_identifier"));
    assert!(!verified(&v, 14, "principal.group.group_identifier"));
    for t in [0, 1, 19, 20, 21] {
        assert!(node(&g, &v, t).member_owner.is_none());
    }
}
#[test]
fn datazone_access_endpoints_require_explicit_exact_known_current_outputs() {
    for (t, p, path, s, out, wrong) in [
        (7, "domain_identifier", "domain_identifier", 2, "id", "arn"),
        (7, "user_identifier", "user_identifier", 0, "arn", "id"),
        (8, "user_identifier", "user_identifier", 1, "arn", "name"),
        (
            10,
            "entity_identifier",
            "entity_identifier",
            4,
            "name",
            "revision",
        ),
        (
            11,
            "entity_identifier",
            "entity_identifier",
            5,
            "id",
            "name",
        ),
        (
            12,
            "entity_identifier",
            "entity_identifier",
            6,
            "environment_blueprint_id",
            "id",
        ),
        (
            10,
            "principal.project.project_identifier",
            "principal.0.project.0.project_identifier",
            3,
            "id",
            "name",
        ),
        (
            11,
            "principal.user.user_identifier",
            "principal.0.user.0.user_identifier",
            7,
            "id",
            "user_identifier",
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
            "source-unknown",
            "consumer-delete",
        ] {
            let mut v = fixture();
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
                        .to_owned();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.{wrong}")]}),
                    );
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("different"),
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({out:true})
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
fn datazone_access_type_and_principal_unions_do_not_guess_users_groups_or_policy_effects() {
    for case in [
        "user-role-mismatch",
        "role-user-mismatch",
        "user-unknown",
        "role-session",
        "computed-type",
        "unknown-entity-type",
        "wrong-entity-type",
        "multiple-principal",
        "unknown-other-principal",
        "multiple-rows",
        "object-block",
        "unknown-designation",
        "wrong-designation",
        "all-users",
    ] {
        let mut v = fixture();
        let (t, p) = match case {
            "user-role-mismatch" => {
                v["resource_changes"][7]["change"]["after"]["user_type"] = json!("IAM_ROLE");
                (7, "user_identifier")
            }
            "role-user-mismatch" => {
                v["resource_changes"][8]["change"]["after"]["user_type"] = json!("IAM_USER");
                (8, "user_identifier")
            }
            "user-unknown" => {
                v["resource_changes"][7]["change"]["after_unknown"] = json!({"user_type":true});
                (7, "user_identifier")
            }
            "role-session" => {
                v["resource_changes"][8]["change"]["after"]["user_type"] =
                    json!("IAM_ROLE_SESSION");
                (8, "user_identifier")
            }
            "computed-type" => {
                v["resource_changes"][7]["change"]["after"]["user_type"] = Value::Null;
                v["resource_changes"][7]["change"]["after"]["type"] = json!("IAM");
                (7, "user_identifier")
            }
            "unknown-entity-type" => {
                v["resource_changes"][10]["change"]["after_unknown"] = json!({"entity_type":true});
                (10, "entity_identifier")
            }
            "wrong-entity-type" => {
                v["resource_changes"][10]["change"]["after"]["entity_type"] =
                    json!("ENVIRONMENT_PROFILE");
                (10, "entity_identifier")
            }
            "multiple-principal" => {
                v["resource_changes"][10]["change"]["after"]["principal"][0]["user"] =
                    json!([{"user_identifier":"user-profile"}]);
                (10, "principal.project.project_identifier")
            }
            "unknown-other-principal" => {
                v["resource_changes"][10]["change"]["after_unknown"] =
                    json!({"principal":[{"user":true}]});
                (10, "principal.project.project_identifier")
            }
            "multiple-rows" => {
                let row = v["resource_changes"][10]["change"]["after"]["principal"][0].clone();
                v["resource_changes"][10]["change"]["after"]["principal"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
                (10, "principal.project.project_identifier")
            }
            "object-block" => {
                v["resource_changes"][10]["change"]["after"]["principal"] =
                    v["resource_changes"][10]["change"]["after"]["principal"][0].clone();
                (10, "principal.project.project_identifier")
            }
            "unknown-designation" => {
                v["resource_changes"][10]["change"]["after_unknown"] =
                    json!({"principal":[{"project":[{"project_designation":true}]}]});
                (10, "principal.project.project_identifier")
            }
            "wrong-designation" => {
                v["resource_changes"][10]["change"]["after"]["principal"][0]["project"][0]["project_designation"] =
                    json!("ADMIN");
                (10, "principal.project.project_identifier")
            }
            "all-users" => {
                v["resource_changes"][11]["change"]["after"]["principal"][0]["user"][0]["all_users_grant_filter"] =
                    json!([{}]);
                (11, "principal.user.user_identifier")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
    let mut v = fixture();
    v["resource_changes"][10]["change"]["after"]["principal"][0]["project"][0]["domain_unit_filter"] =
        json!([{"domain_unit":"external","include_child_domain_units":true}]);
    assert!(verified(&v, 10, "principal.project.project_identifier"));
}
#[test]
fn datazone_access_parent_domain_identity_cannot_be_replaced_by_matching_strings() {
    for case in [
        "foreign-asset",
        "same-id-foreign-domain",
        "foreign-user",
        "unknown-user-domain",
        "unknown-project-domain",
        "unknown-config-domain",
    ] {
        let mut v = fixture();
        let (t, p) = match case {
            "foreign-asset" | "same-id-foreign-domain" => {
                v["configuration"]["root_module"]["resources"][10]["expressions"]["entity_identifier"] =
                    json!({"references":["aws_datazone_asset_type.foreign.name"]});
                if case == "same-id-foreign-domain" {
                    v["resource_changes"][15]["change"]["after"]["id"] = json!("dzd-main");
                    v["resource_changes"][17]["change"]["after"]["domain_identifier"] =
                        json!("dzd-main");
                }
                (10, "entity_identifier")
            }
            "foreign-user" => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]["principal"][0]
                    ["user"][0]["user_identifier"] =
                    json!({"references":["aws_datazone_user_profile.foreign.id"]});
                (11, "principal.user.user_identifier")
            }
            "unknown-user-domain" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (11, "principal.user.user_identifier")
            }
            "unknown-project-domain" => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"domain_identifier":true});
                (10, "principal.project.project_identifier")
            }
            "unknown-config-domain" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"domain_id":true});
                (12, "entity_identifier")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
#[test]
fn datazone_access_independent_accounts_regions_and_global_iam_identity_are_required() {
    for (s, t, p, global) in [
        (0, 7, "user_identifier", true),
        (1, 8, "user_identifier", true),
        (2, 10, "entity_identifier", false),
        (7, 11, "principal.user.user_identifier", false),
        (6, 12, "entity_identifier", false),
    ] {
        for axis in ["account", "region"] {
            let mut v = fixture();
            let mut alias = v["configuration"]["provider_config"]["aws"].clone();
            alias["alias"] = json!("foreign");
            if axis == "account" {
                alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                    json!("arn:aws:iam::999999999999:role/Test");
            } else {
                alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
            }
            v["configuration"]["provider_config"]["aws.foreign"] = alias;
            v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                json!("aws.foreign");
            assert_eq!(verified(&v, t, p), global && axis == "region", "{s} {axis}");
        }
    }
}
#[test]
fn datazone_access_lifecycle_data_filters_named_states_and_personal_data_remain_separate() {
    for t in 7..=14 {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"domain,user"});
        v["resource_changes"][t]["previous_address"] = json!("aws_datazone_user_profile.old");
        assert_eq!(graph(&v).coverage.selected.resources, 22);
    }
    let mut v = fixture();
    v["resource_changes"][7]["mode"] = json!("data");
    assert!(node(&graph(&v), &v, 7).member_owner.is_none());
    assert!(!verified(&v, 11, "principal.user.user_identifier"));
    let v = fixture();
    let g = graph(&v);
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_datazone_policy_grant.profile".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
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
