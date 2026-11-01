use super::*;
use serde_json::json;

fn input(changes: serde_json::Value) -> String {
    json!({"format_version":"1.2", "resource_changes":changes}).to_string()
}

fn named(state: &str, json: &str) -> NamedInput {
    NamedInput {
        state: state.into(),
        json: json.into(),
    }
}

#[test]
fn counts_objects_with_exclusive_actions_without_generation() {
    let mut changes = Vec::new();
    for (i, actions) in [
        json!(["no-op"]),
        json!(["create"]),
        json!(["update"]),
        json!(["delete"]),
        json!(["create", "delete"]),
        json!(["delete", "create"]),
        json!(["read"]),
        json!([]),
    ]
    .into_iter()
    .enumerate()
    {
        changes.push(
            json!({"address":format!("aws_instance.item[{i}]"), "type":"aws_instance",
            "mode":if i == 6 {"data"} else {"managed"}, "change":{"actions":actions,
            "after":{"secret":"PRIVATE_ATTRIBUTE"}}}),
        );
    }
    changes.push(
        json!({"address":"aws_instance.item[0]", "type":"aws_instance",
        "deposed":"PRIVATE_DEPOSED", "change":{"actions":["delete"]}}),
    );
    let mut plan: serde_json::Value = serde_json::from_str(&input(json!(changes))).unwrap();
    plan["planned_values"] = json!({"root_module":{"resources":[
        {"address":"terraform_data.unchanged", "type":"terraform_data"}
    ]}});
    let stages = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = stages.clone();
    let summary = crate::generation::progress::observe(
        std::rc::Rc::new(move |stage| observed.borrow_mut().push(stage.to_owned())),
        || inspect_input_summary(&plan.to_string()).unwrap(),
    );
    assert!(stages.borrow().is_empty());
    assert_eq!(
        summary.counts,
        ObjectCounts {
            resources: 9,
            objects: 10,
            current: 9,
            deposed: 1,
            managed: 9,
            data: 1,
            actions: ActionCounts {
                unchanged: 2,
                create: 1,
                update: 1,
                delete: 2,
                replacement: 2,
                read: 1,
                other: 1
            }
        }
    );
    let encoded = serde_json::to_string(&summary).unwrap();
    assert!(!encoded.contains("PRIVATE"));
    assert!(!encoded.contains("aws_instance"));
}

#[test]
fn reports_declared_inferred_unknown_and_unavailable_sources() {
    let plan = json!({"format_version":"1.2", "planned_values":{"root_module":{"resources":[
        {"address":"aws_instance.explicit", "type":"aws_instance", "provider_name":"hashicorp/aws"}
    ]}}, "resource_changes":[
        {"address":"aws_instance.explicit", "type":"aws_instance", "provider_name":"Example/Custom"},
        {"address":"aws_instance.inferred", "type":"aws_instance"},
        {"address":"terraform_data.builtin", "type":"terraform_data"},
        {"address":"custom_item.unknown", "type":"custom_item"},
        {"address":"custom_item.invalid", "type":"custom_item", "provider_name":"PRIVATE invalid source"}
    ]}).to_string();
    let result = inspect_input_summary(&plan).unwrap();
    assert_eq!(result.providers.len(), 5);
    assert!(result.providers.iter().any(|p| p.source.as_deref()
        == Some("registry.terraform.io/example/custom")
        && p.evidence == ProviderEvidence::Explicit));
    assert!(result.providers.iter().any(|p| p.source.as_deref()
        == Some("registry.terraform.io/hashicorp/aws")
        && p.evidence == ProviderEvidence::Inferred));
    assert!(result.providers.iter().any(|p| p.source.as_deref()
        == Some("terraform.io/builtin/terraform")
        && p.evidence == ProviderEvidence::Inferred));
    for evidence in [ProviderEvidence::Unknown, ProviderEvidence::Explicit] {
        assert!(
            result
                .providers
                .iter()
                .any(|p| p.evidence == evidence && p.source_unavailable && p.source.is_none())
        );
    }
    assert!(!serde_json::to_string(&result).unwrap().contains("PRIVATE"));
}

