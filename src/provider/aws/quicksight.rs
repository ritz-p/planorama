mod content;
mod identity;

use crate::model::{AttributeReference, AwsAccountScope, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const CONNECTION: &str = "aws_quicksight_vpc_connection";
const SOURCE: &str = "aws_quicksight_data_source";

type Rule = (&'static str, &'static [&'static str], &'static str);

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (CONNECTION, "subnet_ids") => ("aws_subnet", &["id"], "Connection subnets"),
        (CONNECTION, "security_group_ids") => {
            ("aws_security_group", &["id"], "Connection security groups")
        }
        (CONNECTION, "role_arn") => ("aws_iam_role", &["arn"], "Connection role"),
        (SOURCE, "vpc_connection_properties.vpc_connection_arn") => {
            (CONNECTION, &["arn"], "VPC connection")
        }
        (SOURCE, "credentials.copy_source_arn") => (SOURCE, &["arn"], "Credential source"),
        (SOURCE, "credentials.secret_arn") => {
            ("aws_secretsmanager_secret", &["arn"], "Credential secret")
        }
        (SOURCE, "parameters.amazon_elasticsearch.domain") => (
            "aws_elasticsearch_domain",
            &["domain_name"],
            "Elasticsearch domain",
        ),
        (SOURCE, "parameters.athena.work_group") => {
            ("aws_athena_workgroup", &["name"], "Athena workgroup")
        }
        (SOURCE, "parameters.athena.role_arn" | "parameters.s3.role_arn") => {
            ("aws_iam_role", &["arn"], "Data access role")
        }
        (SOURCE, "parameters.rds.instance_id") => {
            ("aws_db_instance", &["identifier"], "RDS instance")
        }
        (SOURCE, "parameters.redshift.cluster_id") => (
            "aws_redshift_cluster",
            &["cluster_identifier"],
            "Redshift cluster",
        ),
        (SOURCE, "parameters.s3.manifest_file_location.bucket") => {
            ("aws_s3_bucket", &["id", "bucket"], "Manifest bucket")
        }
        _ => return content::rule(kind, path).or_else(|| identity::rule(kind, path)),
    })
}

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, CONNECTION | SOURCE) || content::resource(kind) || identity::resource(kind)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind)
        && path.split('.').any(|p| p.parse::<usize>().is_ok())
        && label(
            kind,
            &path
                .split('.')
                .filter(|p| p.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}

fn fields(kind: &str, path: &str, endpoint: &str) -> &'static [&'static str] {
    if kind == "aws_quicksight_user" && path == "iam_arn" && endpoint == "aws_iam_user" {
        return &["arn"];
    }
    if kind == "aws_quicksight_folder_membership" && path == "member_id" {
        return content::member_output(endpoint);
    }
    rule(kind, path)
        .filter(|r| r.0 == endpoint)
        .map_or(&[], |r| r.1)
}

fn account(node: &TerraformEntity, value: &Value, mask: &Value) -> Option<Option<AwsAccountScope>> {
    let provider = node.provider_configuration.as_ref()?;
    let mut account = provider.account.clone();
    let mut agree = |id: &str| -> Option<()> {
        let id = AwsAccountScope::new(id)?;
        if account.as_ref().is_some_and(|current| current != &id) {
            return None;
        }
        account = Some(id);
        Some(())
    };
    if let Some(arn) = super::nested_references::value(value, mask, "arn").and_then(Value::as_str) {
        let mut fields = arn.splitn(6, ':');
        if fields.next() != Some("arn") {
            return None;
        }
        let partition = fields.next()?;
        let service = fields.next()?;
        let region = fields.next()?;
        let id = fields.next()?;
        if partition.is_empty() || service.is_empty() || fields.next()?.is_empty() {
            return None;
        }
        if !region.is_empty()
            && super::scope::scope(node)
                .and_then(|s| s.region)
                .is_some_and(|r| r.0 != region)
        {
            return None;
        }
        if !id.is_empty() {
            agree(id)?;
        }
    }
    if node.resource_type.starts_with("aws_quicksight_") {
        let selected = super::nested_references::value(value, mask, "aws_account_id")?;
        if !selected.is_null() {
            agree(selected.as_str()?)?;
        }
    }
    Some(account)
}

