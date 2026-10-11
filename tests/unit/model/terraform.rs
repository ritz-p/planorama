use super::*;

#[test]
fn lifecycle_facts_are_independent_and_value_free() {
    let input = r#"{"format_version":"1.2","resource_changes":[
        {"address":"custom_item.x","type":"custom_item","previous_address":"custom_item.old","change":{"actions":["no-op"],"before":{"secret":"TOP_SECRET"},"after":{"secret":"TOP_SECRET"},"importing":{"id":"TOP_SECRET"}}},
        {"address":"custom_item.x","type":"custom_item","deposed":"old","change":{"actions":["delete"]}}
    ]}"#;
    let plan: TerraformPlan = crate::plan::parse(input).unwrap();
    assert_eq!(plan, crate::plan::parse(input).unwrap());
    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(
        plan.nodes[0].previous_address.as_deref(),
        Some("custom_item.old")
    );
    assert!(plan.nodes[0].metadata.import.unwrap().has_id);
    assert_eq!(plan.nodes[1].deposed_key.as_deref(), Some("old"));
    assert!(!format!("{plan:?}").contains("TOP_SECRET"));
    let before = plan.clone();
    let graph = crate::semantic::transform(&plan);
    assert_eq!(plan, before);
    assert_eq!(graph.nodes[0].metadata, plan.nodes[0].metadata);
    assert_eq!(graph.nodes[1].action, Action::Delete);
}
