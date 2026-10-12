# Large AWS architecture fixture

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

The fixture has 48 resource instances. The renderer produces 28 spatial cards, 2 embedded Route cards,
8 logical-resource cards within one cyan Logical Resources section and 74
relationships: four route-table associations and four target-group attachments
become edges, and two AWS metadata data sources are omitted.

The managed VPC contains four subnet boxes, network policy/route resources, the
target group, ALB, ECS Service, DB subnet group and RDS workload. EC2 workers sit
inside their referenced subnets. ALB/ECS keep individual subnet connections; the
DB subnet group and RDS retain their dependency chain and inferred-placement notes.
Shared VPC and Security Group data sources remain external spatial cards. ECS
Cluster/Task Definition, IAM definitions, the external AMI, CloudWatch Log Group
and Metric Alarm are logical cards.
A purple Logical Components section contains the load balancer, its Listener and
its uniquely referenced Target Group as action-colored member links. The Target
Group link points to the original spatial card. Relationship Resources follow in a separate neutral
section. Empty sections are omitted in smaller examples.
The 74 relationships remain queryable; 37 paths are spatially routed.
Internet and NAT Routes appear inside the public and private Route Tables with
destination constants and directed lines from their own ports to the Internet
Gateway and NAT Gateway. Matching numbers appear on the children, near the targets
and in the Relationship Resources index; routable children omit the `Target:` row. Canvas dimensions follow the current
routing geometry.
ECS Service lists its Cluster and Task definition; each EC2 worker links its AMI and Instance profile. Connections retain their endpoints and undirected dotted presentation.

The ECS Log Group reference comes from `tags.logs`, not a logging configuration,
so it is labeled References. The alarm's dimensions reference the database, but
this fixture does not supply matching planned dimension/identifier values. It
therefore shows Monitors: unresolved, emits UnverifiedMonitoring with
`--diagnostics`, and does not claim Monitored by on RDS.
Moving these two CloudWatch cards reduced routed paths from 39 to 37 (5.1%).
At that step the spatial card envelope stayed 3416 × 2062 (7,043,792 square pixels),
and the canvas stayed 3556 × 3392: other resources determined the outer bounds.

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

Listener owner affinity reduces the architecture-local Manhattan center distance
to its LB from 3,608 to 1,060 pixels without changing the 10,032,636-pixel layout
area or the 37 routed relationships. Total path length changes from 44,380 to
39,914 pixels, bends from 89 to 87 and crossings from 119 to 99; measured overlap
stays 737 pixels. Docker test-profile layout runs measured 6.28 versus 6.68 seconds
on the same machine (one sample, not a timing assertion). The Listener remains
outside the VPC because ownership is a placement preference, not containment.

The subsequent single-neighbor preference keeps that owner distance and layout
area, while reducing total route length from 39,914 to 34,189 pixels, bends from
87 to 79 and crossings from 99 to 62. Overlap remains 737 pixels. The controlled
two-leaf regression also records card gaps (1,484/1,284 → 40/176 pixels), path
length (5,675 → 1,457), bends (10 → 8), and unchanged area (3,135,600 pixels).

## Parallel route clearance

The revised spacing pass plans complete lanes before changing routes. It reserves
missing corridor capacity, reroutes impacted independent edges, then compares
final per-edge shape and proximity. Six corridor attempts and 64px growth per
axis bound the work. Whole-route clarity takes priority over the soft 16px target.
Failed expansion and routing restore geometry and paths together.

The metric ignores the first/last 32px at real endpoints, runs shorter than 64px,
perpendicular crossings and intentional shared trunks. Instance Profile is now
logical and its worker references are linked text; focused mixed-edge fixtures
preserve horizontal/vertical independent-routing coverage.

| Metric | Before #291 | Initial PR | Revised |
| --- | ---: | ---: | ---: |
| Layout width × height | 4196 × 2391 | 4262 × 2391 | 4208 × 2391 |
| Total path length | 34189 | 35016 | 34331 |
| Bends | 79 | 79 | 77 |
| Crossings | 62 | 65 | 64 |
| Exact overlap length | 737 | 745 | 737 |
| Too-close projected length | 5678 | 671 | 3706 |
| Minimum effective parallel clearance | 1 | 6 | 6 |
| Docker test-profile layout time, two runs | 5.631 / 5.614 s | 5.898 / 5.707 s | 7.219 / 7.036 s |

Layout dimensions exclude SVG annotations. At the #291 revision, the final SVG
was 4208 × 3481 pixels; later annotation changes, including the measured
Relationship lines card, can change the canvas without changing these layout
measurements.
The revision accepts more residual proximity than the initial PR in exchange for
shorter, simpler paths and less growth. It still removes 34.7% of the original
close projected length. Runtime adds about 1.3 seconds over the initial PR on this
machine; times are observations, not CI assertions. This is a bounded routing
tradeoff, not a claim that all lanes reach 16px or that aggregate bends prove
visual quality.

The [37-edge comparison](routing-comparison.md) records lengths, bends, short
offsets, reversals/backtracking, S-jogs and stretch before/after the refinement.
Private-b subnet → DB subnet group drops from four bends to two. ECS and VPC
endpoint subnet connectors retain simple two-bend paths. Private Route Table
associations retain only the turns required by fixed ports, numbered-marker
clearance and the neighboring subnet container. Full and enlarged SVG inspection
covers these corridors. Geometry, shape and metrics agree after edge reordering.

The focused narrow corridor carries Dependency, numbered Association and
Connection edges. Three lanes require a 44px gap expansion; final rerouting
produces ordered two-bend routes at 16px separation, distinct from translated old
paths. The transposed fixture covers horizontal routing. Seven lanes exceed the
growth budget and keep clean two-bend paths with residual proximity. Separate
checks protect shared trunks and verify rollback of paths, junctions, scopes and
bands when expanded routing cannot be accepted.

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
