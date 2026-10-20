use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn policy() -> Value {
    serde_json::from_str(include_str!("../../fixtures/s3-policy-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn reference<'a>(graph: &'a Graph, address: &str, attribute: &str) -> &'a ResourceReference {
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .find(|r| r.attribute == attribute && r.label != "References")
        .unwrap()
}

#[test]
fn s3_policy_preserves_configuration_identity_and_bucket_reference() {
    let result = graph(&policy());
    assert_eq!(result.coverage.selected.resources, 3);
    assert_eq!(result.nodes.len(), 3);
    let policy = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_policy.assets")
        .unwrap();
    assert_eq!(policy.placement, Placement::Logical);
    assert_eq!(policy.role, ResourceRole::Policy);
    assert_eq!(policy.action, Action::Update);
    assert_eq!(
        policy
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "us-west-2"
    );
    let bucket = reference(&result, "aws_s3_bucket_policy.assets", "bucket");
    assert!(bucket.complete);
    assert_eq!(bucket.targets[0].id.address, "aws_s3_bucket.assets[0]");
    assert!(!reference(&result, "aws_s3_bucket_policy.literal", "bucket").complete);
    assert!(result.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
}

#[test]
fn s3_policy_requires_unique_current_typed_and_evaluated_bucket() {
    for case in 0..12 {
        let mut value = policy();
        match case {
            0 => {
                value["resource_changes"][0]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            1 => value["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
            2 => value["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
            3 => value["resource_changes"][0]["change"]["after"]["id"] = json!("different"),
            4 => value["resource_changes"][0]["mode"] = json!("data"),
            5 => value["resource_changes"][0]["deposed"] = json!("old"),
            6 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            7 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            8 => value["resource_changes"][1]["change"]["after_unknown"] = json!({"bucket":true}),
            9 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["bucket"] =
                    json!({"references":["aws_s3_bucket.assets[count.index].id"]})
            }
            10 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["bucket"] =
                    json!({"references":["aws_s3_bucket.assets[0].id","var.missing"]})
            }
            _ => {
                value["resource_changes"].as_array_mut().unwrap().remove(0);
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            !reference(
                &semantic::transform(&parsed),
                "aws_s3_bucket_policy.assets",
                "bucket"
            )
            .complete,
            "case {case}"
        );
        assert!(
            semantic::diagnostics::collect(&parsed)
                .iter()
                .any(|d| d.address == "aws_s3_bucket_policy.assets"
                    && d.reason == DiagnosticReason::UnprovenEndpointValue)
        );
    }
}

#[test]
fn s3_policy_keeps_lifecycle_selection_state_and_private_policy_contents() {
    let mut value = policy();
    value["resource_changes"][1]["previous_address"] = json!("aws_s3_bucket_policy.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    value["resource_changes"][1]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let result = graph(&value);
    assert_eq!(result.coverage.selected.resources, 4);
    let current = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_policy.assets" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(current.action, Action::Replace);
    assert!(current.previous_address.is_some() && current.metadata.import.is_some());
    assert!(
        result
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        for icon_mode in [
            crate::icons::IconMode::Icon,
            crate::icons::IconMode::ServiceName,
        ] {
            let output = crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    icon_mode,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!output.contains("PRIVATE_"));
            assert!(!output.contains("fixture-assets"));
            assert!(output.contains("Policy bucket reference"));
        }
    }
    let selected = crate::view::single(
        &graph(&policy()),
        &crate::view::Options {
            focus: Some("aws_s3_bucket_policy.assets".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut missing = policy();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let states = plan::parse_inputs(
        [("one", policy()), ("two", missing)]
            .into_iter()
            .map(|(name, value)| PlanInput {
                state_id: StateId::new(name).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, parsed) in states.states {
        assert_eq!(
            reference(
                &semantic::transform(&parsed),
                "aws_s3_bucket_policy.assets",
                "bucket"
            )
            .complete,
            state.as_str() == "one"
        );
    }
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"]["module_calls"] =
        json!({"storage":{"module":{"resources":resources}}});
    for node in value["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!(
            "module.storage.{}",
            node["address"].as_str().unwrap()
        ));
        node["module_address"] = json!("module.storage");
    }
    let output = crate::generate(
        &value.to_string(),
        &crate::Options {
            modules: vec!["module.storage".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("4 resources"));
    assert!(!output.contains("PRIVATE_"));
}

#[test]
fn s3_policy_indexed_ambiguity_and_foreign_consumers_keep_safe_fallbacks() {
    let mut value = policy();
    let mut second = value["resource_changes"][0].clone();
    second["address"] = json!("aws_s3_bucket.assets[1]");
    second["index"] = json!(1);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(reference(&graph(&value), "aws_s3_bucket_policy.assets", "bucket").complete);
    value["configuration"]["root_module"]["resources"][1]["expressions"]["bucket"] =
        json!({"references":["aws_s3_bucket.assets.id"]});
    assert!(!reference(&graph(&value), "aws_s3_bucket_policy.assets", "bucket").complete);
    let mut value = policy();
    value["resource_changes"][1]["provider_name"] = json!("registry.terraform.io/other/aws");
    let result = graph(&value);
    let node = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_policy.assets")
        .unwrap();
    assert_eq!(node.placement, Placement::Unresolved);
    assert!(
        !node
            .references
            .iter()
            .any(|r| r.label == "Policy bucket reference")
    );
}
