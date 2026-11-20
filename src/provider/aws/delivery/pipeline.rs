use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_codepipeline" | "aws_codepipeline_webhook")
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codepipeline", "role_arn") => {
            Rule::new("Pipeline role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_codepipeline", "stage.action.role_arn") => Rule::new(
            "Pipeline action role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["stage", "stage.action"],
        ),
        ("aws_codepipeline", "artifact_store.location") => Rule::scalar_set(
            "Pipeline artifact bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
        )
        .with_explicit_destination(),
        ("aws_codepipeline", "artifact_store.encryption_key.id") => Rule::new(
            "Pipeline artifact encryption key",
            &[("aws_kms_key", &["arn", "id", "key_id"])],
            false,
            &["artifact_store"],
        )
        .with_explicit_destination(),
        ("aws_codepipeline_webhook", "target_pipeline") => Rule::new(
            "Webhook source pipeline",
            &[("aws_codepipeline", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    use crate::provider::aws::{configured_links::same_account, scope::scope};
    let consumer = r.target;
    if let Some(path) = path.strip_prefix("artifact_store.") {
        let Some((index, _)) = path.split_once('.') else {
            return false;
        };
        let store = format!("artifact_store.{index}");
        let selector = format!("{store}.region");
        let region = |index| scope(&e.nodes[index]).and_then(|scope| scope.region);
        let configured = e.text(consumer, &selector);
        let home = region(consumer);
        let destination = region(source);
        let wanted = configured.or_else(|| {
            e.absent(consumer, &selector)
                .then(|| home.as_ref().map(|region| region.0.as_str()))
                .flatten()
        });
        return wanted
            .zip(destination.as_ref())
            .is_some_and(|(wanted, actual)| wanted == actual.0)
            && same_account(
                e.nodes[consumer].provider_configuration.as_ref(),
                e.nodes[source].provider_configuration.as_ref(),
            )
            && e.text(consumer, &format!("{store}.type")) == Some("S3")
            && (r.attribute.ends_with(".location")
                || e.text(consumer, &format!("{store}.encryption_key.0.type")) == Some("KMS"));
    }
    if r.attribute != "target_pipeline" {
        return true;
    }
    let Some(action) = e.text(consumer, "target_action") else {
        return false;
    };
    let Some(actions) = e
        .value(source, "stage.0.action")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    let mut matches = (0..actions.len()).filter(|index| {
        e.text(source, &format!("stage.0.action.{index}.name")) == Some(action)
            && e.text(source, &format!("stage.0.action.{index}.category")) == Some("Source")
    });
    matches.next().is_some() && matches.next().is_none()
}
