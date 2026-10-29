# Large AWS architecture fixture

For the combined multi-state/region fixture and account evidence limits, see
[the maintained 250-input example](../multi-account/README.md). This smaller
fixture remains its reusable source and a focused single-state reference.

Current spacing budgets and their CI comparison are documented in
[spacing calibration](spacing-calibration.md). Earlier measurements below retain
their original revision labels.

The JSON-formatted Terraform source is split across `network`, `application`,
`database`, and `shared` modules. Module membership differs from containment:
application EC2 instances belong to subnets in the network module. This is a
layout/debugging fixture, not a production deployment.

## Regeneration

From the repository root with the development container running:

```sh
docker compose exec -T dev cargo run --locked --example generate_large_fixture
docker compose exec -T dev cargo run --locked -- examples/terraform-large/plan.json -o examples/terraform-large/diagram.svg
docker compose exec -T dev cargo test --locked --test large_example
```

The SVG integration test remains required on PRs. Three repeated-layout and
quality unit regressions run after merge and nightly; run them locally with
`docker compose exec -T dev cargo test --locked --all-features --lib deferred_regression_ -- --ignored --test-threads=2 --nocapture`.
See [coverage categories](../../docs/ci.md#deferred-routing-regressions-253).

The committed `plan.json` is synthetic, not captured Terraform output. The Rust
generator reads the adjacent `main.tf.json` files and derives references, module
input/output aliases and native addresses. Managed instances use `create`; data
instances use `no-op`. It needs no AWS credentials, Terraform or provider download.
Listener/Rule action blocks retain nested field references so component inference
can verify target-group paths instead of using flattened action dependencies.
For LB, Listener, Rule and Target Group resources, the generator supplies fabricated
known ARN values in `change.after` and evaluates only exact local ARN traversals
in ownership/action fields. These `000000000000:fixture/` values prove equality
in this offline fixture; they are not captured AWS identifiers or predictions of
real create-plan values. Real plans with unknown ARNs retain ungrouped targets.

Route Tables, Internet Gateways and NAT Gateways similarly receive fabricated
`fixture-id:` IDs. Exact route endpoint traversals receive matching planned IDs,
so the two Route cards have verified owners and next hops. These values are only
offline equality evidence; real unknown endpoint values remain unresolved.

The generator supports single-instance local modules, resource-level literal
`count`, constants and traversal interpolations. It rejects resource `for_each`,
computed counts, function interpolations and module `count` / `for_each` /
`depends_on` / `providers`. Tests compare the generated JSON and SVG with their
committed counterparts.

## Expected display

The Relationship lines legend caps its card width at 960px and places its five
relationship kinds in 2 / 2 / 1 rows below the action/color explanation. Samples retain their circles,
arrows and numbered badges, with the complete explanation beside each sample.
Legend height shifts the annotations and architecture together; it does not
change the routed geometry.

Either `[6]` badge links to the Relationship Resources entry for
`module.network.aws_route.nat`. Its directed endpoint list links that Route card
to `module.network.aws_nat_gateway.main`. The index uses five complete entries
per card, with full addresses wrapped before horizontal packing. In HTML, number
selection emphasizes the entry, both badges, endpoints and corresponding path.

The generated plan has 50 resource changes. After omitting two AWS metadata data
sources, the renderer retains 48 resources. Two Route resources embed in their
route tables and produce directed associations. Route-table associations and the
four target-group attachments retain their cards because this input lacks the
known endpoint values required to prove them. Logical resources share the cyan
Logical Resources section; selected resource counts include embedded resources.

The managed VPC contains four subnet boxes, network policy/route resources, the
target group, ALB, ECS Service, DB subnet group and RDS workload. EC2 workers sit
inside their referenced subnets. ALB/ECS keep individual subnet connections; the
DB subnet group and RDS retain their dependency chain and inferred-placement notes.
Shared VPC and Security Group data sources remain external spatial cards. ECS
Task Definition, IAM definitions, the external AMI, EC2 Launch Template and
Metric Alarm are logical-resource rows. CloudWatch Log Group and ECS Cluster are
independent cards with their own scope evidence. The two managed IAM Roles share one
provider/type/mode group; each resource retains its own identity, action underline
and references. Groups use neutral frames rather than a combined action color.
A purple Logical Components section contains the load balancer, its Listener and
its uniquely referenced Target Group as neutral member links, each with one
action-colored underline below its wrapped text. The Target Group link points to
the original spatial card. Relationship Resources follow in a separate neutral
section. Empty sections are omitted in smaller examples.
The 79 relationships remain queryable; 41 paths are spatially routed.
Internet and NAT Routes appear inside the public and private Route Tables with
destination constants and directed lines from their own ports to the Internet
Gateway and NAT Gateway. Matching standalone numbers appear near both child and
target ports, alongside each child's `Next hop [N]` text and one Relationship
Resources entry; routable children omit the `Target:` row. Canvas dimensions follow
the current routing geometry.
ECS Service lists its Cluster and Task definition; each EC2 worker links its AMI and Instance profile. The Auto Scaling Group links its verified Launch template logical resource. Connections retain their endpoints and undirected dotted presentation.
The synthetic fixture lacks planned Cluster identity values, so membership remains
unresolved; the ordinary Cluster reference and dependency remain visible.

The ECS Log Group reference comes from `tags.logs`, not a logging configuration,
so it is labeled References. The alarm's dimensions reference the database, but
this fixture does not supply matching planned dimension/identifier values. It
therefore shows Monitors: unresolved, emits UnverifiedMonitoring with
`--diagnostics`, and does not claim Monitored by on RDS.

Full Uses/Dependencies labels visibly include
`Task definition: module.application.aws_ecs_task_definition.app` and
`Cluster: module.application.aws_ecs_cluster.app` without truncation. Reference
rows have no count limit. Long headings and references wrap at address-element
boundaries where possible, and their cards reserve the resulting height before
packing and routing.

SG cards remain Policies, but their attachments stay dependencies because this
synthetic fixture has no SG collection ID evidence. The dedicated
[SG example](../security-groups.svg) supplies that collection evidence.

Module names remain card metadata rather than layout bands. The diagram exercises
wrapping, long paths and congestion; tests check geometry and routing quality
without claiming globally optimal layout. See [routing](../../docs/routing.md)
and the smaller [spanning example](../spanning-dense.svg).
Per-edge regression guards allow up to 14 bends and twice the shortest valid
route length plus 128 pixels. The reference uses the same hard boundaries,
ports and marker clearance. Failures identify the offending endpoint addresses.

## Terraform validation

The source was validated with Terraform 1.9.8 and AWS provider 5.100.0; the lock
file is committed. Validation can download the provider but does not need AWS
credentials:

```sh
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large init -backend=false -input=false
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large validate
```

A real plan additionally needs AWS credentials and existing `shared_vpc_id` and
`shared_security_group_id` variables in the selected `region`. Real output depends
on provider results and state, so it will not match this synthetic snapshot.
Keep real plan/state output outside the committed fixture.
