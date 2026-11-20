use super::{configured_links::Evidence, lambda_core, scope};
use crate::model::AwsAccountScope;
use serde_json::Value;
use std::collections::BTreeSet;

fn bucket_part(arn: &str) -> Option<&str> {
    let base = arn.split('/').next()?;
    let (prefix, name) = base.rsplit_once(':')?;
    (matches!(
        prefix,
        "arn:aws:s3::" | "arn:aws-cn:s3::" | "arn:aws-us-gov:s3::"
    ) && !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.')))
    .then_some(base)
}

pub(super) fn bucket_arn<'a>(_: &str, value: &'a Value) -> Option<BTreeSet<&'a str>> {
    Some([bucket_part(value.as_str()?)?].into())
}

fn allowed_region(from: &str, to: &str) -> bool {
    from == to
        || (!from.starts_with("cn-")
            && !from.starts_with("us-gov-")
            && !from.starts_with("us-iso")
            && !from.starts_with("eu-isoe")
            && !from.starts_with("eusc-")
            && matches!(
                to,
                "us-east-1"
                    | "us-east-2"
                    | "us-west-1"
                    | "us-west-2"
                    | "ap-south-1"
                    | "ap-northeast-1"
                    | "ap-northeast-2"
                    | "ap-northeast-3"
                    | "ap-southeast-1"
                    | "ap-southeast-2"
                    | "ca-central-1"
                    | "eu-central-1"
                    | "eu-west-1"
                    | "eu-west-2"
                    | "eu-west-3"
                    | "eu-north-1"
                    | "sa-east-1"
            ))
}

fn referenced_bucket(e: &Evidence<'_>, target: usize, path: &str) -> Option<usize> {
    let reference = lambda_core::unique(e.refs, target, path)?;
    let [bucket] = *reference.sources.as_slice() else {
        return None;
    };
    let node = &e.nodes[bucket];
    if !node.provider.is_aws()
        || node.resource_type != "aws_s3_bucket"
        || node.mode != crate::model::EntityMode::Managed
        || node.action == crate::model::Action::Delete
        || node.deposed_key.is_some()
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
    {
        return None;
    }
    let outputs = reference.source_attributes_by_instance.get(&bucket)?;
    let field = outputs.first()?.as_str();
    let actual = e.text(target, path)?;
    let name = match field {
        "arn" => bucket_part(actual)?,
        "bucket" | "id" => actual,
        _ => return None,
    };
    (outputs.len() == 1 && Some(name) == e.text(bucket, field)).then_some(bucket)
}

pub(super) fn region_matches(
    e: &Evidence<'_>,
    target: usize,
    source: usize,
    bucket_path: &str,
    key_path: &str,
) -> bool {
    let region = |i: usize| scope(&e.nodes[i]).and_then(|s| s.region);
    let Some((from, to)) = region(target).zip(region(source)) else {
        return false;
    };
    if !allowed_region(&from.0, &to.0) {
        return false;
    }
    let key = e.nodes[source].resource_type == "aws_kms_key";
    if let Some(arn) = e.text(target, key_path) {
        let mut fields = arn.splitn(6, ':');
        if !super::s3_exports::kms_arn("aws_kms_key", arn) || fields.nth(3) != Some(to.0.as_str()) {
            return false;
        }
        let Some(account) = fields.next().and_then(AwsAccountScope::new) else {
            return false;
        };
        if key
            && e.nodes[source]
                .provider_configuration
                .as_ref()
                .and_then(|p| p.account.as_ref())
                .is_some_and(|a| a != &account)
        {
            return false;
        }
    }
    !key || referenced_bucket(e, target, bucket_path)
        .and_then(region)
        .is_none_or(|r| r == to)
}
