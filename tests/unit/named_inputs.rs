use super::*;

fn inputs() -> Vec<NamedInput> {
    ["network", "application"]
        .into_iter()
        .map(|state| NamedInput {
            state: state.into(),
            json: include_str!("../fixtures/browser-named.json").into(),
        })
        .collect()
}

#[test]
fn named_inventory_preserves_states_modes_history_and_module_counts() {
    let rows = inspect_named_inputs(&inputs()).unwrap();
    assert_eq!(rows.resources.len(), 6);
    for state in ["network", "application"] {
        let root = rows
            .modules
            .iter()
            .find(|row| row.state.as_deref() == Some(state) && row.selector.is_none())
            .unwrap();
        assert_eq!((root.direct, root.subtree), (1, 4));
        let module = rows
            .modules
            .iter()
            .find(|row| row.selector.as_deref() == Some(&format!("{state}:module.app")))
            .unwrap();
        assert_eq!((module.direct, module.subtree), (3, 3));
        let history = rows
            .resources
            .iter()
            .find(|row| row.selector == format!("{state}:module.app.terraform_data.history"))
            .unwrap();
        assert_eq!((history.current, history.deposed), (1, 1));
        assert!(
            rows.resources
                .iter()
                .any(|row| row.state.as_deref() == Some(state) && row.mode == "data")
        );
    }
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("NEVER_EXPOSE")
    );
}

#[test]
fn named_validation_rejects_names_and_aggregate_size_before_parsing() {
    assert_eq!(inspect_named_inputs(&[]).unwrap_err(), Error::EmptyInputs);
    for name in ["", "name:bad", " name", "日本語", &"a".repeat(65)] {
        let mut inputs = inputs();
        inputs[0].state = name.into();
        assert_eq!(
            inspect_named_inputs(&inputs).unwrap_err(),
            Error::InvalidStateName
        );
    }
    let mut inputs = inputs();
    inputs[0].state = inputs[1].state.clone();
    assert_eq!(
        inspect_named_inputs(&inputs).unwrap_err(),
        Error::DuplicateStateName
    );
    inputs[0].state = "other".into();
    inputs[0].json = " ".repeat(crate::generation::MAX_INPUT_BYTES / 2 + 1);
    inputs[1].json = inputs[0].json.clone();
    assert_eq!(
        inspect_named_inputs(&inputs).unwrap_err(),
        Error::InputTooLarge
    );
    inputs[0].json = "secret invalid JSON".into();
    inputs[1].json.clear();
    assert_eq!(
        inspect_named_inputs(&inputs).unwrap_err(),
        Error::InvalidJson
    );
}

#[test]
fn named_selection_uses_qualified_addresses_and_has_no_automatic_relationships() {
    let inputs = inputs();
    let generated = generate_named(&inputs, &Options::default()).unwrap();
    assert!(generated.content.contains("data-state-id=\"network\""));
    assert!(generated.content.contains("data-state-id=\"application\""));
    assert!(!generated.content.contains("cross-state-relationships"));
    assert!(!generated.content.contains("NEVER_EXPOSE"));
    let options = Options {
        modules: vec!["network:module.app".into()],
        ..Default::default()
    };
    assert!(generate_named(&inputs, &options).is_ok());
    let options = Options {
        focus: Some("network:module.app.terraform_data.item".into()),
        focus_depth: Some(0),
        ..Default::default()
    };
    assert!(generate_named(&inputs, &options).is_ok());
    let options = Options {
        focus: Some("network:module.app.terraform_data.history".into()),
        ..Default::default()
    };
    assert_eq!(
        generate_named(&inputs, &options).err(),
        Some(Error::SelectionFailed)
    );
}

