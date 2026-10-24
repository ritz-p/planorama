use super::*;
use serde_json::json;

#[test]
fn resources_preserve_identity_and_group_historical_objects_without_layout() {
    let module = format!("module.app[{}].module.child[0]", json!(" a,b:c.d\"e\\f "));
    let address = format!("{module}.terraform_data.item[{}]", json!(" x,\"\\ "));
    let input = json!({"format_version":"1.2", "resource_changes": [
        {"address":address,"type":"terraform_data","change":{"after":{"secret":"NEVER_EXPOSE"}}},
        {"address":address,"type":"terraform_data","deposed":"old"},
        {"address":format!("{module}.data.aws_region.current"),"type":"aws_region","mode":"data"}
    ]})
    .to_string();
    let stages = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = stages.clone();
    let result = crate::generation::progress::observe(
        std::rc::Rc::new(move |stage| observed.borrow_mut().push(stage.to_owned())),
        || inspect_input(&input).unwrap(),
    );
    assert!(stages.borrow().is_empty());
    let rows = result.resources;
    assert_eq!(rows.len(), 2);
    let row = rows.iter().find(|row| row.address == address).unwrap();
    assert_eq!(row.selector, address);
    assert_eq!(row.module, module);
    assert_eq!(row.state, None);
    assert_eq!((row.current, row.deposed), (1, 1));
    assert_eq!(row.mode, "managed");
    assert_eq!(
        rows.iter().find(|row| row.mode == "data").unwrap().current,
        1
    );
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("NEVER_EXPOSE")
    );
    assert_eq!(
        crate::generate(
            &input,
            &crate::Options {
                focus: Some(address),
                ..Default::default()
            }
        )
        .err(),
        Some(crate::Error::SelectionFailed)
    );
}

#[test]
fn resource_inspection_handles_empty_and_invalid_input() {
    assert!(
        inspect_resources(r#"{"format_version":"1.2","resource_changes":[]}"#)
            .unwrap()
            .is_empty()
    );
    for (input, error) in [
        ("".to_owned(), crate::Error::InvalidJson),
        ("secret{".to_owned(), crate::Error::InvalidJson),
        (
            r#"{"format_version":"1.0","values":{}}"#.to_owned(),
            crate::Error::UnsupportedInput,
        ),
        (
            " ".repeat(crate::generation::MAX_INPUT_BYTES + 1),
            crate::Error::InputTooLarge,
        ),
    ] {
        assert_eq!(inspect_resources(&input).unwrap_err(), error);
        assert_eq!(inspect_input(&input).unwrap_err(), error);
    }
}
