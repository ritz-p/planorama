use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const VM_CLUSTER: &str = "aws_odb_cloud_vm_cluster";
const AUTONOMOUS_CLUSTER: &str = "aws_odb_cloud_autonomous_vm_cluster";
const AUTONOMOUS_DATABASE: &str = "aws_odb_autonomous_database";
const PEERING: &str = "aws_odb_network_peering_connection";
const ROLE_ASSOCIATION: &str = "aws_odb_iam_role_association";
const NETWORK: &str = "aws_odb_network";

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (VM_CLUSTER | AUTONOMOUS_CLUSTER, "cloud_exadata_infrastructure_id") => Rule::new(
            "Exadata infrastructure",
            &[("aws_odb_cloud_exadata_infrastructure", &["id"])],
            false,
            &[],
        ),
        (VM_CLUSTER | AUTONOMOUS_CLUSTER, "cloud_exadata_infrastructure_arn") => Rule::new(
            "Exadata infrastructure",
            &[("aws_odb_cloud_exadata_infrastructure", &["arn"])],
            false,
            &[],
        ),
        (VM_CLUSTER | AUTONOMOUS_CLUSTER | AUTONOMOUS_DATABASE | PEERING, "odb_network_id") => {
            Rule::new("ODB network", &[(NETWORK, &["id"])], false, &[])
        }
        (VM_CLUSTER | AUTONOMOUS_CLUSTER | PEERING, "odb_network_arn") => {
            Rule::new("ODB network", &[(NETWORK, &["arn"])], false, &[])
        }
        (PEERING, "peer_network_id") => Rule::new(
            "Peer network",
            &[(NETWORK, &["id"]), ("aws_vpc", &["id"])],
            false,
            &[],
        ),
        (ROLE_ASSOCIATION, "resource_arn") => Rule::new(
            "KmsTde cluster",
            &[(VM_CLUSTER, &["arn"]), (AUTONOMOUS_CLUSTER, &["arn"])],
            false,
            &[],
        ),
        (ROLE_ASSOCIATION, "iam_role_arn") => {
            Rule::new("KmsTde IAM role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies);
}

fn qualifies(
    evidence: &Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
    _: &str,
) -> bool {
    let target = reference.target;
    let configured = |path| evidence.has_argument(target, path);
    let selected = |path| configured(path) && evidence.text(target, path).is_some();

    match evidence.nodes[target].resource_type.as_str() {
        VM_CLUSTER | AUTONOMOUS_CLUSTER => {
            let id_pair = selected("odb_network_id")
                && selected("cloud_exadata_infrastructure_id")
                && !configured("odb_network_arn")
                && !configured("cloud_exadata_infrastructure_arn");
            let arn_pair = selected("odb_network_arn")
                && selected("cloud_exadata_infrastructure_arn")
                && !configured("odb_network_id")
                && !configured("cloud_exadata_infrastructure_id");
            if reference.attribute.ends_with("_id") {
                id_pair
            } else {
                arn_pair
            }
        }
        ROLE_ASSOCIATION => evidence.text(target, "aws_integration") == Some("KmsTde"),
        PEERING => {
            if configured("odb_network_id") && configured("odb_network_arn") {
                return false;
            }
            let counterpart = if reference.attribute == "peer_network_id" {
                if configured("odb_network_arn") {
                    "odb_network_arn"
                } else {
                    "odb_network_id"
                }
            } else {
                "peer_network_id"
            };
            let output = if counterpart.ends_with("_arn") {
                "arn"
            } else {
                "id"
            };
            evidence.configured_endpoint(target, counterpart, NETWORK, &[output]) != Some(endpoint)
        }
        _ => true,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/odb.rs"]
mod tests;
