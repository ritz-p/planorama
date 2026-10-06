# Captured Terraform AWS plan

`plan.json` was captured with **Terraform 1.9.8**, **hashicorp/aws 5.100.0**, linux_amd64, from the adjacent HCL using real `terraform plan -out` and `terraform show -json`. It is additional to the synthetic large fixture, which remains unchanged.

The capture ran in a container with `--network none`, an empty local state, and only dummy environment credentials (`fixture-only`). No AWS account, API, existing infrastructure, or apply was used. The provider skips credential/account/metadata checks. The IAM policy document data source is computed locally. All names and CIDRs are disposable fixture constants. The provider-generated policy document ID is a local checksum, not an AWS resource ID.

Coverage: a child module, module input/output, locals, counted subnets and associations, VPC, ALB networking, and a data source. Terraform does not serialize local definitions in this configuration JSON; dynamic `count.index` subnet associations remain conservatively unresolved by Planorama. The regression intentionally preserves these real expression shapes instead of replacing them with synthetic references.

## Capture (PowerShell, repository root)

Docker and the pinned image/provider are needed only for capture. Normal `cargo test` uses the committed JSON and requires neither Terraform nor credentials.

```powershell
# Initialization can download the pinned provider; it needs no AWS credentials.
docker run --rm -v "${PWD}/tests/fixtures/aws-captured:/capture" -w /capture hashicorp/terraform:1.9.8 init -backend=false -input=false
# No host AWS configuration or environment credentials are forwarded.
docker run --rm --network none -e AWS_ACCESS_KEY_ID=fixture-only -e AWS_SECRET_ACCESS_KEY=fixture-only -e AWS_EC2_METADATA_DISABLED=true -v "${PWD}/tests/fixtures/aws-captured:/capture" -w /capture --entrypoint /bin/sh hashicorp/terraform:1.9.8 -c 'terraform plan -refresh=false -input=false -out=capture.tfplan && terraform show -json capture.tfplan > raw.json'
./tests/fixtures/aws-captured/sanitize.ps1
cargo test --locked --test terraform_plan
```

For the original capture, initialization also ran offline, mounting the existing `examples/terraform-large/.terraform/providers` cache at `/plugins` and using `init -plugin-dir=/plugins`. That mount must also be present during plan/show because Terraform links to the cached provider. The lock file records the linux_amd64 provider checksum.

## Review and normalization

The full JSON was reviewed before commit: no actual credentials, account IDs, private hostnames, production IDs, or sensitive attribute values are present. Dummy environment credentials are not serialized. Managed resource identifiers remain unknown (`after_unknown`), and all known values originate from this public fixture configuration or provider defaults. `sanitize.ps1` removes only the volatile top-level timestamp and pretty-prints with LF/UTF-8; it does not manufacture resources or expressions. It is not a sanitizer for arbitrary real-world plans. The ignored binary plan and raw JSON are not committed.

Stable expectations: 8 parsed entities (7 managed creates and one locally computed data source), 7 visible cards, 12 relationships, 3 containment relationships, and 2 subnet-to-ALB connections. Repeated rendering must be byte-identical.
