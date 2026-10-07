# Large AWS architecture fixture

This is a layout/debugging example, not a production deployment. The JSON-formatted Terraform source is split across `network`, `application`, `database`, and `shared` modules. Module membership intentionally differs from containment: application EC2 instances belong to subnets in the network module.

The topology includes four subnets in two availability zones, Internet/NAT gateways, public/private routing and four route-table associations, an interface endpoint, VPC peering, an ALB with a target group/listener/attachments, four counted EC2 workers, an ECS service, RDS, IAM roles, security groups, and existing-infrastructure data lookups. Multi-subnet resources remain relationships; neither the ALB nor the target group is treated as a physical container. Explicit network containment also covers NAT gateways in subnets and VPC endpoints, route tables, security groups, and network ACLs in VPCs.

## Offline regeneration

From the repository root:

```sh
docker compose run --rm -T dev cargo run --locked --example generate_large_fixture
docker compose run --rm -T dev cargo run --locked -- examples/terraform-large/plan.json -o examples/terraform-large/diagram.svg
docker compose run --rm -T dev cargo test --locked --test large_example
```

No AWS account, credentials, Terraform installation, or provider download is needed for these commands. The committed `plan.json` is a **synthetic, representative plan fixture**, not output captured from `terraform show`. The Rust generator reads the actual `main.tf.json` files and derives configuration references, module input/output aliases, and native addresses. Managed instances use `create`; data instances use `no-op`. There are no resolved attribute values, state, secrets, real IDs, or provider execution results in the fixture.

The generator deliberately supports only this example's subset: single-instance local modules, resource-level literal `count`, constants, and `${traversal}` interpolations. It is not a Terraform evaluator. It rejects resource-level `for_each`, computed counts, and function interpolations rather than inventing their results. Module-level `count` (including literal counts), `for_each`, `depends_on`, and `providers` are also rejected, including in nested modules, instead of treating these meta-arguments as inputs. Regression tests compare regenerated fixture bytes with the committed input and rendered SVG with the committed output.

## Terraform validation and real plans

The `.tf.json` files are Terraform source, not a second hand-maintained topology. Terraform 1.9+ and the AWS 5.x provider can validate them:

Validation was run with Terraform 1.9.8 and AWS provider 5.100.0; the provider lock file is committed. The fixture itself has 48 resource instances. The current renderer produces 42 cards and 77 relationships after lowering four associations, omitting two metadata data sources, and inferring three common-parent containment relationships. RDS inherits the VPC scope through its DB subnet group, which remains visible with its dependencies.

Balanced wrapping preserves containment while reducing vertical growth; the committed SVG records the current dimensions after network-scope inference.

The layout regression compares occupancy-aware routing with shortest paths on the current geometry, requiring a substantial overlap reduction and a lower combined overlap/crossing/bend/length cost.

```sh
terraform -chdir=examples/terraform-large init -backend=false
terraform -chdir=examples/terraform-large validate
```

The same validation can run in the existing Docker environment:

```sh
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large init -backend=false -input=false
docker compose run --rm -T --entrypoint terraform terraform-example -chdir=examples/terraform-large validate
```

A real plan additionally needs AWS credentials and existing VPC/security-group IDs in the selected region. Provide `shared_vpc_id`, `shared_security_group_id`, and optionally `region` as variables, then use `terraform plan -out=...` and `terraform show -json ...`. Keep real plan/state output outside the committed fixture. AWS AMI lookup results, account data, provider versions, and state make real plans environment-dependent; they will not match the synthetic snapshot. This example is intended for planning/visualization and does not provision a working application or production IAM/network policy.

## Expected display

The managed VPC contains four subnet boxes; private subnet boxes contain the application module's EC2 workers. The ALB and ECS Service are also contained by the common VPC, with separate connections to each referenced subnet. Route-table associations become four action-carrying edges. Shared VPC, Security Group, and AMI data sources remain external entities; region/account metadata data sources are omitted. Security Groups and ECS Services retain the Policy/Controller fallback annotations. Module names remain card metadata; architecture layout has no module bands.

The generator preserves the ECS `network_configuration` block's individual expressions so subnet membership can be distinguished from Security Group references, following Terraform's [block expressions representation](https://developer.hashicorp.com/terraform/internals/json-format#block-expressions-representation).

The larger diagram intentionally exposes long paths, fan-in/fan-out and routing congestion. Overlapping edge segments and a tall layout remain possible; the fixture does not assert that this renderer achieves a globally optimal layout.

The dedicated [three-subnet ALB fixture](../spanning-dense.svg) isolates its benefit: total connection length falls from 1852 to 478 pixels, crossings fall from 2 to 0, and bends fall from 10 to 6. Shared-trunk overlap is intentional; every connection remains individually inspectable.
