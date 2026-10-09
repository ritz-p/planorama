# Route relationships

Statically resolved managed AWS `aws_route` resources become individual `Association` edges from their Route Table to their target. The initial supported pairs are `gateway_id` → `aws_internet_gateway`, `nat_gateway_id` → `aws_nat_gateway`, `transit_gateway_id` → `aws_ec2_transit_gateway`, and `vpc_peering_connection_id` → `aws_vpc_peering_connection`.

Both endpoints must resolve uniquely through complete ID references to the expected AWS resource types. Exactly one target attribute may be present. Additional target ID/ARN attributes, literal IDs, dynamic selections, unresolved references, deposed objects, unsupported target families, or extra incident relationships keep the route card. VPC Endpoint routes remain cards until endpoint subtype semantics can be established safely. CIDRs and other attribute values are not retained or used to infer endpoints.

Each edge retains the route's own Terraform address, action, previous address and operation metadata. Parallel routes between identical endpoints remain distinct. Route destinations are intentionally not displayed; inspect the original plan for routing configuration. See [the routing regression example](../examples/routes.svg) and the [AWS Provider route reference](https://github.com/hashicorp/terraform-provider-aws/blob/main/website/docs/r/route.html.markdown).