fn qualified(value: &Value, mask: &Value, path: &str) -> bool {
    let mut parts = path.split('.').peekable();
    let mut indexed = Vec::new();
    while let Some(field) = parts.next() {
        indexed.push(field);
        if parts.peek().is_some() {
            if !super::nested_references::singleton(value, mask, &indexed.join(".")) {
                return false;
            }
            indexed.push("0");
        }
    }
    let Some(leaf) = super::nested_references::value(value, mask, &indexed.join(".")) else {
        return false;
    };
    if matches!(path, "subnet_ids" | "security_group_ids") {
        return leaf.is_array();
    }
    if !leaf.is_string() {
        return false;
    }
    if let Some(rest) = path.strip_prefix("credentials.") {
        return ["copy_source_arn", "secret_arn", "credential_pair"]
            .into_iter()
            .filter(|k| *k != rest)
            .all(|k| {
                super::nested_references::value(value, mask, &format!("credentials.0.{k}"))
                    .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
            });
    }
    let Some(parameter) = path
        .strip_prefix("parameters.")
        .and_then(|s| s.split('.').next())
    else {
        return true;
    };
    super::nested_references::value(value, mask, "type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == parameter.to_ascii_uppercase())
        && value["parameters"][0].as_object().is_some_and(|p| {
            p.keys()
                .chain(
                    mask["parameters"][0]
                        .as_object()
                        .into_iter()
                        .flat_map(|m| m.keys()),
                )
                .all(|k| {
                    k == parameter
                        || super::nested_references::value(
                            value,
                            mask,
                            &format!("parameters.0.{k}"),
                        )
                        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
                })
        })
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let left = &raw.nodes[r.target];
    label(&left.resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                left.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                matches!(
                    right.resource_type.as_str(),
                    "aws_iam_role" | "aws_iam_user" | "aws_iam_policy"
                ),
            )
        })
}

fn decode<'a>(path: &str, actual: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    if matches!(
        path,
        "subnet_ids" | "security_group_ids" | "identities.group" | "identities.user"
    ) {
        if !actual.is_array() {
            return None;
        }
        super::nested_references::leaf_values(path, actual)
    } else {
        content::decode(path, actual)
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let node = &nodes[r.target];
            label(&node.resource_type, &r.attribute)?;
            let (v, m) = evidence[r.target];
            let content = content::resource(&node.resource_type);
            let identity = identity::resource(&node.resource_type);
            let compatible = |s: usize| {
                let endpoint = &nodes[s];
                let (ev, em) = evidence[s];
                account(node, v, m)
                    .zip(account(endpoint, ev, em))
                    .is_some_and(|(a, b)| a == b)
                    && super::eventbridge::bindings(
                        node.provider_configuration.as_ref(),
                        endpoint.provider_configuration.as_ref(),
                        super::scope::scope(node)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(endpoint)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        matches!(
                            endpoint.resource_type.as_str(),
                            "aws_iam_role" | "aws_iam_user" | "aws_iam_policy"
                        ),
                    )
            };
            let verified = if r.attribute == "iam_identity_center_instance_arn" {
                super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                    && super::service_evidence::known(evidence, r.target, "authentication_method")
                        == Some("IAM_IDENTITY_CENTER")
                    && super::service_evidence::known(evidence, r.target, &r.attribute).is_some_and(
                        |v| super::identity_center_instance::verify(nodes, r, evidence, v),
                    )
                    && compatible(r.sources[0])
            } else {
                (content || identity || qualified(v, m, &r.attribute))
                    && super::nested_references::verify_values(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |s, path| {
                            compatible(s)
                                && (!content
                                    || content::qualifies(nodes, refs, evidence, r, s, path))
                                && (!identity || identity::qualifies(nodes, evidence, r, s, path))
                        },
                        (
                            if identity {
                                &["key_registration"]
                            } else {
                                content::COLLECTIONS
                            },
                            decode,
                            crate::model::EntityMode::Managed,
                        ),
                    )
                    .is_some()
            };
            let parents = if verified
                && node.resource_type == CONNECTION
                && matches!(r.attribute.as_str(), "subnet_ids" | "security_group_ids")
            {
                r.sources
                    .iter()
                    .map(|&s| super::lambda_core::parent(nodes, refs, evidence, s))
                    .collect::<Option<std::collections::BTreeSet<_>>>()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            Some((verified, parents))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((verified, parents)) = result {
            r.service_reference_verified = verified;
            if verified {
                r.complete = true;
            }
            r.endpoint_network_context = parents;
        }
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        CONNECTION,
        ("subnet_ids", "security_group_ids"),
        bindings,
    );
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/quicksight.rs"]
mod tests;
