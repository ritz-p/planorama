use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_dataexchange_revision" | "aws_dataexchange_revision_assets", "data_set_id") => {
            Rule::new(
                "Dataset",
                &[("aws_dataexchange_data_set", &["id"])],
                false,
                &[],
            )
        }
        (
            "aws_dataexchange_revision_assets",
            "asset.create_s3_data_access_from_s3_bucket.asset_source.bucket",
        ) => Rule::new(
            "Shared asset bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "asset",
                "asset.create_s3_data_access_from_s3_bucket.asset_source",
            ],
        ),
        ("aws_dataexchange_revision_assets", "asset.import_assets_from_s3.asset_source.bucket") => {
            Rule::new(
                "Imported asset bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &["asset", "asset.import_assets_from_s3.asset_source"],
            )
        }
        (
            "aws_dataexchange_revision_assets",
            "asset.create_s3_data_access_from_s3_bucket.asset_source.kms_keys_to_grant.kms_key_arn",
        ) => Rule::new(
            "Asset access key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[
                "asset",
                "asset.create_s3_data_access_from_s3_bucket.asset_source",
                "asset.create_s3_data_access_from_s3_bucket.asset_source.kms_keys_to_grant",
            ],
        ),
        ("aws_dataexchange_event_action", "event.revision_published.data_set_id") => Rule::new(
            "Published dataset",
            &[("aws_dataexchange_data_set", &["id"])],
            false,
            &[],
        ),
        (
            "aws_dataexchange_event_action",
            "action.export_revision_to_s3.revision_destination.bucket",
        ) => Rule::new(
            "Revision export bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        (
            "aws_dataexchange_event_action",
            "action.export_revision_to_s3.encryption.kms_key_arn",
        ) => Rule::new(
            "Revision export key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_dataexchange_revision"
            | "aws_dataexchange_revision_assets"
            | "aws_dataexchange_event_action"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    let attribute = r.attribute.as_str();
    if attribute.starts_with("asset.") {
        let dataset = e.configured_endpoint(
            r.target,
            "data_set_id",
            "aws_dataexchange_data_set",
            &["id"],
        );
        let mut fields = path.split('.');
        let (Some("asset"), Some(index), Some(variant)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return false;
        };
        let expected = if variant == "create_s3_data_access_from_s3_bucket" {
            "S3_DATA_ACCESS"
        } else {
            "S3_SNAPSHOT"
        };
        return dataset.and_then(|dataset| e.text(dataset, "asset_type")) == Some(expected)
            && [
                "create_s3_data_access_from_s3_bucket",
                "import_assets_from_s3",
                "import_assets_from_signed_url",
            ]
            .into_iter()
            .filter(|other| *other != variant)
            .all(|other| {
                e.value(r.target, &format!("asset.{index}.{other}"))
                    .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
            });
    }
    if matches!(
        attribute,
        "action.export_revision_to_s3.revision_destination.bucket"
            | "action.export_revision_to_s3.encryption.kms_key_arn"
    ) {
        return e
            .configured_endpoint(
                r.target,
                "event.0.revision_published.0.data_set_id",
                "aws_dataexchange_data_set",
                &["id"],
            )
            .and_then(|dataset| e.text(dataset, "asset_type"))
            == Some("S3_SNAPSHOT")
            && (attribute != "action.export_revision_to_s3.encryption.kms_key_arn"
                || e.text(
                    r.target,
                    "action.0.export_revision_to_s3.0.encryption.0.type",
                ) == Some("aws:kms"));
    }
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dataexchange_configuration.rs"]
mod tests;
