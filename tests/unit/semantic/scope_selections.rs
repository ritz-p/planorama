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

#[test]
fn delegation_control_set_is_scoped_to_a_known_assessment() {
    let v = json!({"assessment_id":"assessment-a","control_set_id":"payments"});
    let select = |mask: &Value| selections("aws_auditmanager_assessment_delegation", &v, mask);
    assert_eq!(select(&Value::Null)[0].2, "assessment-a / payments");
    for mask in [json!({"assessment_id":true}),json!({"control_set_id":true})] {
        assert!(select(&mask).is_empty());
    }
}
