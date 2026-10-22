use super::*;
use serde_json::json;

#[test]
fn inspection_preserves_original_counts_and_literal_selectors() {
    let selector = format!("module.app[{}]", json!(" a,b:c.d\"e\\f "));
    let child = format!("{selector}.module.child[0]");
    let resource = format!("{selector}.terraform_data.item");
    let input = json!({
        "format_version": "1.2",
        "resource_changes": [
            {"address": "terraform_data.root", "type": "terraform_data"},
            {"address": resource, "type": "terraform_data"},
            {"address": resource, "type": "terraform_data", "deposed": "old"},
            {"address": format!("{selector}.data.aws_region.current"), "mode": "data", "type": "aws_region"},
            {"address": format!("{child}.terraform_data.item"), "type": "terraform_data"}
        ],
        "planned_values": {"root_module": {"child_modules": [{"address": "module.empty[0]"}]}},
        "configuration": {"root_module": {"module_calls": {"app": {"module": {}}, "unknown": {"module": {}}}}}
    });
    let rows = inspect_modules(&input.to_string()).unwrap();
    let row = |selector: Option<&str>| {
        rows.iter()
            .find(|row| row.selector.as_deref() == selector)
            .unwrap()
    };
    assert_eq!((row(None).direct, row(None).subtree), (1, 5));
    let actual = row(Some(&selector));
    assert!(actual.instance);
    assert_eq!((actual.direct, actual.subtree), (3, 4));
    let aggregate = row(Some("module.app"));
    assert!(!aggregate.instance);
    assert_eq!((aggregate.direct, aggregate.subtree), (3, 4));
    assert_eq!(row(Some(&child)).direct, 1);
    assert!(row(Some("module.empty[0]")).instance);
    assert_eq!(row(Some("module.empty[0]")).subtree, 0);
    assert!(!row(Some("module.unknown")).instance);
    assert_eq!(row(Some("module.unknown")).subtree, 0);
}

#[test]
fn inspection_uses_bounded_safe_plan_input_contract() {
    for (input, expected) in [
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
        assert_eq!(inspect_modules(&input).unwrap_err(), expected);
    }
}
