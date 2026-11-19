use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ec2_capacity_reservation", "placement_group_arn") => Rule::new(
            "Reservation placement group",
            &[("aws_placement_group", &["arn"])],
            false,
            &[],
        ),
        ("aws_ec2_capacity_reservation" | "aws_ec2_host", "outpost_arn") => {
            Rule::new("Outpost", &[("aws_outposts_outpost", &["arn"])], false, &[]).data_source()
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_ec2_capacity_reservation" | "aws_ec2_host")
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
fn qualifies(_e: &Evidence<'_>, _r: &AttributeReference, _source: usize, _path: &str) -> bool {
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ec2_capacity.rs"]
mod tests;
