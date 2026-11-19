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
fn aggregator_preserves_each_account_and_region_tuple() {
    let v = json!({"account_aggregation_source":[{"account_ids":["123456789012"],"regions":["us-east-1"]},{"account_ids":["210987654321"],"all_regions":true}]});
    let s = selections("aws_config_configuration_aggregator", &v, &Value::Null);
    assert_eq!(
        s.iter().map(|row| row.2.as_str()).collect::<Vec<_>>(),
        ["123456789012 / us-east-1", "210987654321 / all regions"]
    );
    let m = json!({"account_aggregation_source":[{},{"account_ids":true}]});
    assert_eq!(
        selections("aws_config_configuration_aggregator", &v, &m).len(),
        1
    );
}
