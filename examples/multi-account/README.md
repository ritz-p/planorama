# Combined AWS deployment example

This synthetic, offline example combines three independently named Terraform
states and two fabricated AWS accounts. State is not an account boundary:
`application-a` manages both `111111111111` and `222222222222`; the latter spans
`us-east-1` and `us-west-2`. Equal addresses, accounts and regions in other states
retain separate identities.

The source of truth is [generator.rs](generator.rs), which extends the existing
[large Terraform fixture generator](../terraform-large/generator.rs) and its
Terraform JSON sources. Planned IDs/ARNs are clearly fabricated equality evidence,
following that fixture's conventions. No Terraform, AWS credentials, cloud APIs
or network access is needed after Rust dependencies are available. Provider role
metadata uses the verified Terraform nested-expression shape; only the account
identifier survives parsing, not the role ARN or credentials.

## Regeneration and validation

From the repository root with the development container running:

```sh
docker compose exec -T dev cargo run --locked --example generate_multi_fixture
docker compose exec -T dev cargo build --locked --release --bin planorama
docker compose exec -T dev sh scripts/render-multi-account.sh target/release/planorama
docker compose exec -T dev sh scripts/render-multi-account.sh target/release/planorama html
docker compose exec -T dev cargo test --locked --test large_example multi_account_inputs
docker compose exec -T dev cargo test --locked --release --test large_example multi_account_example -- --ignored --nocapture
docker compose exec -T dev cargo test --locked --release --bin planorama benchmarks::multi_account::baseline -- --ignored --nocapture
docker compose exec -T dev python3 tests/support/measure_multi_account.py
docker compose exec -T dev python3 tests/browser/viewer.py
```

[diagram.svg](diagram.svg) and [diagram.html](diagram.html) use identical state
inputs and three explicit remote-state mappings in the rendering script.
Automatic example generation writes back SVG only. Regenerate and commit HTML
explicitly after changes; the opt-in rendering test compares both snapshots.
The input/committed-topology check joins the existing `large_example` integration
target. Expensive complete rendering and layout measurements are explicitly
ignored in ordinary suites and run in release mode with the commands above;
no additional integration target silently joins the standard CI suite.

## Topology and expected display

- `application-a`: main application VPC, four workload subnets, EC2/ECS, ALB and
  target attachments, database, routes and logical definitions. Five additional
  partner VPCs contain 28 distinct subnet/route-table associations. An audit bucket
  uses the shared account; an archive bucket is the sole spatial resource in its
  `us-west-2` region. Both have real reference paths to the application's LB output.
- `application-b`: a smaller copy of the application topology, with update actions
  and a remote dependency on application A's endpoint.
- `shared-services`: shared network/workload topology, exported LB endpoint, AMI
  and IAM role. Its deleted Routes contribute the delete action to the single
  document-level legend, even though this is the last state.

CloudFront creates AWS Global beneath each account. IAM and other logical resources
remain in the annotation area. `legacy_region` has a known account but no region;
`imported_account` has known `us-east-1` but no account. Account IDs are intentionally
visible deployment metadata in SVG/HTML. Peering retains unsupported regional
scope rather than being assigned a region heuristically.

All containment stays within its original state. Remote AMI and IAM references
link logical rows without spatial paths; endpoint consumers produce five outer
gutter paths. The 15 resolved references are distinct from the three mappings.
The repeated `module.network.aws_vpc.main` address has three independent HTML
search results. Dependency, Association, Directed Association, Connection and
Containment appear together, with numbered endpoint badges and embedded Route ports.

## Measured size

| State | Input instances | Retained resources | Spatial cards | Embedded Routes | Logical rows / groups | Indexed relationships / cards | Local paths |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| application-a | 145 | 143 | 96 | 2 | 9 / 8 | 38 / 8 | 77 |
| application-b | 54 | 52 | 33 | 2 | 9 / 8 | 10 / 2 | 46 |
| shared-services | 51 | 49 | 30 | 2 | 9 / 8 | 10 / 2 | 39 |
| Total | 250 | 244 | 159 | 6 | 27 / 24 | 58 / 12 | 162 |

Six AWS metadata data sources are intentionally omitted. Retained resources
include lowered relationship resources, so they do not equal the sum of visible
spatial/embedded/logical cards. There are 328 local semantic relationships, 15
resolved cross-state references, five routed cross-state paths and ten logical
cross-state links. Account/Region/Global panels number 4/6/3. The canvas is
8,932 × 17,134; SVG size is 1,567,103 bytes.
The self-contained HTML is 1,677,243 bytes.

The initial 150–250 instance target is met at its upper end. The primary index has
38 meaningful source resources, close to the initial 40–60 target, rather than
duplicating associations to inflate the count. Separate partner networks preserve
distinct one-to-one subnet/route-table relationships and bound endpoint degree.

## Timing and routing baseline

Measured on 2026-10-13 at renderer revision `f3d862dc38cf06e5b9e699ff6b9c8908bc6a55ee`
plus this example/test change, Rust 1.85.1, release profile, Docker Desktop Linux
x86_64 with 32 available logical processors. One observation per measurement;
other local validation was running, so these are not isolated benchmark medians
or universal CI time limits. Compilation is excluded.

| State | Layout width × height | Layout/routing time | Path length | Bends | Crossings | Shared path length | Visible-border overlap |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| application-a | 8,804 × 6,249 | 107.687 s | 155,690 px | 137 | 157 | 746 px | 0 px |
| application-b | 4,674 × 3,228 | 36.427 s | 55,670 px | 91 | 111 | 689 px | 0 px |
| shared-services | 4,674 × 2,939 | 26.492 s | 38,667 px | 82 | 88 | 24 px | 0 px |

Parsing, semantic transformation, remote resolution and view selection together
took 49.111 ms. Per-state layout times include routing/refinement but exclude SVG
composition and cross-state routing. Geometry is the spatial layout, before
document-level annotation reflow and state offsets. Counts come from the existing
internal `LayoutMetrics` and border-overlap helper; shared path length can include
intentional trunks. Crossings and long offsets remain baseline observations.
Complete three-state SVG generation, including file reads, all layout, annotation
composition and cross-state routing, took 174.599 s in the snapshot check.
HTML generation took 168.734 s; both generated outputs matched their snapshots,
and their spatial path coordinates matched despite HTML withholding Route
destination constants. Chromium's ten offline viewer tests passed, including
three separate selections for the repeated VPC address and zoom/fit behavior.

## Visual review and limitations

The full diagram and enlarged account/region headers, annotation boundaries,
application routing corridors and numbered endpoints were reviewed. Shared width
leaves deliberate whitespace to the right of smaller states. Long cross-account
and cross-state paths and crowded corridors remain visible; this is a reproducible
baseline, not a promise of crossing-free or globally optimal paths.

A more concentrated candidate with 20 extra Routes and ten target attachments
exposed a reserved-port routing panic. Its preserved synthetic input is
[account-high-fanin.json](../../tests/fixtures/routing-stress/account-high-fanin.json),
tracked in [#378](https://github.com/ritz-p/planorama/issues/378). It is excluded from
normal rendering/regeneration. Broader stress testing remains coordinated with
#253 and routing-budget work with #349.
