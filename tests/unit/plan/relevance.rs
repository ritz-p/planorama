use serde_json::{Value, json};

fn input() -> Value {
    let mut plan: Value =
        serde_json::from_str(include_str!("../../fixtures/drift-plan.json")).unwrap();
    plan["relevant_attributes"] = json!([
        {"resource":"terraform_data.planned","attribute":["network",0,"tags","key"]},
        {"resource":"terraform_data.planned","attribute":["id"]},
        {"resource":"terraform_data.missing","attribute":["TOP_SECRET"]}
    ]);
    plan
}

#[test]
fn relevance_is_exact_deterministic_and_separate_from_drift_actions() {
    let mut input = input();
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    assert_eq!(raw.relevant_attributes.len(), 3);
    assert!(
        raw.drift
            .iter()
            .find(|d| d.address == "terraform_data.planned")
            .unwrap()
            .relevant
    );
    assert!(
        !raw.drift
            .iter()
            .find(|d| d.address == "terraform_data.drift_only")
            .unwrap()
            .relevant
    );
    let node = raw
        .nodes
        .iter()
        .find(|n| n.address == "terraform_data.planned")
        .unwrap();
    assert_eq!(node.action, crate::model::Action::Create);
    assert!(node.metadata.drift.contains(&crate::model::Action::Delete));
    assert_eq!(node.metadata.relevant_attributes.as_ref().unwrap().len(), 2);
    assert_eq!(
        node.metadata.relevant_attributes.as_ref().unwrap()[1][1],
        crate::model::AttributePathStep::Index(0)
    );
    assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    input["relevant_attributes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(crate::plan::parse(&input.to_string()).unwrap(), raw);
    input.as_object_mut().unwrap().remove("relevant_attributes");
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    assert!(raw.relevant_attributes.is_empty());
    assert!(raw.drift.iter().all(|d| !d.relevant));
}

#[test]
fn sensitivity_from_changes_drift_and_values_redacts_paths() {
    for source in [
        "resource_changes",
        "resource_drift",
        "planned_values",
        "prior_state",
    ] {
        let mut input = input();
        input["relevant_attributes"] =
            json!([{"resource":"terraform_data.planned","attribute":["tokens","TOP_SECRET"]}]);
        match source {
            "resource_changes" | "resource_drift" => {
                input[source][0]["change"]["before_sensitive"] = json!({"tokens":true})
            }
            "planned_values" => {
                input[source] = json!({"root_module":{"resources":[{"address":"terraform_data.planned","type":"terraform_data","sensitive_values":{"tokens":true}}]}})
            }
            _ => {
                input[source] = json!({"values":{"root_module":{"resources":[{"address":"terraform_data.planned","sensitive_values":{"tokens":true}}]}}})
            }
        }
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert!(raw.relevant_attributes[0].matched);
        assert!(raw.relevant_attributes[0].path.is_none());
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    }
}

#[test]
fn malformed_paths_and_ambiguous_instances_do_not_invent_relevance() {
    let mut input = input();
    let mut deposed = input["resource_changes"][0].clone();
    deposed["deposed"] = "old".into();
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    input["relevant_attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"resource":"terraform_data.drift_only","attribute":[true]}));
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    assert!(raw.relevant_attributes.iter().all(|r| !r.matched));
    assert!(raw.drift.iter().all(|d| !d.relevant));
}
