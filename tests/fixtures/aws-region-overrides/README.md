# Captured AWS region overrides

[The capture](../compatibility/1.9.8/overrides/plan.json) was generated using Terraform **1.9.8**, **hashicorp/aws 6.0.0**,
linux_amd64, from the adjacent HCL and the lock in the capture directory. Only the top-level timestamp
was removed. VPC, Subnet and CloudWatch Log Group explicitly deploy in `eu-west-3`
while the provider and default VPC use `us-east-1`.

[The captured schema](../compatibility/1.9.8/overrides/schema.json) contains the resource schema version and `region` attribute extracted
from `terraform providers schema -json` for the three supported managed types and
the global IAM Role negative control. All three overrides are optional/computed
strings. IAM Role has no such attribute. Their deployment meaning is documented in
the pinned upstream [VPC](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/vpc.html.markdown),
[Subnet](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/subnet.html.markdown)
and [Log Group](https://github.com/hashicorp/terraform-provider-aws/blob/v6.0.0/website/docs/r/cloudwatch_log_group.html.markdown)
documentation. This contract must not be applied to the existing AWS 5.100.0 fixture.

The capture used an empty local state, disabled container networking and dummy
`fixture-only` credentials. No AWS API, account, infrastructure or apply was used.
The reviewed JSON contains public fixture constants, provider defaults and unknown
resource identifiers. It contains no credentials or production data. Account scope
stays unknown. Normal tests use committed JSON without Terraform or AWS credentials.

Use the [compatibility capture procedure](../../../docs/terraform-compatibility.md#reproduction)
with case `overrides` to regenerate the plan, schema and provenance record.
Regenerate the maintained SVG with `sh scripts/update-examples.sh`.
See [the supported evidence and lifecycle boundaries](../../../docs/aws.md#deployment-scopes).
