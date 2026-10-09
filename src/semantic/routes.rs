use crate::model::TerraformGraph;

#[cfg(test)]
#[path = "../../tests/unit/semantic/routes.rs"]
mod tests;

pub(super) fn endpoints(raw: &TerraformGraph, route: usize) -> Option<(usize, usize)> {
    let mut targets = raw.attributes.iter().filter(|attribute| {
        attribute.target == route
            && (attribute.attribute.ends_with("_id") || attribute.attribute.ends_with("_arn"))
            && !matches!(
                attribute.attribute.as_str(),
                "route_table_id" | "destination_prefix_list_id"
            )
    });
    let target = targets.next()?;
    // Even a second literal/unsupported target makes the route ambiguous.
    if targets.next().is_some() {
        return None;
    }
    let target_type = match target.attribute.as_str() {
        "gateway_id" => "aws_internet_gateway",
        "nat_gateway_id" => "aws_nat_gateway",
        "transit_gateway_id" => "aws_ec2_transit_gateway",
        "vpc_peering_connection_id" => "aws_vpc_peering_connection",
        _ => return None,
    };
    let table = raw
        .attributes
        .iter()
        .find(|a| a.target == route && a.attribute == "route_table_id")?;
    if !target.ids_only || !table.ids_only {
        return None;
    }
    let from = super::associations::endpoint(raw, route, "route_table_id", "aws_route_table")?;
    let to = super::associations::endpoint(raw, route, &target.attribute, target_type)?;
    if raw.nodes[from].deposed_key.is_some() || raw.nodes[to].deposed_key.is_some() {
        return None;
    }
    Some((from, to))
}
