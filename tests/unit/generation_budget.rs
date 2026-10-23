use super::validate;
use crate::Error;

#[test]
fn structural_limits_are_checked_without_building_a_json_tree() {
    let array = |count: usize| format!("[{}]", vec!["null"; count].join(","));
    assert!(validate(&array(32_767)).is_ok());
    assert_eq!(validate(&array(32_768)), Err(Error::ResourceLimit));
    assert!(validate(&format!("{}null{}", "[".repeat(63), "]".repeat(63))).is_ok());
    assert_eq!(
        validate(&format!("{}null{}", "[".repeat(64), "]".repeat(64))),
        Err(Error::ResourceLimit)
    );
    for input in ["[", "null null", "{\"a\":}"] {
        assert_eq!(validate(input), Err(Error::InvalidJson));
    }
}
