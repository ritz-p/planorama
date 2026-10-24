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
