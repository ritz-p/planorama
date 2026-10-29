# Purpose-dependent placement (#549, #568)

All seven reviewed types retain **Logical** as their default for managed and data
resources. Identity classification from #551 does not select a presentation.
Explicit independent cards are available through the shared
[generation-time option](configuration.md#explicit-spatial-types) implemented in
#568. The original #549 placement-only prototype lost monitoring and federation
reference rows; the production projection preserves them.

## Decisions by exact AWS type

| Resource type | Default | Optional independent view | Reason and attached-display decision |
| --- | --- | --- | --- |
| `aws_ssm_parameter` | Logical | Configuration-reference diagrams | A stored configuration object can be the subject of a diagram. In an infrastructure overview, individual parameter cards add little beyond the existing consumer reference. A parameter can have several consumers; neither a path nor a reference proves one display owner. Keep reference links, not an embedded copy under each consumer. |
| `aws_cloudwatch_metric_alarm` | Logical | Monitoring diagrams | Alarm identity/action is useful independently of the monitored object. The verified RDS `Monitors` / `Monitored by` pair supports associated reference details, not ownership or monitoring success. Literal-only and unsupported monitoring cases need their unresolved details. Do not attach every alarm to a guessed target. |
| `aws_iam_role` | Logical | IAM/federation diagrams | A principal can be an explicit diagram subject. Infrastructure overviews already link execution/task roles from workloads. A workload reference does not own the role; sharing, data and deposed identities forbid consumer-based merging. |
| `aws_iam_user` | Logical | IAM membership/attachment diagrams | User identity and direct membership matter in an IAM view. A user can belong to several groups; membership does not justify embedding the user under one group. |
| `aws_iam_group` | Logical | IAM membership/attachment diagrams | An identity collection can be a useful independent endpoint. It is not an assumable principal, a network container or an automatic display owner for its users/policies. |
| `aws_iam_openid_connect_provider` | Logical | Federation-configuration diagrams | The federation setting is useful as a named endpoint. Explicit Terraform references remain dependencies, not proven trust. Neither an EKS issuer match nor a role reference establishes display ownership. |
| `aws_iam_saml_provider` | Logical | Federation-configuration diagrams | The federation setting is distinct from a user/principal. A provider may be referenced by several roles; neither a name match nor SAML content establishes ownership or authentication success. |

No default type is promoted. Policy/inline policy, membership,
attachment, instance profile, unsupported Alarm/SSM types and unknown types are
outside the allowlist. ResourceRole, deployment scope and network containment
remain independent of that switch.

## Reproducible comparison

Run inside the development container:

```sh
cargo test --locked --lib view::tests::purpose_placement::compare_purpose_placements -- --ignored --exact
```

The [experiment](../tests/unit/purpose_placement.rs) writes 20 SVGs and
`output/purpose-placement/measurements.md`. It reuses the maintained
[infrastructure](../tests/fixtures/viewer-plan.json),
[IAM](../tests/fixtures/iam-plan.json),
[federation](../tests/fixtures/identity-providers-plan.json) and
[configuration](../tests/fixtures/ssm-plan.json) examples, plus an inline synthetic
RDS/alarm pair and literal-only alarm. Each is rendered once and with four
address-distinct copies. Copies preserve configuration references and action/mode;
they are a count-growth probe within one state, not evidence of cross-state identity.

The Logical variant uses the current default. The independent variant uses
`view::Options::spatial_types` after immutable selection and context closure,
before shared reference measurement/layout. It refreshes target placement and
preserves visible references without changing their verification status.
Role, resource identity, action, coverage, relationships and reference counts
are asserted unchanged; secret sentinels must not appear.

Attached presentation was evaluated against the same examples: none of these
seven types has provider-verified `member_owner` or Route ownership, which the
experiment asserts. Current reference details are the valid associated display:
consumer→Parameter, workload→Role, membership/attachment→User/Group and verified
Alarm↔RDS. Actual Embedded cards are rejected for this scope, so the experiment
does not fabricate owner assignments or misleading attached-card screenshots.
Federation has only configuration dependencies. References, membership, trust,
monitoring and display ownership must not be substituted for one another.

## Generation-time contract

The exact allowlist applies to HashiCorp AWS, including the existing inferred AWS
identity when provider source is absent. Explicit foreign providers are excluded.
CLI `--spatial-type`, TOML `spatial-types`, library `Options::spatial_types` and
Wasm `spatial_types` use one projection; the browser form exposes the same types
as optional checkboxes in Generation settings.

Filtered endpoints stay Unavailable. Equal names, issuer URLs and parameter paths
add no edges, ownership or identity merges. Parameter values, policy bodies, SAML
metadata, credentials and tokens stay excluded. Configuration dependencies,
membership/attachment evidence and verified monitoring retain distinct semantics.
Normal tests cover immutable selection, exact provider/type matching, multiple
states and managed/data/current/deposed identities. Browser tests cover native/Wasm
parity, search, reference navigation and neighborhood/viewport exports.
