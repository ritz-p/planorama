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

The Relationship lines legend uses the wide canvas to place its five relationship
kinds on one row below the action/color explanation. Samples retain their circles,
arrows and numbered badges, with the complete explanation beside each sample.
Legend height shifts the annotations and architecture together; it does not
change the routed geometry.

The generated plan has 50 resource changes. After omitting two AWS metadata data
sources, the renderer retains 48 resources and produces 29 spatial cards, 2 embedded
Route cards, 9 logical resources in 8 group cards within one cyan Logical Resources
section and 79 relationships. Four route-table associations and four target-group
attachments become edges.

The managed VPC contains four subnet boxes, network policy/route resources, the
target group, ALB, ECS Service, DB subnet group and RDS workload. EC2 workers sit
inside their referenced subnets. ALB/ECS keep individual subnet connections; the
DB subnet group and RDS retain their dependency chain and inferred-placement notes.
Shared VPC and Security Group data sources remain external spatial cards. ECS
Cluster/Task Definition, IAM definitions, the external AMI, EC2 Launch Template,
CloudWatch Log Group and Metric Alarm are logical-resource rows. The two managed IAM Roles share one
provider/type/mode group; each resource retains its own identity, action underline
and references. Groups use neutral frames rather than a combined action color.
A purple Logical Components section contains the load balancer, its Listener and
its uniquely referenced Target Group as neutral member links, each with one
action-colored underline below its wrapped text. The Target Group link points to
the original spatial card. Relationship Resources follow in a separate neutral
section. Empty sections are omitted in smaller examples.
The 79 relationships remain queryable; 39 paths are spatially routed.
Internet and NAT Routes appear inside the public and private Route Tables with
destination constants and directed lines from their own ports to the Internet
Gateway and NAT Gateway. Matching standalone numbers appear near both child and
target ports, alongside each child's `Next hop [N]` text and one Relationship
Resources entry; routable children omit the `Target:` row. Canvas dimensions follow
the current routing geometry.
ECS Service lists its Cluster and Task definition; each EC2 worker links its AMI and Instance profile. The Auto Scaling Group links its verified Launch template logical resource. Connections retain their endpoints and undirected dotted presentation.

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

These are historical #291 measurements. Layout dimensions exclude SVG annotations;
the SVG at that revision was 4208 × 3481 pixels. Later annotation changes,
including logical-resource grouping and the measured Relationship lines card,
can alter the canvas independently of the spatial layout.
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
associations were compared with the fixed ports, former graph-wide marker
clearance and neighboring subnet container at that revision. #322 changes the
terminal geometry and side selection; these historical numbers are not current
SVG measurements. Full and enlarged SVG inspection covered these corridors, and
geometry, shape and metrics agreed after edge reordering.

The focused narrow corridor carries Dependency, numbered Association and
Connection edges. Three lanes require a 44px gap expansion; final rerouting
produces ordered two-bend routes at 16px separation, distinct from translated old
paths. The transposed fixture covers horizontal routing. Seven lanes exceed the
growth budget and keep clean two-bend paths with residual proximity. Separate
checks protect shared trunks and verify rollback of paths, junctions, scopes and
bands when expanded routing cannot be accepted.

## Visible border clearance

#317 adds a separate 4px minimum and 8px preferred empty gap between visible
relationship and frame strokes. The final pass shares its six-attempt and 64px
per-axis budgets with parallel-line spacing. It can act on one path and short
contacts, using actual card/container/Route border widths and rounded geometry.
Covered path pieces remain masked; visible exterior paths retain their border
cost. Minimum-gap regressions, extra bends, S-jogs and excessive detours reject a
spacing change, with complete geometry rollback when necessary.

The focused single-path fixture uses a 12px gap between cards. An 8px expansion
creates a 20px corridor and preserves a simple two-bend path for Dependency,
Association and Connection strokes. Transposition and 32px/300px frame spans
exercise both axes and short contacts. A protected junction prevents rerouting
and leaves the original geometry and measurable residual deficit intact. These
checks are separate from the historical #291 measurements above.

## #317 calibration

