# Large AWS architecture fixture

The four Terraform modules in [main.tf.json](main.tf.json) provide the source
for the native AWS Emulator capture. The committed [plan.json](plan.json) is
a separate synthetic routing fixture. It also supplies the
[multi-account generator](../multi-account/README.md).

## Regeneration

From the repository root in the [prepared development environment](../../CONTRIBUTING.md#terraform-samples-with-aws-emulator), run:

```sh
sh scripts/aws-emulator/prepare.sh large
sh scripts/aws-emulator/generate.sh large
```

The printed directory contains the native saved plan, its JSON, SVG and HTML,
and a separate `stress.json`. The emulator creates a seed VPC and security group,
uses their IDs to plan the application and destroys the seed. Keep saved plans
and state in the private output directory.

Review `stress.json` before copying it to `examples/terraform-large/plan.json`.
Run `sh scripts/update-examples.sh` to refresh the maintained diagrams, then
`cargo test --locked --test large_example` to check the large and multi-account
fixtures. The [capture checks](../../CONTRIBUTING.md#capture-checks-and-api-coverage)
compare the projection with the committed input.

## Expected display

The [stress projection](../../scripts/aws-emulator/stress-base.cjs) accepts the
fixture's create actions, local unkeyed modules and read data sources. It adds
fabricated ARN and ID values to exercise equality checks for load-balancer
listeners, target groups, route owners and next hops. The native create plan
retains Terraform's unknowns. The [diagram](diagram.svg) shows the resulting
containment, components and routes. The Log Group reference in `tags.logs`
supplies a reference only, and the alarm lacks the matching database identifier
needed to verify monitoring.

See [routing verification](../../docs/routing.md#verification) for the quality
criteria and [deferred routing regressions](../../docs/ci.md#deferred-routing-regressions-253)
for the local command and CI schedule.

## Terraform validation

The source was validated with Terraform 1.9.8 and AWS provider 5.100.0; the lock
file is committed. Validation can download the provider but does not need AWS
credentials:

```sh
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large init -backend=false -input=false
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large validate
```
