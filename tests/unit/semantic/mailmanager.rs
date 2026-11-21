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

#[test]
fn sesv2_tenant_resource_association_validates_reference_scope() {
    let kind = "aws_sesv2_tenant_resource_association";
    let value = "arn:aws:ses:us-east-1:123456789012:template/example";
    let mut input = reference_plan(
        "aws_ses_template",
        json!({"arn":value}),
        kind,
        json!({"resource_arn":value}),
        json!({"resource_arn":{"references":["aws_ses_template.source.arn"]}}),
    );
    assert!(verified(&input, kind, "Tenant SES resource"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Tenant SES resource"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"resource_arn":true});
    assert!(!verified(&input, kind, "Tenant SES resource"));
}

#[test]
fn workmail_user_validates_reference_scope() {
    let kind = "aws_workmail_user";
    let value = "m-0123456789abcdef0123456789abcdef";
    let mut input = reference_plan(
        "aws_workmail_organization",
        json!({"organization_id":value}),
        kind,
        json!({"organization_id":value}),
        json!({"organization_id":{"references":["aws_workmail_organization.source.organization_id"]}}),
    );
    assert!(verified(&input, kind, "WorkMail organization"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "WorkMail organization"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"organization_id":true});
    assert!(!verified(&input, kind, "WorkMail organization"));
}

#[test]
fn workmail_default_domain_must_belong_to_the_configured_organization() {
    let kind = "aws_workmail_default_domain";
    let org = "m-0123456789abcdef0123456789abcdef";
    let mut input = reference_plan(
        "aws_workmail_domain",
        json!({"domain_name":"example.test","organization_id":org}),
        kind,
        json!({"domain_name":"example.test","organization_id":org}),
        json!({
            "domain_name":{"references":["aws_workmail_domain.source.domain_name"]},
            "organization_id":{"references":["aws_workmail_organization.org.organization_id"]}
        }),
    );
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address":"aws_workmail_organization.org","mode":"managed",
            "type":"aws_workmail_organization","name":"org",
            "provider_name":"registry.terraform.io/hashicorp/aws",
            "change":{"actions":["create"],"after":{"organization_id":org}}
        }));
    let resources = input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    resources[0]["expressions"]["organization_id"] =
        json!({"references":["aws_workmail_organization.org.organization_id"]});
    resources.push(json!({
        "address":"aws_workmail_organization.org","provider_config_key":"aws","expressions":{}
    }));
    assert!(verified(&input, kind, "WorkMail mail domain"));
    input["resource_changes"][0]["change"]["after"]["organization_id"] = json!("m-other");
    assert!(!verified(&input, kind, "WorkMail mail domain"));
}

#[test]
fn elastictranscoder_pipeline_validates_reference_scope() {
    let kind = "aws_elastictranscoder_pipeline";
    let value = "arn:aws:kms:us-east-1:123456789012:key/11111111-1111-1111-1111-111111111111";
    let mut input = reference_plan(
        "aws_kms_key",
        json!({"arn":value}),
        kind,
        json!({"aws_kms_key_arn":value}),
        json!({"aws_kms_key_arn":{"references":["aws_kms_key.source.arn"]}}),
    );
    assert!(verified(&input, kind, "Transcoder encryption key"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Transcoder encryption key"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"aws_kms_key_arn":true});
    assert!(!verified(&input, kind, "Transcoder encryption key"));
}

#[test]
fn gamelift_game_session_queue_validates_reference_scope() {
    let kind = "aws_gamelift_game_session_queue";
    let value = "arn:aws:gamelift:us-east-1:123456789012:alias/alias-example";
    let mut input = reference_plan(
        "aws_gamelift_alias",
        json!({"arn":value}),
        kind,
        json!({"destinations":[value]}),
        json!({"destinations":{"references":["aws_gamelift_alias.source.arn"]}}),
    );
    assert!(verified(&input, kind, "Session placement destinations"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Session placement destinations"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"destinations":[true]});
    assert!(!verified(&input, kind, "Session placement destinations"));
}

#[test]
fn gamelift_subnet_group_requires_common_vpc_evidence() {
    let kind = "aws_gamelift_game_server_group";
    let mut input = reference_plan(
        "aws_subnet",
        json!({"id":"subnet-a","vpc_id":"vpc-a"}),
        kind,
        json!({"vpc_subnets":["subnet-a","subnet-b"]}),
        json!({"vpc_subnets":{"references":["aws_subnet.source.id","aws_subnet.second.id"]}}),
    );
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address":"aws_subnet.second","mode":"managed","type":"aws_subnet","name":"second",
            "provider_name":"registry.terraform.io/hashicorp/aws",
            "change":{"actions":["create"],"after":{"id":"subnet-b","vpc_id":"vpc-a"}}
        }));
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address":"aws_subnet.second","provider_config_key":"aws","expressions":{}
        }));
    assert!(verified(&input, kind, "FleetIQ subnets"));
    input["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-b");
    assert!(!verified(&input, kind, "FleetIQ subnets"));
}

#[test]
fn ivschat_room_validates_reference_scope() {
    let kind = "aws_ivschat_room";
    let value = "arn:aws:ivschat:us-east-1:123456789012:logging-configuration/example";
    let mut input = reference_plan(
        "aws_ivschat_logging_configuration",
        json!({"arn":value}),
        kind,
        json!({"logging_configuration_identifiers":[value]}),
        json!({"logging_configuration_identifiers":{"references":["aws_ivschat_logging_configuration.source.arn"]}}),
    );
    assert!(verified(&input, kind, "Chat logging configurations"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Chat logging configurations"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] =
        json!({"logging_configuration_identifiers":[true]});
    assert!(!verified(&input, kind, "Chat logging configurations"));
}

#[test]
fn location_tracker_association_validates_reference_scope() {
    let kind = "aws_location_tracker_association";
    let value = "arn:aws:geo:us-east-1:123456789012:geofence-collection/example";
    let mut input = reference_plan(
        "aws_location_geofence_collection",
        json!({"collection_arn":value}),
        kind,
        json!({"consumer_arn":value}),
        json!({"consumer_arn":{"references":["aws_location_geofence_collection.source.collection_arn"]}}),
    );
    assert!(verified(&input, kind, "Geofence collection"));

    let provider = &mut input["configuration"]["provider_config"];
    provider["aws.other"] = provider["aws"].clone();
    provider["aws.other"]["alias"] = json!("other");
    provider["aws.other"]["expressions"]["region"]["constant_value"] = json!("us-west-2");
    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
        json!("aws.other");
    assert!(!verified(&input, kind, "Geofence collection"));

    input["configuration"]["root_module"]["resources"][0]["provider_config_key"] = json!("aws");
    input["resource_changes"][1]["change"]["after_unknown"] = json!({"consumer_arn":true});
    assert!(!verified(&input, kind, "Geofence collection"));
}
