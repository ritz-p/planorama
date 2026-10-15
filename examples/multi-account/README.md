# Combined AWS deployment example

This synthetic offline fixture combines 250 inputs across three named Terraform
states. Its provider expressions contain two fabricated account targets, but it
has no real saved-plan archive. Both rendered examples therefore leave every
account unknown. Visible role ARNs in JSON cannot prove that Terraform did not
omit another dynamic role; see [provider evidence](../../docs/provider-evidence.md).

The source of truth is [generator.rs](generator.rs), which extends the existing
[large fixture generator](../terraform-large/generator.rs). Planned IDs and ARNs
are fabricated equality evidence. No AWS credentials, cloud APIs or Terraform are
needed to regenerate this fixture after Rust dependencies are available.

## Regeneration and validation

```sh
docker compose exec -T dev cargo run --locked --example generate_multi_fixture
docker compose exec -T dev cargo build --locked --release --bin planorama
docker compose exec -T dev sh scripts/render-multi-account.sh target/release/planorama
docker compose exec -T dev sh scripts/render-multi-account.sh target/release/planorama html
docker compose exec -T dev cargo test --locked --test large_example multi_account_inputs
docker compose exec -T dev cargo test --locked --release --test large_example multi_account_example -- --ignored --nocapture
docker compose exec -T dev python3 tests/support/measure_multi_account.py
docker compose exec -T dev python3 tests/browser/viewer.py
```

[diagram.svg](diagram.svg) and [diagram.html](diagram.html) use identical inputs
and three explicit remote-state mappings. The manual generation workflow updates SVG only;
regenerate and commit HTML explicitly. The expensive complete rendering test is
opt-in and checks both snapshots. The ordinary integration test verifies source
generation and committed topology.

## Topology and evidence limits

- `application-a` has the main application VPC, four workload subnets, EC2/ECS,
  ALB, database, routes, logical definitions and five partner VPCs with 28 distinct
  subnet/route-table associations. The archive bucket occupies `us-west-2`.
- `application-b` is a smaller application topology with update actions and a
  remote dependency on application A's endpoint.
- `shared-services` exports an LB endpoint, AMI and IAM role. Deleted Routes
  contribute to the shared action legend.

Known regions and AWS Global still form panels within each state. IAM and other
logical resources stay in the annotation area. `legacy_region` has neither known
region nor account; `imported_account` keeps its known `us-east-1` region. Unknown
accounts produce no account frames or labels. Equal addresses and regions across
states remain separate, including three independent search results for
`module.network.aws_vpc.main`.

The 15 resolved remote references are distinct from the three explicit mappings.
AMI and IAM references link logical rows; endpoint consumers produce routed
cross-state paths. Dependency, Association, Directed Association, Connection and
Containment are all represented, with numbered endpoint badges and embedded Routes.

For real multi-account plans, add `--state-provider-plan ID=PATH` for each named
state with a matching supported saved plan. Account evidence is never inferred
from this fixture's fabricated provider aliases, roles or state names. Verified
snapshot tests retain account grouping coverage; `tests/support/verify_provider_plan.sh`
validates the actual saved-plan CLI using a local STS fixture.

## Measurements and limitations

The previous account-panel layout and timing baseline used JSON-only account
inference and no longer describes supported output. Use the measurement script
above for current geometry, path and byte counts. For optional timings, run
`benchmarks::multi_account::baseline` with `--release -- --ignored --nocapture`.
Layout costs depend on the machine; no timing threshold is imposed. Long paths
and crowded corridors remain possible.

The concentrated [routing stress fixture](../../tests/fixtures/routing-stress/account-high-fanin.json)
is excluded from routine rendering because of its cost. It originally exposed
the reserved-port routing panic fixed in #378. Spacing calibration remains in
[spacing calibration](../terraform-large/spacing-calibration.md).
