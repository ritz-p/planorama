use super::nested_references::value;
use crate::model::{
    Action, AttributeReference, AwsAccountScope, EntityMode, Graph, ReferenceKind,
    ResourceReference, TerraformEntity, TerraformPlan,
};
use serde_json::Value;

pub(super) fn region(s: &str) -> Option<&str> {
    let mut parts = s.split('-');
    let country = parts.next()?;
    let number = parts.next_back()?;
    let segments: Vec<_> = parts.collect();
    (country.len() == 2
        && country.bytes().all(|b| b.is_ascii_lowercase())
        && !segments.is_empty()
        && segments.len() <= 3
        && segments
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 12 && p.bytes().all(|b| b.is_ascii_lowercase()))
        && !number.is_empty()
        && number.len() <= 2
        && number.bytes().all(|b| b.is_ascii_digit()))
    .then_some(s)
}

type Details = Vec<(&'static str, &'static str, String)>;

fn selections(kind: &str, v: &Value, mask: &Value) -> Details {
    let mut details = Vec::new();
    match kind {
        "aws_cloudformation_stack_instances" => {
            if let Some(regions) = strings(v, mask, "regions", |s| region(s).is_some())
                .filter(|regions| !regions.is_empty())
            {
                details.push((
                    "regions",
                    "Configured deployment regions",
                    regions.join(", "),
                ));
            }
        }
        "aws_securityhub_aggregator_v2" | "aws_securityhub_finding_aggregator" => {
            let v2 = kind == "aws_securityhub_aggregator_v2";
            let (mode, regions) = if v2 {
                ("region_linking_mode", "linked_regions")
            } else {
                ("linking_mode", "specified_regions")
            };
            if let Some(home) = value(v, mask, "aggregation_region")
                .and_then(Value::as_str)
                .and_then(region)
                .filter(|_| v2)
            {
                details.push(("aggregation_region", "Aggregation region", home.to_owned()));
            }
            if let Some((mode, regions)) = value(v, mask, mode)
                .and_then(Value::as_str)
                .zip(strings(v, mask, regions, |s| region(s).is_some()))
            {
                let selection = match mode {
                    "ALL_REGIONS" if regions.is_empty() => Some("all regions".to_owned()),
                    "NO_REGIONS" if !v2 && regions.is_empty() => Some("no regions".to_owned()),
                    "ALL_REGIONS_EXCEPT_SPECIFIED" if !regions.is_empty() => {
                        Some(format!("all except {}", regions.join(", ")))
                    }
                    "SPECIFIED_REGIONS" if !regions.is_empty() => Some(regions.join(", ")),
                    _ => None,
                };
                if let Some(selection) = selection {
                    details.push((
                        if v2 {
                            "linked_regions"
                        } else {
                            "specified_regions"
                        },
                        "Linked regions",
                        selection,
                    ));
                }
            }
        }
        "aws_securitylake_subscriber" | "aws_securitylake_custom_log_source" => {
            let path = if kind == "aws_securitylake_subscriber" {
                "subscriber_identity.0.principal"
            } else {
                "configuration.0.provider_identity.0.principal"
            };
            if let Some(principal) = value(v, mask, path).and_then(Value::as_str).filter(|s| {
                AwsAccountScope::new(s).is_some()
                    || [".amazonaws.com", ".amazon.com"].iter().any(|suffix| {
                        s.strip_suffix(suffix).is_some_and(|prefix| {
                            !prefix.is_empty()
                                && prefix.split('.').all(|part| {
                                    !part.is_empty()
                                        && part.bytes().all(|b| {
                                            b.is_ascii_lowercase()
                                                || b.is_ascii_digit()
                                                || b == b'-'
                                        })
                                })
                        })
                    })
            }) {
                details.push((
                    if kind == "aws_securitylake_subscriber" {
                        "subscriber_identity.principal"
                    } else {
                        "configuration.provider_identity.principal"
                    },
                    "AWS principal",
                    principal.to_owned(),
                ));
            }
            if kind == "aws_securitylake_custom_log_source" {
                if let Some((name, version)) = value(v, mask, "source_name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .zip(
                        value(v, mask, "source_version")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty()),
                    )
                {
                    details.push((
                        "source_version",
                        "Custom log source",
                        format!("{name} / {version}"),
                    ));
                }
            }
        }
        "aws_securitylake_data_lake" => {
            for i in 0..v["configuration"].as_array().map_or(0, Vec::len) {
                let prefix = format!("configuration.{i}");
                if let Some(home) = value(v, mask, &format!("{prefix}.region"))
                    .and_then(Value::as_str)
                    .and_then(region)
                {
                    details.push(("configuration.region", "Lake region", home.to_owned()));
                    if let Some(regions) = strings(
                        v,
                        mask,
                        &format!("{prefix}.replication_configuration.0.regions"),
                        |s| region(s).is_some(),
                    )
                    .filter(|r| !r.is_empty())
                    {
                        details.push((
                            "configuration.replication_configuration.regions",
                            "Replication regions",
                            format!("{home} to {}", regions.join(", ")),
                        ));
                    }
                }
            }
        }
        "aws_securitylake_aws_log_source" => {
            for i in 0..v["source"].as_array().map_or(0, Vec::len) {
                let p = format!("source.{i}");
                let Some(name) = value(v, mask, &format!("{p}.source_name"))
                    .and_then(Value::as_str)
                    .filter(|s| {
                        matches!(
                            *s,
                            "ROUTE53"
                                | "VPC_FLOW"
                                | "SH_FINDINGS"
                                | "CLOUD_TRAIL_MGMT"
                                | "LAMBDA_EXECUTION"
                                | "S3_DATA"
                                | "EKS_AUDIT"
                                | "WAF"
                        )
                    })
                else {
                    continue;
                };
                if let Some((regions, accounts)) =
                    strings(v, mask, &format!("{p}.regions"), |s| region(s).is_some())
                        .zip(strings(v, mask, &format!("{p}.accounts"), |s| {
                            AwsAccountScope::new(s).is_some()
                        }))
                        .filter(|(r, _)| !r.is_empty())
                {
                    let accounts = if accounts.is_empty() {
                        "all lake accounts".to_owned()
                    } else {
                        accounts.join(", ")
                    };
                    details.push((
                        "source",
                        "AWS log source",
                        format!("{name} / {accounts} / {}", regions.join(", ")),
                    ));
                }
            }
        }
        "aws_config_configuration_aggregator" => {
            for block in [
                "account_aggregation_source",
                "organization_aggregation_source",
            ] {
                for i in 0..v[block].as_array().map_or(0, Vec::len) {
                    let prefix = format!("{block}.{i}");
                    let Some(regions) = selected_regions(v, mask, &prefix) else {
                        continue;
                    };
                    if block == "account_aggregation_source" {
                        if let Some(accounts) =
                            strings(v, mask, &format!("{prefix}.account_ids"), |s| {
                                AwsAccountScope::new(s).is_some()
                            })
                        {
                            if !accounts.is_empty() {
                                details.push((
                                    "account_aggregation_source",
                                    "Account aggregation source",
                                    format!("{} / {regions}", accounts.join(", ")),
                                ));
                            }
                        }
                    } else {
                        details.push((
                            "organization_aggregation_source",
                            "Organization aggregation regions",
                            regions,
                        ));
                    }
                }
            }
        }
        "aws_config_aggregate_authorization" => {
            if let Some((account, region)) = value(v, mask, "account_id")
                .and_then(Value::as_str)
                .and_then(AwsAccountScope::new)
                .zip(
                    value(v, mask, "authorized_aws_region")
                        .and_then(Value::as_str)
                        .and_then(region),
                )
            {
                details.push((
                    "authorized_aws_region",
                    "Authorized aggregator",
                    format!("{} / {region}", account.as_str()),
                ));
            }
        }
        "aws_auditmanager_assessment_delegation" => {
            if let Some((assessment, control)) = value(v, mask, "assessment_id")
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
                .zip(
                    value(v, mask, "control_set_id")
                        .and_then(Value::as_str)
                        .filter(|v| !v.is_empty()),
                )
            {
                details.push((
                    "control_set_id",
                    "Assessment control set",
                    format!("{assessment} / {control}"),
                ));
            }
        }
        "aws_waf_rule_group"
        | "aws_wafregional_rule_group"
        | "aws_waf_web_acl"
        | "aws_wafregional_web_acl" => {
            let (block, attribute) = match kind {
                "aws_waf_web_acl" => ("rules", "rules.priority"),
                "aws_wafregional_web_acl" => ("rule", "rule.priority"),
                _ => ("activated_rule", "activated_rule.priority"),
            };
            for index in 0..v[block].as_array().map_or(0, Vec::len) {
                let base = format!("{block}.{index}");
                if let Some((id, priority)) = value(v, mask, &format!("{base}.rule_id"))
                    .and_then(Value::as_str)
                    .zip(value(v, mask, &format!("{base}.priority")).and_then(Value::as_u64))
                {
                    details.push((attribute, "Rule priority", format!("{priority} / {id}")));
                }
            }
        }
        "aws_auditmanager_framework_share" => {
            if let Some((account, region)) = value(v, mask, "destination_account")
                .and_then(Value::as_str)
                .and_then(AwsAccountScope::new)
                .zip(
                    value(v, mask, "destination_region")
                        .and_then(Value::as_str)
                        .and_then(region),
                )
            {
                details.push((
                    "destination_region",
                    "Framework share destination",
                    format!("{} / {region}", account.as_str()),
                ));
            }
        }
        _ => {}
    }
    details
}

