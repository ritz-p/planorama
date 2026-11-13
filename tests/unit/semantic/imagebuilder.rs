use crate::{
    model::{EdgeKind, Graph},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/imagebuilder-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
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
fn placed(g: &Graph) -> bool {
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.to].resource_type == "aws_imagebuilder_infrastructure_configuration"
            && g.nodes[e.from].resource_type == "aws_vpc"
    })
}
#[test]
fn imagebuilder_exact_references_keep_reusable_inputs_and_outputs_independent() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (11, "kms_key_id"),
        (13, "kms_key_id"),
        (15, "subnet_id"),
        (15, "security_group_ids"),
        (15, "instance_profile_name"),
        (15, "sns_topic_arn"),
        (15, "logging.s3_logs.s3_bucket_name"),
        (17, "component.component_arn"),
        (17, "parent_image"),
        (17, "block_device_mapping.ebs.kms_key_id"),
        (17, "block_device_mapping.ebs.snapshot_id"),
        (18, "component.component_arn"),
        (18, "parent_image"),
        (18, "target_repository.repository_name"),
        (18, "kms_key_id"),
        (
            18,
            "instance_configuration.block_device_mapping.ebs.kms_key_id",
        ),
        (
            18,
            "instance_configuration.block_device_mapping.ebs.snapshot_id",
        ),
        (20, "image_recipe_arn"),
        (20, "infrastructure_configuration_arn"),
        (20, "distribution_configuration_arn"),
        (20, "execution_role"),
        (20, "workflow.workflow_arn"),
        (21, "image_recipe_arn"),
        (21, "workflow.workflow_arn"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 22);
    assert_eq!(g.nodes.len(), 22);
    assert!(placed(&g));
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert!(n.member_owner.is_none());
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
        assert_eq!(n.placement, crate::model::Placement::Logical);
        if n.resource_type != "aws_imagebuilder_infrastructure_configuration" {
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == EdgeKind::Containment
                        && g.nodes[e.to].entity.id == n.entity.id)
            );
        }
    }
}
#[test]
fn imagebuilder_scalar_proof_requires_exact_known_current_outputs() {
    for (t, s, p, out) in [
        (11, 5, "kms_key_id", "arn"),
        (15, 1, "subnet_id", "id"),
        (15, 2, "security_group_ids", "id"),
        (15, 4, "instance_profile_name", "name"),
        (15, 6, "sns_topic_arn", "arn"),
        (17, 9, "parent_image", "id"),
        (20, 17, "image_recipe_arn", "arn"),
        (20, 15, "infrastructure_configuration_arn", "arn"),
        (20, 16, "distribution_configuration_arn", "arn"),
        (20, 3, "execution_role", "arn"),
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
                    if s == 3 || s == 4 {
                        continue;
                    }
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":"us-west-2"}}});
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
fn imagebuilder_nested_lists_require_complete_rows_and_exact_versions() {
    for (t, block, leaf, source) in [
        (17, "component", "component_arn", 11),
        (20, "workflow", "workflow_arn", 13),
    ] {
        for case in [
            "partial",
            "unknown",
            "sensitive",
            "literal",
            "wildcard",
            "object",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "partial" => v["resource_changes"][t]["change"]["after"][block]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({leaf:"external"})),
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({block:[{leaf:true},{}]})
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"][block][0]
                        [leaf] = json!({"constant_value":"literal"})
                }
                "wildcard" => {
                    let arn = v["resource_changes"][source]["change"]["after"]["arn"]
                        .as_str()
                        .unwrap()
                        .replace("1.0.0", "x.x.x");
                    v["resource_changes"][source]["change"]["after"]["arn"] = json!(arn);
                    v["resource_changes"][t]["change"]["after"][block][0][leaf] = json!(arn);
                }
                "object" => {
                    v["resource_changes"][t]["change"]["after"][block] =
                        v["resource_changes"][t]["change"]["after"][block][0].clone()
                }
                _ => unreachable!(),
            }
            assert!(
                !verified(&v, t, &format!("{block}.{leaf}")),
                "{block} {case}"
            );
        }
    }
}
#[test]
fn imagebuilder_recipe_variants_and_ebs_modes_are_independent() {
    for value in [json!(true), json!("false"), json!(null)] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][17]["change"]["after"]["block_device_mapping"][0]["ebs"][0]["encrypted"] =
            value;
        assert!(!verified(&v, 17, "block_device_mapping.ebs.kms_key_id"));
        assert!(verified(&v, 17, "block_device_mapping.ebs.snapshot_id"));
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][20]["change"]["after"]["container_recipe_arn"] =
        v["resource_changes"][18]["change"]["after"]["arn"].clone();
    v["configuration"]["root_module"]["resources"][20]["expressions"]["container_recipe_arn"] =
        json!({"references":["aws_imagebuilder_container_recipe.build.arn"]});
    assert!(!verified(&v, 20, "image_recipe_arn"));
    assert!(!verified(&v, 20, "container_recipe_arn"));
    v["resource_changes"][20]["change"]["after"]["image_recipe_arn"] = Value::Null;
    assert!(verified(&v, 20, "container_recipe_arn"));
    for value in [json!("OTHER"), json!(null)] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][18]["change"]["after"]["target_repository"][0]["service"] = value;
        assert!(!verified(&v, 18, "target_repository.repository_name"));
    }
}
#[test]
fn imagebuilder_network_requires_common_vpc_and_visible_complete_security_groups() {
    for case in ["known", "partial", "foreign", "unknown", "filtered"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "partial" => {
                v["resource_changes"][15]["change"]["after"]["security_group_ids"] =
                    json!(["sg-build", "external"])
            }
            "foreign" => v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            "unknown" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"security_group_ids":true})
            }
            "filtered" => v["resource_changes"][2]["change"]["actions"] = json!(["no-op"]),
            _ => (),
        }
        assert!(verified(&v, 15, "subnet_id"));
        let g = graph(&v);
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                changed_only: case == "filtered",
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(placed(&selected), case == "known", "{case}");
    }
}
#[test]
fn imagebuilder_filter_named_states_and_private_payloads_preserve_identity() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let g = graph(&v);
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_imagebuilder_image_pipeline.build".into()),
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
