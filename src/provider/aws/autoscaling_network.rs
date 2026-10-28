use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/autoscaling_network.rs"]
mod tests;

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        (
            "aws_autoscaling_group",
            "mixed_instances_policy.launch_template.override.launch_template_specification.launch_template_id",
        ) => Some((
            "aws_launch_template",
            "id",
            true,
            "Override launch templates",
        )),
        (
            "aws_autoscaling_group",
            "mixed_instances_policy.launch_template.override.launch_template_specification.launch_template_name",
        ) => Some((
            "aws_launch_template",
            "name",
            true,
            "Override launch templates",
        )),
        ("aws_autoscaling_group", "vpc_zone_identifier") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_autoscaling_group", "launch_configuration") => Some((
            "aws_launch_configuration",
            "name",
            false,
            "Launch configuration",
        )),
        ("aws_autoscaling_group", "load_balancers") => {
            Some(("aws_elb", "name", true, "Classic load balancers"))
        }
        ("aws_autoscaling_group", "target_group_arns") => {
            Some(("aws_lb_target_group", "arn", true, "Traffic target groups"))
        }
        ("aws_autoscaling_group", "placement_group") => {
            Some(("aws_placement_group", "name", false, "Placement group"))
        }
        ("aws_autoscaling_group", "service_linked_role_arn") => Some((
            "aws_iam_service_linked_role",
            "arn",
            false,
            "Service-linked role",
        )),
        ("aws_autoscaling_group", "termination_policies") => {
            Some(("aws_lambda_function", "arn", true, "Termination functions"))
        }
        ("aws_launch_configuration", "security_groups") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_launch_configuration", "image_id") => Some(("aws_ami", "id", false, "Machine image")),
        ("aws_launch_configuration", "iam_instance_profile") => Some((
            "aws_iam_instance_profile",
            "name",
            false,
            "Instance profile",
        )),
        ("aws_launch_configuration", "key_name") => {
            Some(("aws_key_pair", "key_name", false, "Key pair"))
        }
        (
            "aws_autoscaling_attachment" | "aws_autoscaling_traffic_source_attachment",
            "autoscaling_group_name",
        ) => Some(("aws_autoscaling_group", "name", false, "Auto Scaling group")),
        ("aws_autoscaling_attachment", "elb") => {
            Some(("aws_elb", "name", false, "Classic load balancer"))
        }
        ("aws_autoscaling_attachment", "lb_target_group_arn") => {
            Some(("aws_lb_target_group", "arn", false, "Traffic target group"))
        }
        (
            "aws_autoscaling_group" | "aws_autoscaling_traffic_source_attachment",
            "traffic_source.identifier",
        ) => Some(("", "", true, "Traffic sources")),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.3)
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn compatible(consumer: &TerraformEntity, endpoint: &TerraformEntity) -> bool {
    let a = super::scope::scope(consumer);
    let b = super::scope::scope(endpoint);
    super::eventbridge::bindings(
        consumer.provider_configuration.as_ref(),
        endpoint.provider_configuration.as_ref(),
        a.as_ref().and_then(|s| s.region.as_ref()),
        b.as_ref().and_then(|s| s.region.as_ref()),
        matches!(
            endpoint.resource_type.as_str(),
            "aws_iam_instance_profile" | "aws_iam_service_linked_role"
        ),
    )
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let consumer = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        super::eventbridge::bindings(
            consumer.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            consumer
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            matches!(
                endpoint.resource_type.as_str(),
                "aws_iam_instance_profile" | "aws_iam_service_linked_role"
            ),
        )
    })
}

fn traffic<'a>(value: &'a Value, mask: &'a Value) -> Option<BTreeSet<(&'a str, &'a str)>> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let entries = value.get("traffic_source")?.as_array()?;
    let mask = &mask["traffic_source"];
    if entries.is_empty()
        || (!matches!(mask, Value::Null | Value::Bool(false))
            && mask.as_array().is_none_or(|m| m.len() != entries.len()))
    {
        return None;
    }
    let mut result = BTreeSet::new();
    let mut identifiers = BTreeSet::new();
    for (i, entry) in entries.iter().enumerate() {
        let m = mask.get(i).unwrap_or(&Value::Null);
        let identifier = super::streaming::values(entry, m, "identifier", false)?
            .into_iter()
            .next()?;
        let kind = super::streaming::values(entry, m, "type", false)?
            .into_iter()
            .next()?;
        if !matches!(kind, "elb" | "elbv2" | "vpc-lattice") || !identifiers.insert(identifier) {
            return None;
        }
        result.insert((kind, identifier));
    }
    Some(result)
}

