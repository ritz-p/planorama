use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/backup-plan.json");

#[test]
fn backup_preserves_distinct_identity_and_only_explicit_resource_collections() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    let explicit = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_backup_selection.explicit")
        .unwrap();
    let resources = explicit
        .references
        .iter()
        .find(|r| r.label == "Explicit backup resource reference")
        .unwrap();
    assert!(resources.complete);
    assert_eq!(resources.targets.len(), 2);
    let tagged = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_backup_selection.tagged")
        .unwrap();
    assert!(!tagged.references.iter().any(|r| r.attribute == "resources"));
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.resource_type.starts_with("aws_backup_"))
            .all(|n| n.member_owner.is_none())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(output.contains("planorama-icon-aws-backup"));
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
    }
}

#[test]
fn backup_resource_proof_rejects_partial_wildcard_and_filtered_selections() {
    for case in [
        "wildcard",
        "literal-extra",
        "unknown",
        "source-unknown",
        "wrong-provider",
        "wrong-type",
        "delete",
        "deposed",
        "data",
        "condition",
        "exclusion",
        "tags",
        "ambiguous",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "wildcard" => {
                value["resource_changes"][5]["change"]["after"]["resources"][0] =
                    json!("arn:aws:s3:::*")
            }
            "literal-extra" => value["resource_changes"][5]["change"]["after"]["resources"]
                .as_array_mut()
                .unwrap()
                .push(json!("arn:aws:s3:::extra")),
            "unknown" => {
                value["resource_changes"][5]["change"]["after_unknown"] = json!({"resources":true})
            }
            "source-unknown" => {
                value["resource_changes"][3]["change"]["after_unknown"] = json!({"arn":true})
            }
            "wrong-provider" => {
                value["resource_changes"][3]["provider_name"] = json!("example.com/foreign/aws")
            }
            "wrong-type" => value["resource_changes"][3]["type"] = json!("aws_iam_role"),
            "delete" => value["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            "deposed" => value["resource_changes"][3]["deposed"] = json!("old"),
            "data" => value["resource_changes"][3]["mode"] = json!("data"),
            "condition" => {
                value["resource_changes"][5]["change"]["after"]["condition"] =
                    json!([{"string_equals":[{"key":"tag","value":"value"}]}])
            }
            "exclusion" => {
                value["resource_changes"][5]["change"]["after"]["not_resources"] = json!(["*"])
            }
            "tags" => {
                value["resource_changes"][5]["change"]["after"]["selection_tag"] =
                    json!([{"key":"tag","value":"value"}])
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["resources"] =
                    json!({"references":["aws_s3_bucket.data.arn","var.targets"]})
            }
            _ => unreachable!(),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .filter(|r| r.attribute == "resources")
                .all(|r| !r.service_reference_verified),
            "{case}"
        );
    }
}

#[test]
fn backup_single_rule_proof_and_filtered_indexed_state_identity() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let rule = value["resource_changes"][1]["change"]["after"]["rule"][0].clone();
    value["resource_changes"][1]["change"]["after"]["rule"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(
        parsed
            .attributes
            .iter()
            .filter(|r| r.attribute == "rule.target_vault_name")
            .all(|r| !r.service_reference_verified)
    );
    let indexed = FIXTURE.replace("aws_s3_bucket.data", "aws_s3_bucket.data[0]");
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_backup_selection.explicit".into()),
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
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
