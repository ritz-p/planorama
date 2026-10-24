use super::*;
use serde_json::json;

#[test]
fn service_decisions_require_known_non_sensitive_evidence() {
    for (kind, values, unknown, sensitive, expected) in [
        (
            "aws_db_instance",
            json!({"storage_encrypted":true}),
            json!({}),
            json!({}),
            ENABLED,
        ),
        (
            "aws_rds_cluster",
            json!({"storage_encrypted":false}),
            json!({}),
            json!({}),
            DISABLED,
        ),
        (
            "aws_db_instance",
            json!({"kms_key_id":"external"}),
            json!({}),
            json!({}),
            UNKNOWN,
        ),
        (
            "aws_db_instance",
            json!({"storage_encrypted":false}),
            json!({"storage_encrypted":true}),
            json!({}),
            UNKNOWN,
        ),
        (
            "aws_db_instance",
            json!({"storage_encrypted":true}),
            json!({}),
            json!({"storage_encrypted":true}),
            UNKNOWN,
        ),
        (
            "aws_sqs_queue",
            json!({"sqs_managed_sse_enabled":true}),
            json!({}),
            json!({}),
            SQS,
        ),
        (
            "aws_sqs_queue",
            json!({"sqs_managed_sse_enabled":false}),
            json!({}),
            json!({}),
            UNKNOWN,
        ),
        (
            "aws_sqs_queue",
            json!({"sqs_managed_sse_enabled":false,"kms_master_key_id":null}),
            json!({}),
            json!({}),
            DISABLED,
        ),
        (
            "aws_sqs_queue",
            json!({"kms_master_key_id":"alias/aws/sqs"}),
            json!({}),
            json!({}),
            KMS,
        ),
        (
            "aws_sqs_queue",
            json!({"kms_master_key_id":"secret"}),
            json!({}),
            json!({"kms_master_key_id":true}),
            UNKNOWN,
        ),
        (
            "aws_sqs_queue",
            json!({"sqs_managed_sse_enabled":false,"kms_master_key_id":null}),
            json!({"kms_master_key_id":true}),
            json!({}),
            UNKNOWN,
        ),
        ("aws_s3_bucket", json!({}), json!({}), json!({}), UNKNOWN),
    ] {
        assert_eq!(
            summary(kind, &values, &unknown, &sensitive),
            expected,
            "{kind}: {values}"
        );
    }
    for (algorithm, expected) in [
        ("AES256", S3),
        ("aws:kms", KMS),
        ("aws:kms:dsse", DSSE),
        ("future", UNKNOWN),
    ] {
        let values = json!({"rule":[{"apply_server_side_encryption_by_default":[{"sse_algorithm":algorithm}]}]});
        assert_eq!(
            summary(
                "aws_s3_bucket_server_side_encryption_configuration",
                &values,
                &Value::Null,
                &Value::Null
            ),
            expected
        );
        assert_eq!(
            summary(
                "aws_s3_bucket_server_side_encryption_configuration",
                &values,
                &json!({"rule":true}),
                &Value::Null
            ),
            UNKNOWN
        );
        assert_eq!(
            summary(
                "aws_s3_bucket_server_side_encryption_configuration",
                &values,
                &Value::Null,
                &json!({"rule":[{"apply_server_side_encryption_by_default":true}]})
            ),
            UNKNOWN
        );
        assert_eq!(
            summary(
                "aws_s3_bucket",
                &json!({"server_side_encryption_configuration":[values]}),
                &Value::Null,
                &Value::Null
            ),
            expected
        );
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/kms-plan.json")).unwrap()
}

#[test]
fn historical_values_and_sensitive_key_references_do_not_borrow_current_configuration() {
    for case in [
        "delete",
        "deposed",
        "sensitive",
        "unknown",
        "literal",
        "partial",
        "sensitive_key",
        "mismatch",
    ] {
        let mut input = fixture();
        let change = &mut input["resource_changes"][2];
        match case {
            "sensitive_key" => {
                input["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true})
            }
            "mismatch" => change["change"]["after"]["kms_master_key_id"] = json!("different-key"),
            "delete" | "deposed" => {
                change["change"]["before"] =
                    json!({"sqs_managed_sse_enabled":false,"kms_master_key_id":null});
                if case == "delete" {
                    change["change"]["actions"] = json!(["delete"]);
                } else {
                    change["deposed"] = json!("old");
                }
            }
            "sensitive" => change["change"]["after_sensitive"] = json!({"kms_master_key_id":true}),
            "unknown" => change["change"]["after_unknown"] = json!({"kms_master_key_id":true}),
            "literal" => {
                input["configuration"]["root_module"]["resources"][2]["expressions"] = json!({})
            }
            "partial" => {
                input["configuration"]["root_module"]["resources"][2]["expressions"]["kms_master_key_id"] =
                    json!({"references":["aws_kms_key.data.arn","var.other"]})
            }
            _ => unreachable!(),
        }
        let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
        let queue = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_sqs_queue")
            .unwrap();
        let expected = match case {
            "delete" | "deposed" => DISABLED,
            "sensitive" | "unknown" => UNKNOWN,
            _ => KMS,
        };
        assert!(
            queue.references.iter().any(|r| r.label == expected),
            "{case}"
        );
        assert!(
            !queue
                .references
                .iter()
                .any(|r| r.label == "Encryption key reference"
                    && r.complete
                    && !r.targets.is_empty()),
            "{case}"
        );
    }
}

#[test]
fn s3_embedding_retains_identity_and_falls_back_when_selected_alone() {
    for case in [
        "valid",
        "literal",
        "sensitive",
        "ambiguous",
        "data",
        "deleted",
        "deposed",
        "missing",
        "foreign",
    ] {
        let mut input = fixture();
        match case {
            "data" => input["resource_changes"][6]["mode"] = json!("data"),
            "deleted" => input["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            "deposed" => input["resource_changes"][6]["deposed"] = json!("old"),
            "missing" => input["resource_changes"][5]["address"] = json!("aws_s3_bucket.other"),
            "foreign" => {
                input["resource_changes"][6]["provider_name"] = json!("example.com/custom/aws")
            }
            "literal" => {
                input["configuration"]["root_module"]["resources"][6]["expressions"]["bucket"] =
                    json!({"constant_value":"bucket-fixture"})
            }
            "sensitive" => {
                input["resource_changes"][6]["change"]["after_sensitive"] = json!({"bucket":true})
            }
            "ambiguous" => {
                input["configuration"]["root_module"]["resources"][6]["expressions"]["bucket"] =
                    json!({"references":["aws_s3_bucket.data.id","var.other"]})
            }
            _ => {}
        }
        let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
        let child = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_s3_bucket_server_side_encryption_configuration")
            .unwrap();
        assert_eq!(
            crate::view::presentation::owner(&graph, child).is_some(),
            case == "valid"
        );
        assert_eq!(graph.coverage.selected.resources, 7);
        let output = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-terraform-address") == Some(child.address.as_str()))
                .count(),
            1
        );
        assert!(output.contains(KMS));
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some(child.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert!(crate::view::presentation::owner(&selected, &selected.nodes[0]).is_none());
        let bucket = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_s3_bucket")
            .unwrap();
        let selected_bucket = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some(bucket.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            selected_bucket.nodes[0]
                .references
                .iter()
                .any(|r| r.label == UNKNOWN)
        );
        assert_eq!(selected.coverage.selected.resources, 1);
    }
}

#[test]
fn planned_values_masks_and_provider_dispatch_are_preserved() {
    for (mode, provider, mask, expected) in [
        (
            "managed",
            "registry.terraform.io/hashicorp/aws",
            json!({}),
            Some(ENABLED),
        ),
        (
            "data",
            "registry.terraform.io/hashicorp/aws",
            json!({}),
            Some(ENABLED),
        ),
        (
            "managed",
            "registry.terraform.io/hashicorp/aws",
            json!(true),
            Some(UNKNOWN),
        ),
        ("managed", "example.com/custom/aws", json!({}), None),
    ] {
        let input = json!({"format_version":"1.2", "planned_values":{"root_module":{"resources":[{"address":"aws_db_instance.example", "type":"aws_db_instance", "mode":mode, "provider_name":provider, "values":{"storage_encrypted":true}, "sensitive_values":mask}]}}});
        let parsed = crate::plan::parse(&input.to_string()).unwrap();
        assert_eq!(parsed.nodes[0].encryption, expected);
    }
}
