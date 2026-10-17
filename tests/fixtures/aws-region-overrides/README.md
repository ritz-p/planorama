# Captured AWS region overrides

`plan.json` was captured using Terraform **1.9.8**, **hashicorp/aws 6.0.0**,
linux_amd64, with the adjacent HCL and dependency lock. Only the top-level timestamp
was removed. VPC, Subnet and CloudWatch Log Group explicitly deploy in `eu-west-3`
while the provider and default VPC use `us-east-1`.

`schema.json` contains the resource schema version and `region` attribute extracted
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

To recapture from the repository root, initialize with network access, then plan
offline (POSIX shell; replace the bind source with the repository's absolute path):

```sh
docker compose exec -T -w /workspace/tests/fixtures/aws-region-overrides dev terraform init -backend=false -input=false -lockfile=readonly
docker compose exec -T -w /workspace/tests/fixtures/aws-region-overrides dev sh -c 'terraform providers schema -json > /workspace/output/region-schema.json'
docker run --rm --network none --mount type=bind,source="$PWD",target=/workspace -w /workspace/tests/fixtures/aws-region-overrides -e AWS_ACCESS_KEY_ID=fixture-only -e AWS_SECRET_ACCESS_KEY=fixture-only -e AWS_EC2_METADATA_DISABLED=true planorama-dev sh -c 'terraform plan -refresh=false -input=false -out=/workspace/output/region.tfplan && terraform show -json /workspace/output/region.tfplan > /workspace/output/region-raw.json'
```

Review the raw JSON before removing its timestamp and formatting it as UTF-8 JSON.
Extract the same four schema entries, retaining each `version` and `block.attributes.region`
(or null when absent). Do not manufacture configuration expressions or plan values.
Regenerate the maintained SVG with `sh scripts/update-examples.sh` inside Docker.
See [the supported evidence and lifecycle boundaries](../../../docs/aws.md#deployment-scopes).
