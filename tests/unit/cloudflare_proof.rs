use super::*;
use crate::provider::cloudflare::evidence::scope_id;
use serde_json::json;

#[test]
fn scope_ids_remain_typed_exact_and_case_preserving() {
    for value in [
        "0123456789abcdef0123456789abcdef",
        "ABCDEF0123456789ABCDEF0123456789",
    ] {
        assert_eq!(scope_id(value), Some(value));
    }
    for value in [
        "",
        "account-name",
        "0123456789abcdef0123456789abcde",
        "gggggggggggggggggggggggggggggggg",
        "01234567-89ab-cdef-0123-456789abcdef",
    ] {
        assert_eq!(scope_id(value), None);
    }
}

#[test]
fn qualifiers_require_known_null_and_preserve_mask_diagnostics() {
    for (value, mask, unknown, sensitive, expected) in [
        (Value::Null, Value::Null, Value::Null, Value::Null, Ok(())),
        (Value::Null, json!(false), Value::Null, Value::Null, Ok(())),
        (
            json!("production"),
            Value::Null,
            Value::Null,
            Value::Null,
            Err(Failure::UnsupportedContract),
        ),
        (
            Value::Null,
            json!(true),
            json!(true),
            Value::Null,
            Err(Failure::UnknownValue),
        ),
        (
            Value::Null,
            json!(true),
            Value::Null,
            json!(true),
            Err(Failure::SensitiveValue),
        ),
        (
            Value::Null,
            json!({}),
            Value::Null,
            Value::Null,
            Err(Failure::InvalidValue),
        ),
    ] {
        assert_eq!(
            Evidence {
                value: &value,
                mask: &mask,
                unknown: &unknown,
                sensitive: &sensitive
            }
            .require_null(),
            expected
        );
    }
}

#[test]
fn lifecycle_mode_and_known_provider_configuration_are_separate_requirements() {
    let plan =
        crate::plan::parse(include_str!("../fixtures/cloudflare-imported-plan.json")).unwrap();
    let original = &plan.nodes[0];
    assert!(current(original));
    assert!(known_configuration_matches(original, original));
    let mut a = original.clone();
    let mut b = original.clone();
    a.provider_configuration = None;
    b.provider_configuration = None;
    assert!(!known_configuration_matches(&a, &b));
    assert!(!known_configuration_matches(original, &a));
    let mut alias = original.clone();
    alias
        .provider_configuration
        .as_mut()
        .unwrap()
        .key
        .push_str(".alias");
    assert!(!known_configuration_matches(original, &alias));
    let mut unsupported = original.clone();
    unsupported.resource_type = "cloudflare_unknown".into();
    assert!(active(&unsupported));
    assert!(!current(&unsupported));
    let mut deposed = original.clone();
    deposed.deposed_key = Some("old".into());
    assert!(!active(&deposed));
    let mut deleted = original.clone();
    deleted.action = Action::Delete;
    assert!(!active(&deleted));
    assert!(unique(&["a", "b"]).is_ok());
    assert_eq!(unique(&["a", "a"]), Err(Failure::IncompleteCollection));
    assert!(unique::<&str>(&[]).is_ok());
}
