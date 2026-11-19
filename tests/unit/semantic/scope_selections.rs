use super::selections;
use serde_json::{Value, json};

#[test]
fn share_destination_requires_both_known_selectors() {
    let v = json!({"destination_account":"123456789012","destination_region":"us-west-2"});
    let select = |m: &Value| selections("aws_auditmanager_framework_share", &v, m);
    assert_eq!(select(&Value::Null)[0].2, "123456789012 / us-west-2");
    for mask in [
        json!({"destination_account":true}),
        json!({"destination_region":true}),
    ] {
        assert!(select(&mask).is_empty());
    }
}
