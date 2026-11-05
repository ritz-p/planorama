use crate::{plan, semantic};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/backup-plan-rules-plan.json")).unwrap()
}
fn complete(v: &Value, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes
        .iter()
        .any(|r| r.attribute == path && super::super::service_references::verified(&raw, r))
}
#[test]
fn complete_rule_and_scan_sets_match_explicit_endpoint_outputs() {
    for (path, source, output) in [
        ("rule.target_vault_name", 0, "name"),
        (
            "rule.target_logically_air_gapped_backup_vault_arn",
            2,
            "arn",
        ),
        (super::COPY, 1, "arn"),
        (super::ROLE, 3, "arn"),
    ] {
        assert!(complete(&fixture(), path), "{path}");
        for case in [
            "literal",
            "partial",
            "ambiguous",
            "unknown",
            "source-unknown",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "wrong-type",
            "scope",
        ] {
            let mut v = fixture();
            let first = path.split('.').next().unwrap();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][4]["expressions"]
                        .as_object_mut()
                        .unwrap()
                        .remove(first);
                }
                "partial" => {
                    v["configuration"]["root_module"]["resources"][4]["expressions"][first] =
                        json!([{}])
                }
                "ambiguous" => {
                    let mut expression =
                        &mut v["configuration"]["root_module"]["resources"][4]["expressions"];
                    for field in path.split('.') {
                        expression = &mut expression[field];
                        if expression.is_array() {
                            expression = &mut expression[0];
                        }
                    }
                    expression["references"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("aws_backup_plan.main.arn"));
                }
                "unknown" => {
                    v["resource_changes"][4]["change"]["after_unknown"] = json!({first:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("wrong")
                }
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "wrong-type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                _ => {
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("unknown")
                }
            }
            assert!(!complete(&v, path), "{path} {case}");
        }
    }
}
#[test]
fn collection_order_duplicates_masks_and_partial_provenance_are_conservative() {
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"]["rule"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(complete(&v, "rule.target_vault_name"));
    v = fixture();
    v["resource_changes"][4]["change"]["after_unknown"] = json!({"rule":[{}]});
    assert!(!complete(&v, "rule.target_vault_name"));
    v = fixture();
    v["resource_changes"][4]["change"]["after"]["rule"][1]["target_vault_name"] =
        json!("literal-only");
    assert!(!complete(&v, "rule.target_vault_name"));
    v = fixture();
    v["configuration"]["root_module"]["resources"][4]["expressions"]["rule"][1]["target_vault_name"] =
        json!({"constant_value":"second"});
    assert!(!complete(&v, "rule.target_vault_name"));
    v = fixture();
    v["resource_changes"][4]["change"]["after"]["scan_setting"][0]["malware_scanner"] =
        json!("UNKNOWN");
    assert!(!complete(&v, super::ROLE));
    let g = semantic::transform(&plan::parse(&fixture().to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 5);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
}
#[test]
fn cross_region_copy_uses_destination_scope_and_independent_account_evidence() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws.west"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.west"]["alias"] = json!("west");
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws.west");
    let arn = "arn:aws:backup:us-west-2:123456789012:backup-vault:second";
    v["resource_changes"][1]["change"]["after"]["arn"] = json!(arn);
    for i in 0..2 {
        v["resource_changes"][4]["change"]["after"]["rule"][i]["copy_action"] =
            json!([{"destination_vault_arn":arn}]);
        v["configuration"]["root_module"]["resources"][4]["expressions"]["rule"][i]["copy_action"] =
            json!([{"destination_vault_arn":{"references":["aws_backup_vault.second.arn"]}}]);
    }
    assert!(!complete(&v, super::COPY));
    let p = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    let r = raw
        .attributes
        .iter()
        .find(|r| r.attribute == super::COPY)
        .unwrap();
    assert!(super::super::service_references::verified(&raw, r));
    let mut p = p;
    for node in &mut p.nodes {
        if node.address == "aws_backup_vault.second" {
            node.provider_configuration.as_mut().unwrap().account =
                crate::model::AwsAccountScope::new("210987654321");
        }
    }
    let raw = semantic::Input::new(&p);
    let r = raw
        .attributes
        .iter()
        .find(|r| r.attribute == super::COPY)
        .unwrap();
    assert!(!super::super::service_references::verified(&raw, r));
}

#[test]
fn filtered_plan_keeps_exact_coverage_and_reference_fallback() {
    let graph = semantic::transform(&plan::parse(&fixture().to_string()).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_backup_plan.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    let svg = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
    assert!(svg.contains("Unavailable"));
    assert!(!svg.contains("arn:aws:"));
}