#[test]
fn named_aggregate_preserves_state_identity_and_untruncated_provider_totals() {
    let changes: Vec<_> = (0..70)
        .map(|i| {
            json!({"address":format!("custom_item.item[{i}]"),
        "type":"custom_item", "provider_name":format!("example/provider{i:02}")})
        })
        .collect();
    let plan = input(json!(changes));
    let result = inspect_named_input_summaries(&[named("b", &plan), named("a", &plan)]).unwrap();
    assert_eq!(
        result
            .inputs
            .iter()
            .map(|r| r.state.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert_eq!(result.aggregate.counts.resources, 140);
    assert_eq!(result.aggregate.counts.objects, 140);
    assert_eq!(result.aggregate.providers.len(), 64);
    assert_eq!(result.aggregate.omitted_provider_objects, 12);
    assert_eq!(
        result
            .aggregate
            .providers
            .iter()
            .map(|p| p.objects)
            .sum::<usize>()
            + result.aggregate.omitted_provider_objects,
        140
    );
    assert_eq!(result.inputs[0].summary.omitted_provider_objects, 6);
    assert_eq!(result.aggregate.counts.actions.other, 140);
}

#[test]
fn empty_invalid_and_named_row_errors_are_safe() {
    let empty = input(json!([]));
    assert_eq!(
        inspect_input_summary(&empty).unwrap().counts,
        ObjectCounts::default()
    );
    for (plan, code) in [
        ("PRIVATE{", Error::InvalidJson),
        ("{}", Error::UnsupportedInput),
        (
            r#"{"format_version":"2.0","resource_changes":[]}"#,
            Error::UnsupportedInput,
        ),
    ] {
        assert_eq!(inspect_input_summary(plan).unwrap_err(), code);
        let error =
            inspect_named_input_summaries(&[named("ok", &empty), named("bad", plan)]).unwrap_err();
        assert_eq!(
            error,
            InspectionError {
                code,
                input_index: Some(1)
            }
        );
        assert!(!error.to_string().contains("PRIVATE"));
    }
    for (inputs, code, index) in [
        (vec![], Error::EmptyInputs, None),
        (
            (0..65).map(|i| named(&format!("s{i}"), &empty)).collect(),
            Error::EmptyInputs,
            None,
        ),
        (
            vec![named("ok", &empty), named("bad/name", &empty)],
            Error::InvalidStateName,
            Some(1),
        ),
        (
            vec![named("same", &empty), named("same", &empty)],
            Error::DuplicateStateName,
            Some(1),
        ),
    ] {
        assert_eq!(
            inspect_named_input_summaries(&inputs).unwrap_err(),
            InspectionError {
                code,
                input_index: index
            }
        );
    }
}

#[test]
fn bounds_input_structure_objects_and_named_aggregate() {
    let too_large = " ".repeat(crate::generation::MAX_INPUT_BYTES + 1);
    assert_eq!(
        inspect_input_summary(&too_large).unwrap_err(),
        Error::InputTooLarge
    );
    assert_eq!(
        inspect_named_input_summaries(&[named("large", &too_large)])
            .unwrap_err()
            .code,
        Error::InputTooLarge
    );
    let many_values =
        json!({"format_version":"1.2", "resource_changes":[], "extra":vec![0; 32_768]}).to_string();
    assert_eq!(
        inspect_input_summary(&many_values).unwrap_err(),
        Error::ResourceLimit
    );
    let deep = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    assert_eq!(
        inspect_input_summary(&deep).unwrap_err(),
        Error::ResourceLimit
    );
    let objects = |count| {
        input(json!(
            (0..count)
                .map(|i| json!({"address":format!("custom_item.a[{i}]")}))
                .collect::<Vec<_>>()
        ))
    };
    assert_eq!(
        inspect_input_summary(&objects(4097)).unwrap_err(),
        Error::ResourceLimit
    );
    let plan = objects(2100);
    assert_eq!(inspect_input_summary(&plan).unwrap().counts.objects, 2100);
    assert_eq!(
        inspect_named_input_summaries(&[named("a", &plan), named("b", &plan)]).unwrap_err(),
        InspectionError {
            code: Error::ResourceLimit,
            input_index: Some(1)
        }
    );
    let values =
        json!({"format_version":"1.2", "resource_changes":[], "extra":vec![0; 17_000]}).to_string();
    assert_eq!(
        inspect_named_input_summaries(&[named("a", &values), named("b", &values)]).unwrap_err(),
        InspectionError {
            code: Error::ResourceLimit,
            input_index: Some(1)
        }
    );
}
