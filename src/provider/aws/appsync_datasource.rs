use super::appsync::scalar;
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const SOURCE: &str = "aws_appsync_datasource";
const RDS: &str = "relational_database_config.http_endpoint_config";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if kind != SOURCE {
        return None;
    }
    match path {
        "api_id" => Some("Data source GraphQL API"),
        "service_role_arn" => Some("Data source service role"),
        "lambda_config.function_arn"
        | "dynamodb_config.table_name"
        | "dynamodb_config.delta_sync_config.delta_sync_table_name"
        | "event_bridge_config.event_bus_arn"
        | "relational_database_config.http_endpoint_config.aws_secret_store_arn"
        | "relational_database_config.http_endpoint_config.db_cluster_identifier"
        | "elasticsearch_config.endpoint"
        | "opensearchservice_config.endpoint" => Some("Typed data source backend"),
        _ => None,
    }
}

type Rule = (
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
);
fn rule(path: &str) -> Option<Rule> {
    match path {
        "api_id" => Some(("", super::appsync::API, &["id"], None)),
        "service_role_arn" => Some(("", "aws_iam_role", &["arn"], None)),
        "lambda_config.function_arn" => Some(("AWS_LAMBDA", "aws_lambda_function", &["arn"], None)),
        "dynamodb_config.table_name"
        | "dynamodb_config.delta_sync_config.delta_sync_table_name" => Some((
            "AMAZON_DYNAMODB",
            "aws_dynamodb_table",
            &["name", "id"],
            Some("dynamodb_config.region"),
        )),
        "event_bridge_config.event_bus_arn" => Some((
            "AMAZON_EVENTBRIDGE",
            "aws_cloudwatch_event_bus",
            &["arn"],
            None,
        )),
        "relational_database_config.http_endpoint_config.aws_secret_store_arn" => Some((
            "RELATIONAL_DATABASE",
            "aws_secretsmanager_secret",
            &["arn"],
            Some("relational_database_config.http_endpoint_config.region"),
        )),
        "relational_database_config.http_endpoint_config.db_cluster_identifier" => Some((
            "RELATIONAL_DATABASE",
            "aws_rds_cluster",
            &["id", "cluster_identifier"],
            Some("relational_database_config.http_endpoint_config.region"),
        )),
        "elasticsearch_config.endpoint" => Some((
            "AMAZON_ELASTICSEARCH",
            "aws_elasticsearch_domain",
            &["endpoint"],
            Some("elasticsearch_config.region"),
        )),
        "opensearchservice_config.endpoint" => Some((
            "AMAZON_OPENSEARCH_SERVICE",
            "aws_opensearch_domain",
            &["endpoint"],
            Some("opensearchservice_config.region"),
        )),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs.iter_mut() {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        let Some((typ, kind, outputs, region_path)) = rule(&r.attribute) else {
            continue;
        };
        let Some(s) = super::route53::endpoint(nodes, r, kind, outputs) else {
            continue;
        };
        if !super::route53::configured(node, &nodes[s]) {
            continue;
        }
        if !typ.is_empty() && scalar(evidence, r.target, "type") != Some(typ) {
            continue;
        }
        if r.attribute == "service_role_arn"
            && !matches!(
                scalar(evidence, r.target, "type"),
                Some(
                    "AWS_LAMBDA"
                        | "AMAZON_DYNAMODB"
                        | "AMAZON_ELASTICSEARCH"
                        | "AMAZON_EVENTBRIDGE"
                        | "AMAZON_OPENSEARCH_SERVICE"
                        | "RELATIONAL_DATABASE"
                )
            )
        {
            continue;
        }
        if typ == "RELATIONAL_DATABASE"
            && scalar(evidence, r.target, "relational_database_config.source_type")
                != Some("RDS_HTTP_ENDPOINT")
        {
            continue;
        }
        let value = scalar(evidence, r.target, &r.attribute);
        let expected = scalar(evidence, s, r.source_attributes.first().unwrap());
        let equal = value.is_some() && value == expected;
        let url = outputs == ["endpoint"]
            && value
                .zip(expected)
                .is_some_and(|(a, b)| a == format!("https://{b}"));
        if !equal && !url {
            continue;
        }
        if r.attribute == "dynamodb_config.delta_sync_config.delta_sync_table_name" {
            let (v, m) = evidence[r.target];
            if v["dynamodb_config"][0]["versioned"] != true
                || !matches!(
                    &m["dynamodb_config"]
                        .as_array()
                        .and_then(|a| a.first())
                        .unwrap_or(&Value::Null)["versioned"],
                    Value::Null | Value::Bool(false)
                )
            {
                continue;
            }
        }
        let own = super::scope::scope(node).and_then(|s| s.region);
        let other = super::scope::scope(&nodes[s]).and_then(|s| s.region);
        if own.is_none() {
            continue;
        }
        if kind != "aws_iam_role" {
            let region = if let Some(path) = region_path {
                let (v, m) = evidence[r.target];
                let Some(region) =
                    super::appsync::optional(v, m, &path.split('.').collect::<Vec<_>>())
                else {
                    continue;
                };
                region
            } else {
                None
            };
            if other.is_none()
                || region.map_or(own != other, |r| other.as_ref().is_none_or(|s| s.0 != r))
            {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
    let verified: Vec<_> = refs
        .iter()
        .filter(|r| nodes[r.target].resource_type == SOURCE && r.service_reference_verified)
        .map(|r| (r.target, r.attribute.clone()))
        .collect();
    for r in refs
        .iter_mut()
        .filter(|r| nodes[r.target].resource_type == SOURCE && r.attribute != "api_id")
    {
        r.service_reference_verified &= verified.contains(&(r.target, "api_id".to_owned()));
        if r.attribute.starts_with(RDS) {
            r.service_reference_verified &= ["aws_secret_store_arn", "db_cluster_identifier"]
                .iter()
                .all(|f| verified.contains(&(r.target, format!("{RDS}.{f}"))));
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    super::route53::bindings(raw, r)
        && (r.attribute == "api_id"
            || raw.attributes.iter().any(|a| {
                a.target == r.target
                    && a.attribute == "api_id"
                    && a.service_reference_verified
                    && super::route53::bindings(raw, a)
            }))
        && (!r.attribute.starts_with(RDS)
            || ["aws_secret_store_arn", "db_cluster_identifier"]
                .iter()
                .all(|field| {
                    raw.attributes.iter().any(|a| {
                        a.target == r.target
                            && a.attribute == format!("{RDS}.{field}")
                            && a.service_reference_verified
                            && super::route53::bindings(raw, a)
                    })
                }))
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync_datasource.rs"]
mod tests;
