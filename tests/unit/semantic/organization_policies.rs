use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/organization-policies-plan.json");

#[test]
fn direct_policy_attachments_keep_each_resource_and_partial_evidence() {
    for case in 0..9 {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            1 => input["resource_changes"][5]["provider_name"] = json!("example.com/foreign/aws"),
            2 => input["resource_changes"][5]["type"] = json!("aws_iam_policy"),
            3 => input["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            4 => input["resource_changes"][5]["deposed"] = json!("old"),
            5 => {
                input["resource_changes"][7]["change"]["after_unknown"] = json!({"policy_id":true})
            }
            6 => {
                input["configuration"]["root_module"]["resources"][7]["expressions"]["policy_id"] =
                    json!({"constant_value":"p-guardrails"})
            }
            7 => {
                input["configuration"]["root_module"]["resources"][7]["expressions"]["policy_id"] =
                    json!({"references":["aws_organizations_policy.guardrails.id", "var.dynamic"]})
            }
            8 => input["resource_changes"][5]["mode"] = json!("data"),
            _ => {}
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        let attachment = graph
            .nodes
            .iter()
            .find(|node| node.address == "aws_organizations_policy_attachment.root")
            .unwrap();
        assert_eq!(
            attachment
                .references
                .iter()
                .any(|r| r.label == "Direct attachment policy" && r.complete),
            matches!(case, 0 | 8),
            "case {case}"
        );
        assert!(
            attachment
                .references
                .iter()
                .any(|r| r.label == "Direct attachment target" && r.complete)
        );
        assert_eq!(graph.coverage.selected.resources, 11);
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.resource_type == "aws_organizations_policy_attachment")
                .count(),
            4
        );
    }
}

#[test]
fn policy_types_are_explicit_and_never_include_content() {
    let svg = crate::generate(FIXTURE, &Default::default())
        .unwrap()
        .content;
    assert!(svg.contains("Policy type: SERVICE_CONTROL_POLICY (SCP)"));
    assert!(svg.contains("Policy type: TAG_POLICY"));
    assert!(svg.contains("Direct attachment target"));
    assert!(!svg.contains("PRIVATE_"));
    let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
    input["resource_changes"][5]["change"]["after"]["type"] = json!("PRIVATE_UNKNOWN_TYPE");
    let svg = crate::generate(&input.to_string(), &Default::default())
        .unwrap()
        .content;
    assert!(svg.contains("Policy type: unknown or unsupported"));
    assert!(!svg.contains("PRIVATE_"));
}
