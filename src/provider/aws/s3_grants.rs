use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const GRANT: &str = "aws_s3control_access_grant";
const LOCATION: &str = "aws_s3control_access_grants_location";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (GRANT, "access_grants_location_id") => Some("Access Grants location"),
        (GRANT, "grantee.grantee_identifier") => Some("Access Grants IAM grantee"),
        (LOCATION, "iam_role_arn") => Some("Access Grants IAM role"),
        _ => None,
    }
}

fn known<'a>(v: &'a Value, m: &Value, path: &str) -> Option<&'a str> {
    super::nested_references::value(v, m, path)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn iam_arn(kind: &str, value: &str, endpoint: &TerraformEntity) -> bool {
    let parts: Vec<_> = value.splitn(6, ':').collect();
    let ["arn", partition, "iam", "", account, resource] = parts.as_slice() else {
        return false;
    };
    let prefix = if kind == "aws_iam_role" {
        "role/"
    } else {
        "user/"
    };
    !partition.is_empty()
        && resource.strip_prefix(prefix).is_some_and(|s| !s.is_empty())
        && crate::model::AwsAccountScope::new(account).is_some_and(|id| {
            endpoint
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref())
                .is_none_or(|a| a == &id)
        })
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if super::lambda_core::unique(refs, r.target, &r.attribute).is_none() {
        return false;
    }
    let [s] = r.sources.as_slice() else {
        return false;
    };
    let target = &nodes[*s];
    let (v, m) = evidence[r.target];
    let (sv, sm) = evidence[*s];
    let (kind, field, path) = match r.attribute.as_str() {
        "access_grants_location_id" => (
            LOCATION,
            "access_grants_location_id",
            "access_grants_location_id",
        ),
        "iam_role_arn" => ("aws_iam_role", "arn", "iam_role_arn"),
        "grantee.grantee_identifier" => {
            if !v["grantee"].as_array().is_some_and(|a| a.len() == 1)
                || known(v, m, "grantee.0.grantee_type") != Some("IAM")
                || !matches!(
                    target.resource_type.as_str(),
                    "aws_iam_role" | "aws_iam_user"
                )
            {
                return false;
            }
            (
                target.resource_type.as_str(),
                "arn",
                "grantee.0.grantee_identifier",
            )
        }
        _ => return false,
    };
    if super::route53::endpoint(nodes, r, kind, &[field]).is_none()
        || !super::route53::configured(&nodes[r.target], target)
    {
        return false;
    }
    let Some(actual) = known(v, m, path) else {
        return false;
    };
    if known(sv, sm, field) != Some(actual) || (field == "arn" && !iam_arn(kind, actual, target)) {
        return false;
    }
    if kind == LOCATION && !super::vpc_gateways::absent(sv, sm, "account_id") {
        let Some(account) =
            known(sv, sm, "account_id").and_then(crate::model::AwsAccountScope::new)
        else {
            return false;
        };
        if target
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            != Some(&account)
        {
            return false;
        }
    }
    if !super::vpc_gateways::absent(v, m, "account_id") {
        let Some(account) = known(v, m, "account_id").and_then(crate::model::AwsAccountScope::new)
        else {
            return false;
        };
        if nodes[r.target]
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            != Some(&account)
        {
            return false;
        }
        if kind == LOCATION
            && (known(sv, sm, "account_id") != Some(account.as_str())
                || target
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
                    != Some(&account))
        {
            return false;
        }
    }
    true
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).is_some()
                && verify(nodes, refs, evidence, r)
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            matches!(
                right.resource_type.as_str(),
                "aws_iam_role" | "aws_iam_user"
            ),
        )
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_grants.rs"]
mod tests;