#[test]
fn resumable_named_preserves_output_selection_and_stage_boundaries() {
    use std::{cell::RefCell, rc::Rc};
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        for selection in 0..3 {
            let options = || Options {
                format,
                html_diagnostics: matches!(format, OutputFormat::Html),
                html_semantic_diagnostics: matches!(format, OutputFormat::Html),
                modules: if selection == 1 {
                    vec!["network:module.app".into()]
                } else {
                    vec![]
                },
                focus: (selection == 2).then(|| "network:module.app.terraform_data.item".into()),
                ..Default::default()
            };
            let expected = generate_named(&inputs(), &options()).unwrap();
            for units in [1, 4096] {
                let mut job = crate::Generator::new_named(inputs(), options()).unwrap();
                let stages = Rc::new(RefCell::new(Vec::new()));
                let actual = loop {
                    let captured = stages.clone();
                    if let Some(result) = job
                        .step_with_progress(units, move |stage| {
                            captured.borrow_mut().push(stage);
                        })
                        .unwrap()
                    {
                        break result;
                    }
                };
                assert_eq!(actual.content, expected.content);
                assert_eq!(actual.diagnostics, expected.diagnostics);
                assert_eq!(job.stage(), "output");
                assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
                assert_eq!(Rc::strong_count(&stages), 1);
                let stages = stages.borrow();
                assert_eq!(stages.first(), Some(&"parsing"));
                assert_eq!(stages.last(), Some(&"output"));
                let selection = stages.iter().position(|s| *s == "selection").unwrap();
                let placement = stages.iter().position(|s| *s == "placement").unwrap();
                let routing = stages.iter().position(|s| *s == "routing").unwrap();
                assert!(selection < placement && placement < routing);
                assert!(stages.iter().all(|s| {
                    ["parsing", "selection", "placement", "routing", "output"].contains(s)
                }));
                assert_eq!(stages.iter().filter(|s| **s == "output").count(), 1);
            }
        }
    }
}

#[test]
fn named_jobs_cancel_fail_close_and_retry_independently() {
    for steps in [0, 1, 10] {
        let mut job = crate::Generator::new_named(inputs(), Options::default()).unwrap();
        for units in [0, 4097, usize::MAX] {
            assert_eq!(job.step(units).err(), Some(Error::InvalidOptions));
        }
        for _ in 0..steps {
            if job.step(1).unwrap().is_some() {
                break;
            }
        }
        job.cancel();
        job.cancel();
        assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
    }
    let mut invalid = inputs();
    invalid[1].json = "secret invalid".into();
    let mut job = crate::Generator::new_named(invalid, Options::default()).unwrap();
    assert_eq!(job.step(4096).err(), Some(Error::InvalidJson));
    assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
    let mut retry = crate::Generator::new_named(inputs(), Options::default()).unwrap();
    while retry.step(4096).unwrap().is_none() {}
}

#[test]
fn named_jobs_enforce_aggregate_bytes_structure_and_nodes() {
    let mut oversized = inputs();
    for input in &mut oversized {
        input.json = " ".repeat(crate::generation::MAX_BROWSER_INPUT_BYTES / 2 + 1);
    }
    assert!(matches!(
        crate::Generator::new_named(oversized, Options::default()),
        Err(Error::ResourceLimit)
    ));
    let structure =
        serde_json::json!({"format_version":"1.2", "resource_changes":[], "extra": vec![0; 17000]})
            .to_string();
    let resources: Vec<_> = (0..129).map(|i| serde_json::json!({
        "address": format!("terraform_data.item{i}"), "mode":"managed", "type":"terraform_data",
        "name":format!("item{i}"), "provider_name":"terraform.io/builtin/terraform",
        "change":{"actions":["create"], "before":null, "after":{"input":i}}
    })).collect();
    let nodes =
        serde_json::json!({"format_version":"1.2", "resource_changes":resources}).to_string();
    for json in [structure, nodes] {
        let named = || {
            ["a", "b"]
                .into_iter()
                .map(|state| NamedInput {
                    state: state.into(),
                    json: json.clone(),
                })
                .collect()
        };
        for units in [1, 4096] {
            let mut job = crate::Generator::new_named(named(), Options::default()).unwrap();
            loop {
                match job.step(units) {
                    Ok(None) => {}
                    Err(Error::ResourceLimit) => break,
                    _ => panic!("aggregate limit must reject the job"),
                }
            }
            assert_eq!(job.step(1).err(), Some(Error::GenerationClosed));
        }
    }
    assert!(matches!(
        crate::Generator::new_named(
            inputs(),
            Options {
                modules: vec!["x".into(); 257],
                ..Default::default()
            }
        ),
        Err(Error::ResourceLimit)
    ));
    assert!(matches!(
        crate::Generator::new_named(vec![], Options::default()),
        Err(Error::EmptyInputs)
    ));
    let mut duplicate = inputs();
    duplicate[1].state = duplicate[0].state.clone();
    assert!(matches!(
        crate::Generator::new_named(duplicate, Options::default()),
        Err(Error::DuplicateStateName)
    ));
}
