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

#[test]
fn linked_regions_keep_their_mode_and_home_region() {
    let v = json!({"aggregation_region":"us-east-1","region_linking_mode":"ALL_REGIONS_EXCEPT_SPECIFIED","linked_regions":["us-west-2"]});
    let s = selections("aws_securityhub_aggregator_v2", &v, &Value::Null);
    assert!(s.iter().any(|row| row.2 == "us-east-1"));
    assert!(s.iter().any(|row| row.2 == "all except us-west-2"));
    assert_eq!(
        selections(
            "aws_securityhub_aggregator_v2",
            &v,
            &json!({"linked_regions":true})
        )
        .len(),
        1
    );
}

#[test]
fn lake_regions_and_catalog_log_sources_remain_separate() {
    let v = json!({"configuration":[{"region":"us-east-1","replication_configuration":[{"regions":["us-west-2"]}]},{"region":"eu-west-1"}]});
    let s = selections("aws_securitylake_data_lake", &v, &Value::Null);
    assert!(s.iter().any(|row| row.2 == "us-east-1 to us-west-2"));
    assert!(s.iter().any(|row| row.2 == "eu-west-1"));
    let src = json!({"source":[{"source_name":"ROUTE53","regions":["us-east-1"]}]});
    assert_eq!(
        selections("aws_securitylake_aws_log_source", &src, &Value::Null)[0].2,
        "ROUTE53 / all lake accounts / us-east-1"
    );
}
