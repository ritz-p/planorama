use crate::{layout::Layout, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/lambda-plan.json")).unwrap()
}

#[test]
fn lambda_links_require_complete_current_provider_and_version_evidence() {
    for case in [
        "valid",
        "literal",
        "unknown",
        "wrong_provider",
        "wrong_type",
        "deleted",
        "deposed",
        "data",
        "ambiguous",
        "version_mismatch",
        "qualifier_mismatch",
        "partial_layers",
        "unknown_layer",
        "malformed_unknown",
        "unknown_region",
        "literal_function",
        "literal_version_owner",
        "wrong_version_owner",
        "moved_imported",
        "historical_alias",
    ] {
        let mut value = input();
        match case {
            "historical_alias" => {
                value["resource_changes"][3]["deposed"] = json!("old");
                let change = &mut value["resource_changes"][3]["change"];
                change["before"] = change["after"].take();
                change["actions"] = json!(["delete"]);
            }
            "malformed_unknown" => {
                value["resource_changes"][1]["change"]["after_unknown"]["arn"] =
                    json!({"invalid":true})
            }
            "unknown_layer" => {
                value["resource_changes"][1]["change"]["after_unknown"]["arn"] = json!(true)
            }
            "unknown_region" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            "literal_function" => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["function_name"] =
                    json!({"constant_value":"synthetic-function"})
            }
            "literal_version_owner" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["function_name"] =
                    json!({"constant_value":"synthetic-function"})
            }
            "wrong_version_owner" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["version_number"] = json!({"references":["aws_lambda_layer_version.shared[1].version","aws_lambda_layer_version.shared[1]"]})
            }
            "moved_imported" => {
                value["resource_changes"][3]["previous_address"] = json!("aws_lambda_alias.old");
                value["resource_changes"][3]["change"]["importing"] =
                    json!({"id":"SYNTHETIC_IMPORT_ID"});
            }
            "literal" => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["layers"] = json!({"constant_value": value["resource_changes"][0]["change"]["after"]["layers"]})
            }
            "unknown" => {
                value["resource_changes"][0]["change"]["after_unknown"]["layers"] =
                    json!([false, true])
            }
            "wrong_provider" => {
                value["resource_changes"][1]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "wrong_type" => value["resource_changes"][1]["type"] = json!("aws_s3_bucket"),
            "deleted" => {
                let change = &mut value["resource_changes"][1]["change"];
                change["before"] = change["after"].take();
                change["actions"] = json!(["delete"]);
            }
            "deposed" => value["resource_changes"][1]["deposed"] = json!("old"),
            "data" => value["resource_changes"][1]["mode"] = json!("data"),
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["layers"] =
                    json!({"references":["aws_lambda_layer_version.shared.arn"]})
            }
            "version_mismatch" => {
                value["resource_changes"][6]["change"]["after"]["version_number"] = json!(2)
            }
            "qualifier_mismatch" => {
                value["resource_changes"][5]["change"]["after"]["qualifier"] = json!("canary")
            }
            "partial_layers" => value["resource_changes"][0]["change"]["after"]["layers"]
                .as_array_mut()
                .unwrap()
                .push(json!("unreferenced")),
            _ => {}
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        let node = |address: &str| graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(graph.coverage.selected.resources, 8, "{case}");
        let layers = node("aws_lambda_function.app")
            .references
            .iter()
            .find(|r| r.label == "Layer versions");
        assert_eq!(
            layers.is_some_and(|r| r.complete && r.targets.len() == 2),
            matches!(
                case,
                "valid"
                    | "version_mismatch"
                    | "qualifier_mismatch"
                    | "literal_function"
                    | "literal_version_owner"
                    | "wrong_version_owner"
                    | "moved_imported"
                    | "historical_alias"
            ),
            "{case}: {:?}",
            raw.attributes
        );
        if matches!(case, "version_mismatch" | "wrong_version_owner") {
            assert!(
                node("aws_lambda_layer_version_permission.shared")
                    .references
                    .iter()
                    .filter(|r| r.label == "Permission layer version")
                    .all(|r| !r.complete)
            );
        }
        if matches!(
            case,
            "qualifier_mismatch" | "literal_function" | "literal_version_owner"
        ) {
            assert!(
                node("aws_lambda_function_url.live")
                    .references
                    .iter()
                    .filter(|r| r.label == "Function URL alias")
                    .all(|r| !r.complete)
            );
        }
        if matches!(case, "valid" | "moved_imported") {
            for (index, label) in [
                ("aws_lambda_alias.live", "Published function version"),
                ("aws_lambda_function_url.live", "Function URL alias"),
                (
                    "aws_lambda_layer_version_permission.shared",
                    "Permission layer version",
                ),
            ] {
                assert!(
                    node(index)
                        .references
                        .iter()
                        .any(|r| r.label == label && r.complete && r.targets.len() == 1),
                    "{label}"
                );
            }
        }
        if matches!(case, "literal_version_owner" | "historical_alias") {
            assert!(
                node("aws_lambda_alias.live")
                    .references
                    .iter()
                    .filter(|r| r.label == "Published function version")
                    .all(|r| !r.complete)
            );
        }
        assert!(
            node("aws_lambda_function_url.unresolved")
                .references
                .iter()
                .all(|r| r.label != "Lambda function" || !r.complete)
        );
    }
}

#[test]
fn lambda_rendering_preserves_identity_coverage_and_redacts_payloads() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let graph = crate::generation::project(&raw, &view::Options::default()).unwrap();
    let output = svg::render(&graph, &Layout::new(&graph));
    assert!(!output.contains("NOT_FOR_OUTPUT"));
    for node in &graph.nodes {
        assert!(output.contains(&svg::resource_id(node)));
    }
    assert_eq!(graph.coverage.selected.resources, 8);
    let selected = crate::generation::project(
        &raw,
        &view::Options {
            focus: Some("aws_lambda_function.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let output = svg::render(&selected, &Layout::new(&selected));
    let doc = roxmltree::Document::parse(&output).unwrap();
    for link in doc
        .descendants()
        .filter_map(|n| n.attribute("href"))
        .filter_map(|v| v.strip_prefix('#'))
    {
        assert!(doc.descendants().any(|n| n.attribute("id") == Some(link)));
    }
}
