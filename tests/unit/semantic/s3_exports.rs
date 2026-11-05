use crate::{plan, semantic};
use serde_json::{Value, json};

fn build_graph(value: &Value) -> crate::model::ArchitectureGraph {
    let mut parsed = plan::parse(&value.to_string()).unwrap();
    for node in &mut parsed.nodes {
        if let Some(binding) = &mut node.provider_configuration {
            binding.account = crate::model::AwsAccountScope::new("123456789012");
        }
    }
    let evidence = value["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (&node["change"]["after"], &node["change"]["after_unknown"]))
        .collect::<Vec<_>>();
    super::annotate(&parsed.nodes, &mut parsed.attributes, &evidence);
    semantic::transform(&parsed)
}

fn nested(path: &str, leaf: Value) -> Value {
    let fields: Vec<_> = path.split('.').collect();
    let mut value = leaf;
    for (index, field) in fields.iter().enumerate().rev() {
        value = json!({*field: value});
        if index != 0 {
            value = json!([value]);
        }
    }
    value
}

fn fixture(kind: &str, path: &str, target: &str, collection: bool) -> Value {
    let arn = if target == "aws_kms_key" {
        "arn:aws:kms:us-west-2:123456789012:key/test"
    } else {
        "arn:aws:s3:::test"
    };
    let field = if path == "bucket" || path == "target_bucket" {
        "bucket"
    } else {
        "arn"
    };
    let actual = if collection { json!([arn]) } else { json!(arn) };
    let consumer = format!("{kind}.test");
    let endpoint = format!("{target}.test");
    let mut value = json!({
        "format_version":"1.2", "terraform_version":"1.9.8",
        "resource_changes":[
            {"address":endpoint,"type":target,"mode":"managed","change":{"actions":["no-op"],"after":{field:arn}}},
            {"address":consumer,"type":kind,"mode":"managed","change":{"actions":["create"],"after":nested(path,actual)}}
        ],
        "configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}}},"root_module":{"resources":[
            {"address":endpoint,"provider_config_key":"aws"},
            {"address":consumer,"provider_config_key":"aws","expressions":nested(path,json!({"references":[format!("{endpoint}.{field}")]}))}
        ]}}
    });
    if kind == "aws_s3_bucket_metadata_configuration" {
        let table = path.split('.').nth(1).unwrap();
        value["resource_changes"][1]["change"]["after"]["metadata_configuration"][0][table][0]["encryption_configuration"]
            [0]["sse_algorithm"] = json!("aws:kms");
    }
    if kind == "aws_s3control_storage_lens_configuration"
        && path.ends_with("s3_bucket_destination.arn")
    {
        let block = path.split('.').nth(1).unwrap();
        value["resource_changes"][1]["change"]["after"]["storage_lens_configuration"][0][block]
            [0]["s3_bucket_destination"][0]["account_id"] = json!("123456789012");
    }
    value
}

#[test]
fn s3_export_references_require_complete_current_typed_value_proof() {
    for (kind, path, target, collection) in [
        (
            "aws_s3_bucket_analytics_configuration",
            "bucket",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3_bucket_analytics_configuration",
            "storage_class_analysis.data_export.destination.s3_bucket_destination.bucket_arn",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3_bucket_inventory",
            "bucket",
            "aws_s3_directory_bucket",
            false,
        ),
        (
            "aws_s3_bucket_inventory",
            "destination.bucket.bucket_arn",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3_bucket_inventory",
            "destination.bucket.encryption.sse_kms.key_id",
            "aws_kms_key",
            false,
        ),
        (
            "aws_s3_bucket_logging",
            "target_bucket",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3_bucket_metadata_configuration",
            "metadata_configuration.inventory_table_configuration.encryption_configuration.kms_key_arn",
            "aws_kms_key",
            false,
        ),
        (
            "aws_s3_bucket_metadata_configuration",
            "metadata_configuration.journal_table_configuration.encryption_configuration.kms_key_arn",
            "aws_kms_key",
            false,
        ),
        (
            "aws_s3_bucket_replication_configuration",
            "role",
            "aws_iam_role",
            false,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.data_export.s3_bucket_destination.arn",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.expanded_prefixes_data_export.s3_bucket_destination.arn",
            "aws_s3_bucket",
            false,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.data_export.s3_bucket_destination.encryption.sse_kms.key_id",
            "aws_kms_key",
            false,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.aws_org.arn",
            "aws_organizations_organization",
            false,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.include.buckets",
            "aws_s3_bucket",
            true,
        ),
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.exclude.buckets",
            "aws_s3_bucket",
            true,
        ),
    ] {
        for invalid in 0..6 {
            let mut value = fixture(kind, path, target, collection);
            match invalid {
                1 => value["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
                2 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
                3 => value["resource_changes"][0]["mode"] = json!("data"),
                4 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                5 => value["resource_changes"][1]["change"]["after_unknown"] = json!(true),
                _ => (),
            }
            let graph = build_graph(&value);
            let reference = graph
                .nodes
                .iter()
                .find(|node| node.resource_type == kind)
                .unwrap()
                .references
                .iter()
                .find(|reference| reference.attribute == path && reference.label != "References")
                .unwrap();
            assert_eq!(
                reference.complete,
                invalid == 0,
                "{kind} {path} case {invalid}"
            );
            assert_eq!(graph.coverage.selected.resources, 2);
        }
    }
}

