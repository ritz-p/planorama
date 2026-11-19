use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_detective_invitation_accepter", "graph_arn") => Rule::new(
            "Invited Detective graph",
            &[("aws_detective_graph", &["graph_arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_detective_member" | "aws_detective_organization_configuration", "graph_arn") => {
            Rule::new(
                "Detective graph",
                &[("aws_detective_graph", &["graph_arn"])],
                false,
                &[],
            )
        }
        ("aws_detective_member" | "aws_detective_organization_admin_account", "account_id") => {
            Rule::new(
                "Detective account",
                &[("aws_organizations_account", &["id"])],
                false,
                &[],
            )
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_detective_invitation_accepter"
            | "aws_detective_member"
            | "aws_detective_organization_configuration"
            | "aws_detective_organization_admin_account"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let _ = path;
    if e.nodes[source].resource_type == "aws_organizations_account" {
        return e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some();
    }
    if e.nodes[r.target].resource_type != "aws_detective_invitation_accepter" {
        return true;
    }
    let Some(arn) = e.text(source, "graph_arn") else {
        return false;
    };
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", _, "detective", region, account, graph] = parts.as_slice() else {
        return false;
    };
    let scope = |i| super::scope(&e.nodes[i]);
    graph.starts_with("graph:")
        && scope(source)
            .and_then(|s| s.region)
            .is_some_and(|v| v.0 == *region)
        && scope(r.target)
            .and_then(|s| s.region)
            .is_some_and(|v| v.0 == *region)
        && scope(source)
            .and_then(|s| s.account)
            .is_some_and(|v| v.as_str() == *account)
        && scope(r.target).and_then(|s| s.account).is_some()
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/detective_graphs.rs"]
mod tests;
