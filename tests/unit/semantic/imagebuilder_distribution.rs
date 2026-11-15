use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/imagebuilder-distribution-plan.json");
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
fn pointer(path: &str) -> String {
    format!("/{}", path.replace('.', "/"))
}
fn set(v: &mut Value, p: &str, value: Value) {
    *v.pointer_mut(&pointer(p)).unwrap() = value;
}
#[test]
fn imagebuilder_distribution_verifies_each_destination_and_keeps_configuration_scope() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for p in [
        super::KMS,
        super::ECR,
        super::LT,
        super::BUCKET,
        super::ROLE,
        super::PARAMETER,
    ] {
        assert!(verified(&v, 9, p), "{p}");
    }
    for t in [10, 11] {
        for p in ["execution_role", super::NAME, super::VERSION] {
            assert!(verified(&v, t, p), "{t} {p}");
        }
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 13);
    assert_eq!(g.nodes.len(), 13);
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(n.placement, Placement::Logical);
        assert!(n.member_owner.is_none());
        assert_eq!(
            n.entity.scope.as_ref().unwrap().region,
            Some(crate::model::RegionScope("us-east-1".into()))
        );
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
    }
}
#[test]
fn imagebuilder_distribution_rejects_incomplete_unknown_sensitive_or_wrong_outputs() {
    for (path, canonical, s, out) in [
        (
            "distribution.0.ami_distribution_configuration.0.kms_key_id",
            super::KMS,
            0,
            "arn",
        ),
        (
            "distribution.0.container_distribution_configuration.0.target_repository.0.repository_name",
            super::ECR,
            2,
            "name",
        ),
        (
            "distribution.0.launch_template_configuration.0.launch_template_id",
            super::LT,
            3,
            "id",
        ),
        (
            "distribution.0.s3_export_configuration.0.s3_bucket",
            super::BUCKET,
            4,
            "bucket",
        ),
        (
            "distribution.0.s3_export_configuration.0.role_name",
            super::ROLE,
            5,
            "name",
        ),
        (
            "distribution.0.ssm_parameter_configuration.0.parameter_name",
            super::PARAMETER,
            6,
            "name",
        ),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "mismatch",
            "output",
            "data",
            "delete",
            "deposed",
            "provider",
            "source-sensitive",
            "shape",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v,
                    &format!("configuration.root_module.resources.9.expressions.{path}"),
                    json!({"constant_value":"literal"}),
                ),
                "unknown" | "sensitive" => {
                    v["resource_changes"][9]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({"distribution":true})
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.9.expressions.{path}"),
                        json!({"references":[format!("{a}.unsupported")]}),
                    );
                }
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => v["resource_changes"][s]["provider_name"] = json!("foreign/aws"),
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                "shape" => {
                    v["resource_changes"][9]["change"]["after"]["distribution"] =
                        v["resource_changes"][9]["change"]["after"]["distribution"][0].clone()
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, 9, canonical), "{canonical} {case}");
        }
    }
}
#[test]
fn imagebuilder_distribution_qualifiers_are_per_entry_and_never_inferred_from_names() {
    for case in [
        "second-region",
        "unknown-region",
        "repository-service",
        "missing-member",
        "literal-account",
        "wrong-account",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let path = match case {
            "second-region" => {
                v["resource_changes"][9]["change"]["after"]["distribution"][1]["region"] =
                    json!("us-west-2");
                super::KMS
            }
            "unknown-region" => {
                v["resource_changes"][9]["change"]["after_unknown"] =
                    json!({"distribution":[{}, {"region":true}]});
                super::KMS
            }
            "repository-service" => {
                set(
                    &mut v,
                    "resource_changes.9.change.after.distribution.0.container_distribution_configuration.0.target_repository.0.service",
                    json!("OTHER"),
                );
                super::ECR
            }
            "missing-member" => {
                v["resource_changes"][9]["change"]["after"]["distribution"].as_array_mut().unwrap().push(json!({"region":"us-east-1","ami_distribution_configuration":[{"kms_key_id":"external"}]}));
                super::KMS
            }
            "literal-account" => {
                set(
                    &mut v,
                    "resource_changes.9.change.after.distribution.0.ssm_parameter_configuration.0.ami_account_id",
                    json!("999999999999"),
                );
                super::PARAMETER
            }
            "wrong-account" => {
                set(
                    &mut v,
                    "resource_changes.9.change.after.distribution.0.launch_template_configuration.0.account_id",
                    json!("999999999999"),
                );
                super::LT
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, 9, path), "{case}");
    }
    let raw = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let d = raw
        .nodes
        .iter()
        .find(|n| n.resource_type == super::DIST)
        .unwrap();
    assert!(!d.references.iter().any(|r| r.attribute == super::ECR
        && r.label == "Distribution repositories"
        && r.complete));
}
#[test]
fn imagebuilder_distribution_allows_only_explicit_verified_foreign_destinations() {
    for qualified in [false, true] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let mut other = v["configuration"]["provider_config"]["aws.west"].clone();
        other["alias"] = json!("foreign");
        other["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
            json!("arn:aws:iam::999999999999:role/Test");
        v["configuration"]["provider_config"]["aws.foreign"] = other;
        for t in [0, 3, 6] {
            v["configuration"]["root_module"]["resources"][t]["provider_config_key"] =
                json!("aws.foreign");
        }
        if qualified {
            v["resource_changes"][0]["change"]["after"]["arn"] =
                json!("arn:aws:kms:us-west-2:999999999999:key/west");
            set(
                &mut v,
                "resource_changes.9.change.after.distribution.0.ami_distribution_configuration.0.kms_key_id",
                json!("arn:aws:kms:us-west-2:999999999999:key/west"),
            );
            set(
                &mut v,
                "resource_changes.9.change.after.distribution.0.ami_distribution_configuration.0.target_account_ids",
                json!(["999999999999"]),
            );
            set(
                &mut v,
                "resource_changes.9.change.after.distribution.0.launch_template_configuration.0.account_id",
                json!("999999999999"),
            );
            set(
                &mut v,
                "resource_changes.9.change.after.distribution.0.ssm_parameter_configuration.0.ami_account_id",
                json!("999999999999"),
            );
        }
        for p in [super::KMS, super::LT, super::PARAMETER] {
            assert_eq!(verified(&v, 9, p), qualified, "{p}");
        }
        assert_eq!(
            graph(&v)
                .nodes
                .iter()
                .find(|n| n.resource_type == super::DIST)
                .unwrap()
                .entity
                .scope
                .as_ref()
                .unwrap()
                .account,
            crate::model::AwsAccountScope::new("123456789012")
        );
    }
}
#[test]
fn imagebuilder_lifecycle_requires_type_name_and_exact_version_for_every_recipe() {
    for case in [
        "type",
        "name",
        "version",
        "wildcard",
        "unknown",
        "sensitive",
        "partial",
        "tag-only",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "type" => {
                v["resource_changes"][10]["change"]["after"]["resource_type"] =
                    json!("CONTAINER_IMAGE")
            }
            "name" => v["resource_changes"][7]["change"]["after"]["name"] = json!("other"),
            "version" => v["resource_changes"][7]["change"]["after"]["version"] = json!("3.0.0"),
            "wildcard" => {
                v["resource_changes"][7]["change"]["after"]["version"] = json!("x.x.x");
                set(
                    &mut v,
                    "resource_changes.10.change.after.resource_selection.0.recipe.0.semantic_version",
                    json!("x.x.x"),
                );
            }
            "unknown" | "sensitive" => {
                v["resource_changes"][10]["change"][if case == "unknown" {
                    "after_unknown"
                } else {
                    "after_sensitive"
                }] = json!({"resource_selection":[{"recipe":[{"semantic_version":true},{}]}]})
            }
            "partial" => {
                v["resource_changes"][10]["change"]["after"]["resource_selection"][0]["recipe"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"external","semantic_version":"1.0.0"}))
            }
            "tag-only" => {
                v["resource_changes"][10]["change"]["after"]["resource_selection"] =
                    json!([{"tag_map":{"private":"PRIVATE_TAG"}}])
            }
            _ => unreachable!(),
        }
        for p in [super::NAME, super::VERSION] {
            assert!(!verified(&v, 10, p), "{case} {p}");
        }
        assert!(verified(&v, 10, "execution_role"));
    }
}
#[test]
fn imagebuilder_distribution_filter_and_named_states_preserve_safe_coverage() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_imagebuilder_distribution_configuration.build".into()),
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
