use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/access-analyzer-plan.json"),
        crate::plan::parse,
        super::label,
        &[("aws_accessanalyzer_archive_rule.sample0", "analyzer_name")],
        ("aws_accessanalyzer_analyzer.sample1", "analyzer_name"),
    );
}

#[test]
fn only_a_current_managed_analyzer_in_the_same_region_can_own_the_rule() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/access-analyzer-plan.json")).unwrap();
    let owner = |input: &Value| {
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_accessanalyzer_archive_rule")
            .unwrap()
            .member_owner
            .is_some()
    };
    assert!(owner(&fixture));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["region"] = json!("eu-west-1");
    changed["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
        json!({"constant_value":"eu-west-1"});
    assert!(!owner(&changed));
    let mut literal = fixture.clone();
    literal["configuration"]["root_module"]["resources"][0]["expressions"]["analyzer_name"] =
        json!({"constant_value": "accessanalyzer_analyzer-fixture"});
    assert!(!owner(&literal));
    let mut lookup = fixture;
    lookup["resource_changes"][1]["mode"] = json!("data");
    assert!(!owner(&lookup));
}
