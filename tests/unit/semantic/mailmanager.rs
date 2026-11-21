use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn ingress_rule_set_requires_a_known_managed_endpoint() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-catalog-plan.json")).unwrap();
    input["resource_changes"] = json!([
        {
            "address":"aws_mailmanager_rule_set.rules","mode":"managed",
            "type":"aws_mailmanager_rule_set","name":"rules",
            "provider_name":"registry.terraform.io/hashicorp/aws",
            "change":{"actions":["create"],"after":{"id":"rs-example"}}
        },
        {
            "address":"aws_mailmanager_ingress_point.ingress","mode":"managed",
            "type":"aws_mailmanager_ingress_point","name":"ingress",
            "provider_name":"registry.terraform.io/hashicorp/aws",
            "change":{"actions":["create"],"after":{"rule_set_id":"rs-example"}}
        }
    ]);
    input["configuration"]["root_module"]["resources"] = json!([
        {"address":"aws_mailmanager_rule_set.rules","provider_config_key":"aws","expressions":{}},
        {"address":"aws_mailmanager_ingress_point.ingress","provider_config_key":"aws",
         "expressions":{"rule_set_id":{"references":["aws_mailmanager_rule_set.rules.id"]}}}
    ]);

    let verified = |input: &Value| {
        semantic::transform(&plan::parse(&input.to_string()).unwrap())
            .nodes
            .iter()
            .find(|node| node.resource_type == "aws_mailmanager_ingress_point")
            .unwrap()
            .references
            .iter()
            .any(|reference| {
                reference.label == "Mail rule set"
                    && reference.complete
                    && !reference.dependency
                    && !reference.targets.is_empty()
            })
    };

    assert!(verified(&input));
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"rule_set_id":true});
    assert!(!verified(&input));
}

#[test]
fn pinpoint_event_stream_accepts_only_the_documented_stream_types() {
    let rule = super::rule("aws_pinpoint_event_stream", "destination_stream_arn").unwrap();
    assert_eq!(
        rule.outputs,
        &[
            ("aws_kinesis_stream", &["arn"][..]),
            ("aws_kinesis_firehose_delivery_stream", &["arn"][..]),
        ]
    );
    assert!(super::rule("aws_pinpoint_email_template", "application_id").is_none());
}

#[test]
fn sms_pool_and_keyword_use_distinct_origination_types() {
    let pool = super::rule("aws_pinpointsmsvoicev2_pool", "origination_identities").unwrap();
    let keyword =
        super::rule("aws_pinpointsmsvoicev2_keyword", "origination_identity_arn").unwrap();
    assert_eq!(
        pool.outputs,
        &[
            ("aws_pinpointsmsvoicev2_phone_number", &["arn"][..]),
            ("aws_pinpointsmsvoicev2_sender_id", &["arn"][..]),
        ]
    );
    assert_eq!(
        keyword.outputs,
        &[
            ("aws_pinpointsmsvoicev2_phone_number", &["arn"][..]),
            ("aws_pinpointsmsvoicev2_pool", &["arn"][..]),
        ]
    );
}

fn reference_plan(
    source_kind: &str,
    source_values: Value,
    target_kind: &str,
    target_values: Value,
    expressions: Value,
) -> Value {
    let provider = "registry.terraform.io/hashicorp/aws";
    json!({
        "format_version":"1.2",
        "resource_changes":[
            {
                "address":format!("{source_kind}.source"),"mode":"managed",
                "type":source_kind,"name":"source","provider_name":provider,
                "change":{"actions":["create"],"after":source_values}
            },
            {
                "address":format!("{target_kind}.target"),"mode":"managed",
                "type":target_kind,"name":"target","provider_name":provider,
                "change":{"actions":["create"],"after":target_values}
            }
        ],
        "configuration":{
            "provider_config":{
                "aws":{"name":"aws","full_name":provider,"expressions":{
                    "region":{"constant_value":"us-east-1"},
                    "allowed_account_ids":{"constant_value":["123456789012"]}
                }}
            },
            "root_module":{"resources":[
                {"address":format!("{source_kind}.source"),"provider_config_key":"aws","expressions":{}},
                {"address":format!("{target_kind}.target"),"provider_config_key":"aws","expressions":expressions}
            ]}
        }
    })
}

fn verified(input: &Value, target_kind: &str, label: &str) -> bool {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
        .nodes
        .iter()
        .find(|node| node.resource_type == target_kind)
        .unwrap()
        .references
        .iter()
        .any(|reference| {
            reference.label == label
                && reference.complete
                && !reference.dependency
                && !reference.targets.is_empty()
        })
}

#[test]
fn sms_pool_verifies_root_collection_members() {
    let kind = "aws_pinpointsmsvoicev2_pool";
    let arn = "arn:aws:sms-voice:us-east-1:123456789012:phone-number/phone-example";
    let mut input = reference_plan(
        "aws_pinpointsmsvoicev2_phone_number",
        json!({"arn":arn}),
        kind,
        json!({"origination_identities":[arn]}),
        json!({"origination_identities":{"references":["aws_pinpointsmsvoicev2_phone_number.source.arn"]}}),
    );
    assert!(verified(&input, kind, "Pool origination identities"));
    input["resource_changes"][1]["change"]["after_unknown"] =
        json!({"origination_identities":[true]});
    assert!(!verified(&input, kind, "Pool origination identities"));
}

#[test]
fn ses_receipt_rule_validates_reference_scope() {
    let kind = "aws_ses_receipt_rule";
    let value = "receipt-example";
    let mut input = reference_plan(
        "aws_ses_receipt_rule_set",
        json!({"rule_set_name":value}),
        kind,
        json!({"rule_set_name":value}),
        json!({"rule_set_name":{"references":["aws_ses_receipt_rule_set.source.rule_set_name"]}}),
    );
    assert!(verified(&input, kind, "Receipt rule set"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Receipt rule set"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"rule_set_name":true});
    assert!(!verified(&input, kind, "Receipt rule set"));
}
