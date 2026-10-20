use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/rds_proxy.rs"]
mod tests;

pub(super) fn validate_targets(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let endpoint = |target: usize, attribute: &str| {
        let mut found = references
            .iter()
            .filter(|r| r.target == target && r.attribute == attribute);
        let reference = found.next()?;
        if found.next().is_some() || !reference.service_reference_verified {
            return None;
        }
        let [source] = reference.sources.as_slice() else {
            return None;
        };
        Some(*source)
    };
    let owned: BTreeSet<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(target, node)| {
            if node.resource_type != "aws_db_proxy_target" {
                return None;
            }
            let proxy = endpoint(target, "db_proxy_name")?;
            let group = endpoint(target, "target_group_name")?;
            (endpoint(group, "db_proxy_name") == Some(proxy)).then_some(target)
        })
        .collect();
    for reference in references {
        if nodes[reference.target].resource_type != "aws_db_proxy_target" {
            continue;
        }
        let opposite = match reference.attribute.as_str() {
            "target_group_name" => {
                reference.service_reference_verified &= owned.contains(&reference.target);
                continue;
            }
            "db_instance_identifier" => "db_cluster_identifier",
            "db_cluster_identifier" => "db_instance_identifier",
            _ => continue,
        };
        let (value, unknown) = evidence[reference.target];
        reference.service_reference_verified &= owned.contains(&reference.target)
            && value[opposite].is_null()
            && matches!(&unknown[opposite], Value::Null | Value::Bool(false));
    }
}
