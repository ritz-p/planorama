use super::nested_references::value;
use crate::model::{
    Action, AwsAccountScope, EntityMode, Graph, ReferenceKind, ResourceReference, TerraformEntity,
    TerraformPlan,
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

pub(crate) fn enrich(nodes: &mut [TerraformEntity], values: &[(&Value, &Value)]) {
    for (node, &(v, mask)) in nodes.iter_mut().zip(values) {
        if node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
        {
            node.configured_scopes = selections(&node.resource_type, v, mask);
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
