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

The fixture has 48 resource instances. The renderer produces 30 spatial cards, 2 embedded Route cards,
6 logical-resource cards within one cyan Logical Resources section and 74
relationships: four route-table associations and four target-group attachments
become edges, and two AWS metadata data sources are omitted.

The managed VPC contains four subnet boxes, network policy/route resources, the
target group, ALB, ECS Service, DB subnet group and RDS workload. EC2 workers sit
inside their referenced subnets. ALB/ECS keep individual subnet connections; the
DB subnet group and RDS retain their dependency chain and inferred-placement notes.
Shared VPC and Security Group data sources remain external spatial cards. ECS
Cluster/Task Definition, IAM definitions and the external AMI are logical cards.
A purple Logical Components section contains the load balancer, its Listener and
its uniquely referenced Target Group as action-colored member links. The Target
Group link points to the original spatial card. Relationship Resources follow in a separate neutral
section. Empty sections are omitted in smaller examples.
The 74 relationships remain queryable; 39 paths are spatially routed.
Internet and NAT Routes appear inside the public and private Route Tables with
destination constants and directed lines from their own ports to the Internet
Gateway and NAT Gateway. Matching numbers appear on the children, near the targets
and in the Relationship Resources index; routable children omit the `Target:` row. Canvas dimensions follow the current
routing geometry.
ECS Service lists its Cluster and Task definition; each EC2 worker links its AMI and Instance profile. Connections retain their endpoints and undirected dotted presentation.

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