#[test]
fn metadata_kms_requires_kms_mode_and_selectors_require_the_whole_set() {
    let kind = "aws_s3_bucket_metadata_configuration";
    let path =
        "metadata_configuration.inventory_table_configuration.encryption_configuration.kms_key_arn";
    let mut value = fixture(kind, path, "aws_kms_key", false);
    value["resource_changes"][1]["change"]["after"]["metadata_configuration"][0]["inventory_table_configuration"]
        [0]["encryption_configuration"][0]["sse_algorithm"] = json!("AES256");
    let graph = build_graph(&value);
    assert!(
        !graph.nodes[1]
            .references
            .iter()
            .any(|reference| reference.attribute == path
                && reference.label != "References"
                && reference.complete)
    );
    let kind = "aws_s3control_storage_lens_configuration";
    let path = "storage_lens_configuration.include.buckets";
    let mut value = fixture(kind, path, "aws_s3_bucket", true);
    value["resource_changes"][1]["change"]["after"]["storage_lens_configuration"][0]["include"]
        [0]["buckets"] = json!(["arn:aws:s3:::test", "arn:aws:s3:::unreferenced"]);
    let graph = build_graph(&value);
    assert!(
        !graph.nodes[1]
            .references
            .iter()
            .any(|reference| reference.attribute == path
                && reference.label != "References"
                && reference.complete)
    );
}

#[test]
fn storage_lens_destination_owner_must_be_known_and_match() {
    let kind = "aws_s3control_storage_lens_configuration";
    let path = "storage_lens_configuration.data_export.s3_bucket_destination.arn";
    for account in [json!("999999999999"), Value::Null] {
        let mut value = fixture(kind, path, "aws_s3_bucket", false);
        value["resource_changes"][1]["change"]["after"]["storage_lens_configuration"][0]["data_export"]
            [0]["s3_bucket_destination"][0]["account_id"] = account;
        let graph = build_graph(&value);
        assert!(
            graph
                .nodes
                .iter()
                .flat_map(|node| &node.references)
                .filter(|reference| reference.attribute == path && reference.label != "References")
                .all(|reference| !reference.complete)
        );
    }
}

#[test]
fn analytics_destination_account_unknown_stays_unresolved() {
    let kind = "aws_s3_bucket_analytics_configuration";
    let path = "storage_class_analysis.data_export.destination.s3_bucket_destination.bucket_arn";
    let account_path = path.replace("bucket_arn", "bucket_account_id");
    for mask in [json!(true), json!({"invalid":false})] {
        let mut value = fixture(kind, path, "aws_s3_bucket", false);
        value["resource_changes"][1]["change"]["after_unknown"] = nested(&account_path, mask);
        let graph = build_graph(&value);
        let reference = graph
            .nodes
            .iter()
            .flat_map(|node| &node.references)
            .find(|reference| reference.attribute == path && reference.label != "References")
            .unwrap();
        assert!(!reference.complete);
    }
}