This comparison uses the [#322 baseline SVG at e4d50682](https://github.com/ritz-p/planorama/blob/e4d506821aaebd41abb42fa2380ef841733604d3/examples/terraform-large/diagram.svg)
and the [#317 SVG at fc87e583](https://github.com/ritz-p/planorama/blob/fc87e583bc726db56aebf74673c3f3929e5b3f9d/examples/terraform-large/diagram.svg),
generated from [source 3f920401](https://github.com/ritz-p/planorama/commit/3f920401c5fd287b0aeabe39d5b22029853827b4)
by [CI run 38225323594](https://github.com/ritz-p/planorama/actions/runs/38225323594).
Both compared revisions had 37 rendered paths, a 4196 × 2391 layout and a
4196 × 3481 SVG canvas. These measurements predate the review fixes. At
review-fix commit `91159e7`, the SVG measured 4246 × 3481; the additional 50px
width was within the 64px expansion budget. These historical canvas dimensions
do not include the measured Relationship lines card introduced by #316.
The existing large-fixture clearance, shape, canvas and reordered-edge
regressions still apply.

The read-only SVG audit recovers orthogonal corners from `M`/`L`/`Q` commands and
applies the same new border metric to both revisions: actual stroke widths,
rounded frames, masks, card covers and conservative quarter-pixel gaps with bounded
curve sampling. Line spacing retains its 32px endpoint allowance, 64px sustained
projection and shared-trunk exclusions. Raw overlap includes intentional bundles.
Border deficits below are quarter-pixel gap deficits multiplied by pixel path
length; a negative gap denotes intersecting stroke envelopes.

| Large-fixture metric | #322 baseline | #317 |
| --- | ---: | ---: |
| Total path length, px | 34072 | 34055 |
| Bends | 72 | 77 |
| Internal orthogonal crossings | 70 | 76 |
| Raw overlap length, px | 745 | 737 |
| Too-close line projection, px | 3766 | 3234 |
| Line-spacing deficit, px² | 25736 | 24190 |
| Minimum sustained line separation, px | 4 | 4 |
| Minimum visible border gap, px | -2 | 4.5 |
| Too-close border projection, px | 5247 | 28 |
| Minimum border deficit, quarter-pixel × px | 7242 | 0 |
| Preferred border deficit, quarter-pixel × px | 54115 | 182 |

Border clearance improves with five additional bends and six additional crossings.
Across those two revisions, total length fell by 17px and canvas size stayed fixed. Offsets remain two and
S-jogs remain zero; reversals rise from two to three and backtracking from 204px
to 227px. These compare complete layouts; the final spacing guard constrains each
candidate against that run's pre-spacing routes. Only four routes change these
shape measurements:

| Route | Baseline length / bends | #317 length / bends | Border change |
| --- | ---: | ---: | --- |
| LB → Listener | 757px / 2 | 772px / 4 | Minimum deficit 120 → 0 |
| IGW → NAT | 1126px / 1 | 1126px / 3 | Minimum deficit 4443 → 0 |
| Public-a subnet → Public Route Table | 522px / 1 | 474px / 2 | Minimum deficit 104 → 0 |
| Shared VPC → Peering | 4007px / 3 | 4023px / 3 | Preferred close projection 3520px → 1px |

The Listener route adds one reversal and 15px of backtracking; Shared VPC's
backtracking rises from 188px to 196px. The private-b subnet → private Route Table
path retains 709px and two bends without offsets, reversals or backtracking.
The four target attachments retain their lengths and two/three-bend shapes.
The largest remaining preferred border deficit is ALB SG → application SG beside
the public-a corner: 16px of projection at a 4.5px gap, contributing 135 of 182.
The remaining 12px of close border projection comes from short rounded contacts;
every measured border gap meets 4px. All 18 numbered badges retain their identities,
with no badge overlap or foreign-stroke intrusion found in the audit.

The separate dense unit fixture also exposes a tradeoff. Its
[#322 baseline](https://github.com/ritz-p/planorama/blob/e4d506821aaebd41abb42fa2380ef841733604d3/tests/unit/layout.rs)
and measurements after the review fixes are:

| Dense-fixture metric | #322 baseline | #317 |
| --- | ---: | ---: |
| Overlap length, px | 168 | 1 |
| Crossings | 373 | 394 |
| Bends | 88 | 87 |
| Total path length, px | 71553 | 71184 |

The review fixes raise crossings from 392 to 394 while retaining overlap, bends
and length. Docker tests preserve exact repeated-layout and reversed-edge geometry
assertions alongside the updated metric expectation. These results document clearance,
overlap and length gains alongside increased crossings and some more complex
routes; the finite routing policy does not minimize every metric simultaneously.

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
