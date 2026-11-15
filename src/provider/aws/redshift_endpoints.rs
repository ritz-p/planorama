use crate::model::{AttributeReference, AwsAccountScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/redshift_endpoints.rs"]
mod tests;
const ACCESS: &str = "aws_redshift_endpoint_access";
const AUTH: &str = "aws_redshift_endpoint_authorization";
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (ACCESS, "cluster_identifier") => Some("Accessed cluster"),
        (ACCESS, "subnet_group_name") => Some("Endpoint subnet group"),
        (ACCESS, "vpc_security_group_ids") => Some("Endpoint security groups"),
        (AUTH, "cluster_identifier") => Some("Authorized cluster"),
        (AUTH, "vpc_ids") => Some("Grantee VPCs"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (ACCESS | AUTH, "cluster_identifier", "aws_redshift_cluster") => &["cluster_identifier"],
        (ACCESS, "subnet_group_name", "aws_redshift_subnet_group") => &["name", "id"],
        (ACCESS, "vpc_security_group_ids", "aws_security_group") | (AUTH, "vpc_ids", "aws_vpc") => {
            &["id"]
        }
        _ => &[],
    }
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            label(k, &r.attribute)?;
            let proof = || {
                let (v, m) = e[r.target];
                let value = super::nested_references::value(v, m, &r.attribute)?;
                let collection =
                    matches!(r.attribute.as_str(), "vpc_security_group_ids" | "vpc_ids");
                if if collection {
                    !value.is_array()
                } else {
                    !value.is_string()
                } {
                    return None;
                }
                super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true)?;
                let qualifier = match (k, r.attribute.as_str()) {
                    (ACCESS, "cluster_identifier") => Some("resource_owner"),
                    (AUTH, "vpc_ids") => Some("account"),
                    _ => None,
                };
                let account = if let Some(p) = qualifier {
                    let value = super::nested_references::value(v, m, p)?;
                    if value.is_null() && k == ACCESS {
                        None
                    } else {
                        Some(AwsAccountScope::new(value.as_str()?)?)
                    }
                } else {
                    None
                };
                if k == ACCESS
                    && r.attribute != "cluster_identifier"
                    && !super::service_evidence::eligible(nodes, r)
                {
                    return None;
                }
                Some(account)
            };
            Some(proof())
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(proof) = result {
            r.service_reference_verified = proof.is_some();
            r.service_reference_owner_account = proof.flatten();
        }
    }
    super::network::annotate_subnet_group(nodes, refs, e, ACCESS, "subnet_group_name");
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let l = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let right = &raw.nodes[s];
        let mut binding = l.provider_configuration.clone();
        if let Some(account) = &r.service_reference_owner_account {
            let Some(b) = binding.as_mut() else {
                return false;
            };
            if b.account_conflict {
                return false;
            }
            b.account = Some(account.clone());
        }
        super::eventbridge::bindings(
            binding.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            false,
        )
    })
}