pub(super) fn stackset_target_region<'a>(
    node: usize,
    value_and_mask: (&'a Value, &Value),
    references: &[AttributeReference],
) -> Option<(&'static str, &'a str)> {
    let (v, mask) = value_and_mask;
    let field = "stack_set_instance_region";
    let configured = value(v, mask, field);
    let has_argument = |path| {
        references
            .iter()
            .any(|r| r.target == node && r.attribute == path && !r.root_constant_null)
    };
    let use_legacy = configured.is_some_and(Value::is_null)
        || (configured.is_none() && has_argument("region") && !has_argument(field));
    let (field, selected) = if use_legacy {
        ("region", value(v, mask, "region"))
    } else {
        (field, configured)
    };
    Some((field, region(selected?.as_str()?)?))
}

pub(crate) fn enrich(
    nodes: &mut [TerraformEntity],
    values: &[(&Value, &Value)],
    references: &[AttributeReference],
) {
    for (index, (node, &(v, mask))) in nodes.iter_mut().zip(values).enumerate() {
        if node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
        {
            node.configured_scopes = selections(&node.resource_type, v, mask);
            if node.resource_type == "aws_cloudformation_stack_set_instance" {
                if let Some((field, region)) = stackset_target_region(index, (v, mask), references)
                {
                    node.configured_scopes.push((
                        field,
                        "Configured deployment region",
                        region.to_owned(),
                    ));
                }
            }
        }
    }
}
pub(crate) fn facts(plan: &TerraformPlan, graph: &mut Graph) {
    for (fact, node) in plan.nodes.iter().zip(&mut graph.nodes) {
        for &(attribute, label, ref value) in &fact.configured_scopes {
            node.references.push(ResourceReference {
                kind: ReferenceKind::ConfiguredScope(value.clone()),
                preserve_spatial: false,
                show_spatial: true,
                label,
                attribute: attribute.into(),
                targets: Vec::new(),
                complete: true,
                dependency: false,
                issues: Default::default(),
            });
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/scope_selections.rs"]
mod tests;

fn strings<'a>(
    v: &'a Value,
    mask: &Value,
    path: &str,
    valid: impl Fn(&str) -> bool,
) -> Option<Vec<&'a str>> {
    let value = value(v, mask, path)?;
    if value.is_null() {
        return Some(Vec::new());
    }
    let mut result = value
        .as_array()?
        .iter()
        .map(|s| s.as_str().filter(|s| valid(s)))
        .collect::<Option<Vec<_>>>()?;
    result.sort_unstable();
    result.dedup();
    Some(result)
}
pub(super) fn selected_regions(v: &Value, mask: &Value, prefix: &str) -> Option<String> {
    let all = value(v, mask, &format!("{prefix}.all_regions"))?;
    let regions = strings(v, mask, &format!("{prefix}.regions"), |s| {
        region(s).is_some()
    })?;
    if all.as_bool() == Some(true) && regions.is_empty() {
        Some("all regions".into())
    } else if (all.is_null() || all.as_bool() == Some(false)) && !regions.is_empty() {
        Some(regions.join(", "))
    } else {
        None
    }
}
