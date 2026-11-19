use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    node.references.iter().any(|reference| {
        reference.attribute == path
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
            && super::label(&node.resource_type, path) == Some(reference.label)
    })
}

#[test]
fn configured_links_require_current_typed_evidence() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/kendra-index-plan.json")).unwrap();
    for (address, path) in [
        ("aws_kendra_index.sample1", "role_arn"),
        (
            "aws_kendra_index.sample1",
            "server_side_encryption_configuration.kms_key_id",
        ),
        (
            "aws_kendra_index.sample1",
            "user_token_configurations.jwt_token_type_configuration.secrets_manager_arn",
        ),
        ("aws_kendra_data_source.sample3", "index_id"),
        ("aws_kendra_data_source.sample3", "role_arn"),
        (
            "aws_kendra_data_source.sample3",
            "configuration.s3_configuration.bucket_name",
        ),
        (
            "aws_kendra_data_source.sample3",
            "custom_document_enrichment_configuration.role_arn",
        ),
        (
            "aws_kendra_data_source.sample3",
            "custom_document_enrichment_configuration.pre_extraction_hook_configuration.lambda_arn",
        ),
        (
            "aws_kendra_data_source.sample3",
            "custom_document_enrichment_configuration.pre_extraction_hook_configuration.s3_bucket",
        ),
        (
            "aws_kendra_data_source.sample3",
            "custom_document_enrichment_configuration.post_extraction_hook_configuration.lambda_arn",
        ),
        (
            "aws_kendra_data_source.sample3",
            "custom_document_enrichment_configuration.post_extraction_hook_configuration.s3_bucket",
        ),
        ("aws_kendra_faq.sample4", "index_id"),
        ("aws_kendra_faq.sample4", "role_arn"),
        ("aws_kendra_faq.sample4", "s3_path.bucket"),
        ("aws_kendra_experience.sample5", "index_id"),
        ("aws_kendra_experience.sample5", "role_arn"),
        (
            "aws_kendra_experience.sample5",
            "configuration.content_source_configuration.data_source_ids",
        ),
        (
            "aws_kendra_experience.sample5",
            "configuration.content_source_configuration.faq_ids",
        ),
        (
            "aws_kendra_query_suggestions_block_list.sample9",
            "index_id",
        ),
        (
            "aws_kendra_query_suggestions_block_list.sample9",
            "role_arn",
        ),
        (
            "aws_kendra_query_suggestions_block_list.sample9",
            "source_s3_path.bucket",
        ),
        ("aws_kendra_thesaurus.sample10", "index_id"),
        ("aws_kendra_thesaurus.sample10", "role_arn"),
        ("aws_kendra_thesaurus.sample10", "source_s3_path.bucket"),
    ] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["user_token_configurations"][0]["jwt_token_type_configuration"]
        [0]["key_location"] = json!("URL");
    assert!(!verified(
        &changed,
        "aws_kendra_index.sample1",
        "user_token_configurations.jwt_token_type_configuration.secrets_manager_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["customer_master_key_spec"] =
        json!("RSA_2048");
    assert!(!verified(
        &changed,
        "aws_kendra_index.sample1",
        "server_side_encryption_configuration.kms_key_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["type"] = json!("CUSTOM");
    assert!(!verified(
        &changed,
        "aws_kendra_data_source.sample3",
        "role_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["configuration"][0]["template_configuration"] =
        json!([{"template":"{}"}]);
    assert!(!verified(
        &changed,
        "aws_kendra_data_source.sample3",
        "configuration.s3_configuration.bucket_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["index_id"] = json!("another-index");
    assert!(!verified(
        &changed,
        "aws_kendra_experience.sample5",
        "configuration.content_source_configuration.data_source_ids"
    ));
    let mut changed = fixture.clone();
    changed["configuration"]["root_module"]["resources"][5]["expressions"]["configuration"][0]["content_source_configuration"]
        [0]["faq_ids"] = json!({"references":["aws_kendra_faq.sample4.id"]});
    assert!(!verified(
        &changed,
        "aws_kendra_experience.sample5",
        "configuration.content_source_configuration.faq_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["change"]["after"]["configuration"][0]["content_source_configuration"]
        [0]["data_source_ids"] = json!(["fixture-document-source", "unproven-source"]);
    assert!(!verified(
        &changed,
        "aws_kendra_experience.sample5",
        "configuration.content_source_configuration.data_source_ids"
    ));
}

#[test]
fn content_requires_the_same_index_resource_even_when_ids_match() {
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/kendra-index-plan.json")).unwrap();
    let graph = semantic::transform(&plan::parse(&fixture.to_string()).unwrap());
    let index = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_kendra_index.sample1")
        .unwrap();
    assert_eq!(
        graph.coverage.selected.resources,
        fixture["resource_changes"].as_array().unwrap().len()
    );
    assert_eq!(index.placement, crate::model::Placement::Spatial);
    for node in graph.nodes.iter().filter(|node| {
        node.resource_type.starts_with("aws_kendra_") && node.address != index.address
    }) {
        assert_eq!(node.placement, crate::model::Placement::Logical);
        assert_eq!(
            node.member_owner.as_ref().unwrap().target,
            index.entity.provenance[0]
        );
    }

    let mut duplicate = fixture["resource_changes"][1].clone();
    duplicate["address"] = json!("aws_kendra_index.other");
    duplicate["name"] = json!("other");
    fixture["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let resources = fixture["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    let mut duplicate = resources[1].clone();
    duplicate["address"] = json!("aws_kendra_index.other");
    duplicate["name"] = json!("other");
    resources.push(duplicate);
    resources[4]["expressions"]["index_id"] = json!({"references":["aws_kendra_index.other.id"]});
    assert!(!verified(
        &fixture,
        "aws_kendra_experience.sample5",
        "configuration.content_source_configuration.faq_ids"
    ));
    assert!(verified(
        &fixture,
        "aws_kendra_experience.sample5",
        "configuration.content_source_configuration.data_source_ids"
    ));
}