fn overrides<'a>(
    value: &'a Value,
    mask: &Value,
    path: &[&str],
    values: &mut BTreeSet<&'a str>,
) -> Option<()> {
    if let Some(entries) = value.as_array() {
        if entries.is_empty() {
            return None;
        }
        let masks = match mask {
            Value::Null | Value::Bool(false) => None,
            Value::Array(masks) if masks.len() == entries.len() => Some(masks),
            _ => return None,
        };
        for (i, entry) in entries.iter().enumerate() {
            overrides(entry, masks.map_or(&Value::Null, |m| &m[i]), path, values)?;
        }
    } else if let Some((key, rest)) = path.split_first() {
        if !value.is_object()
            || !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        overrides(&value[*key], &mask[*key], rest, values)?;
    } else {
        if !matches!(mask, Value::Null | Value::Bool(false)) {
            return None;
        }
        values.insert(value.as_str().filter(|s| !s.is_empty())?);
    }
    Some(())
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || !r
                .sources
                .iter()
                .all(|&s| current(&nodes[s]) && compatible(node, &nodes[s]))
            || (!collection && r.sources.len() != 1)
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if node.resource_type == "aws_autoscaling_attachment"
            && matches!(r.attribute.as_str(), "elb" | "lb_target_group_arn")
        {
            let other = if r.attribute == "elb" {
                "lb_target_group_arn"
            } else {
                "elb"
            };
            if !matches!(&mask[other], Value::Null | Value::Bool(false))
                || !matches!(&value[other], Value::Null) && value[other].as_str() != Some("")
            {
                continue;
            }
        }
        if r.attribute == "traffic_source.identifier" {
            let Some(actual) = traffic(value, mask) else {
                continue;
            };
            if node.resource_type == "aws_autoscaling_traffic_source_attachment"
                && actual.len() != 1
            {
                continue;
            }
            let mut fields = BTreeSet::new();
            let expected = r
                .sources
                .iter()
                .map(|&source| {
                    let (kind, field) = match super::aliases::entity_type(&nodes[source]) {
                        "aws_elb" => ("elb", "name"),
                        "aws_lb_target_group" => ("elbv2", "arn"),
                        "aws_vpclattice_target_group" => ("vpc-lattice", "arn"),
                        _ => return None,
                    };
                    fields.insert(field.to_owned());
                    super::service_evidence::known(evidence, source, field).map(|id| (kind, id))
                })
                .collect::<Option<BTreeSet<_>>>();
            r.service_reference_verified = fields.len() == 1
                && fields == r.source_attributes
                && expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
            continue;
        }
        let field = if r.attribute == "autoscaling_group_name" && r.source_attributes.contains("id")
        {
            "id"
        } else {
            field
        };
        if r.source_attributes.len() != 1 || !r.source_attributes.contains(field) {
            continue;
        }
        let actual = if r.attribute.starts_with("mixed_instances_policy.") {
            let mut values = BTreeSet::new();
            overrides(
                value,
                mask,
                &r.attribute.split('.').collect::<Vec<_>>(),
                &mut values,
            )
            .map(|()| values)
        } else {
            super::streaming::values(value, mask, &r.attribute, collection)
        };
        let Some(actual) = actual else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (super::aliases::entity_type(&nodes[source]) == kind)
                    .then(|| super::service_evidence::known(evidence, source, field))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_autoscaling_group" {
            continue;
        }
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == "vpc_zone_identifier");
        let Some(r) = refs.next() else {
            continue;
        };
        if refs.next().is_some() || !super::service_references::verified(raw, r) {
            continue;
        }
        let Some(parents) = r
            .sources
            .iter()
            .map(|&i| super::streaming::vpc(raw, i))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().expect("one VPC");
        let mut evidence = Vec::new();
        for &member in &r.sources {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
