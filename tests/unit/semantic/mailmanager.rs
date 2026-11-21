use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn mailmanager_rule_verifies_s3_bucket_id() {
    let provider = "registry.terraform.io/hashicorp/aws";
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-catalog-plan.json")).unwrap();
    input["resource_changes"] = json!([
        {
            "address":"aws_s3_bucket.bucket","mode":"managed","type":"aws_s3_bucket",
            "name":"bucket","provider_name":provider,
            "change":{"actions":["create"],"after":{"id":"mail-bucket"}}
        },
        {
            "address":"aws_mailmanager_rule_set.rules","mode":"managed","type":"aws_mailmanager_rule_set",
            "name":"rules","provider_name":provider,
            "change":{"actions":["create"],"after":{"rule":[{"action":[{"write_to_s3":[{"s3_bucket":"mail-bucket"}]}]}]}}
        }
    ]);
    input["configuration"]["root_module"]["resources"] = json!([
        {"address":"aws_s3_bucket.bucket","provider_config_key":"aws","expressions":{}},
        {"address":"aws_mailmanager_rule_set.rules","provider_config_key":"aws",
         "expressions":{"rule":[{"action":[{"write_to_s3":[{"s3_bucket":{
            "references":["aws_s3_bucket.bucket.id"]
         }}]}]}]}}
    ]);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let node = graph
        .nodes
        .iter()
        .find(|node| node.resource_type == "aws_mailmanager_rule_set")
        .unwrap();
    assert!(node.references.iter().any(|reference| {
        reference.label == "S3 bucket"
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
    }));
}

#[test]
fn mailmanager_rule_verifies_nested_relay_action() {
    let provider = "registry.terraform.io/hashicorp/aws";
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-catalog-plan.json")).unwrap();
    input["resource_changes"] = json!([
        {
            "address":"aws_mailmanager_relay.relay","mode":"managed","type":"aws_mailmanager_relay",
            "name":"relay","provider_name":provider,
            "change":{"actions":["create"],"after":{"id":"relay-example"}}
        },
        {
            "address":"aws_mailmanager_rule_set.rules","mode":"managed","type":"aws_mailmanager_rule_set",
            "name":"rules","provider_name":provider,
            "change":{"actions":["create"],"after":{"rule":[{"action":[{"relay":[{"relay":"relay-example"}]}]}]}}
        }
    ]);
    input["configuration"]["root_module"]["resources"] = json!([
        {"address":"aws_mailmanager_relay.relay","provider_config_key":"aws","expressions":{}},
        {"address":"aws_mailmanager_rule_set.rules","provider_config_key":"aws",
         "expressions":{"rule":[{"action":[{"relay":[{"relay":{
            "references":["aws_mailmanager_relay.relay.id"]
         }}]}]}]}}
    ]);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let node = graph
        .nodes
        .iter()
        .find(|node| node.resource_type == "aws_mailmanager_rule_set")
        .unwrap();
    assert!(node.references.iter().any(|reference| {
        reference.label == "Mail relay"
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
    }));
}

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
