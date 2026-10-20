# Purpose-dependent placement review (#549)

All seven reviewed types retain **Logical** as their default for managed and data
resources. Identity classification from #551 does not select a presentation.
Explicit independent cards are useful for specialized diagrams, but require a
shared generation-time option and preservation of reference details before release.
This review adds a reproducible experiment, not a public option or a new renderer.

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

No default type is promoted in this review. Policy/inline policy, membership,
attachment, instance profile, unsupported Alarm/SSM types and unknown types are
outside the proposed switch. ResourceRole, deployment scope and network containment
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

The Logical variant uses the current default. The independent prototype changes
only the seven types' placement and cached reference-target placement in a cloned
graph before immutable selection and the shared measured layout. It deliberately
does not patch reference visibility: missing rows expose work needed before a
real option can ship. Role, resource identity, action, coverage and relationships
are asserted unchanged; secret sentinels must not appear. The prototype is test-only.

Attached presentation was evaluated against the same examples: none of these
seven types has provider-verified `member_owner` or Route ownership, which the
experiment asserts. Current reference details are the valid associated display:
consumer→Parameter, workload→Role, membership/attachment→User/Group and verified
Alarm↔RDS. Actual Embedded cards are rejected for this scope, so the experiment
does not fabricate owner assignments or misleading attached-card screenshots.
Federation has only configuration dependencies. References, membership, trust,
monitoring and display ownership must not be substituted for one another.

Measured against renderer commit `e884873`, canvases below are in pixels
(Logical → independent). Reference rows count the shared visible reference
projection, not graph-edge count:

| Scenario | Resources, 1 / 4 copies | Canvas, 1 copy | Canvas, 4 copies | Reference rows, 1 copy |
| --- | --- | --- | --- | --- |
| Infrastructure | 8 / 32 | 1188×2452 → 1188×2470 | 1284×5006 → 1284×5648 | 5 → 5 |
| Monitoring | 3 / 12 | 1040×1694 → 1040×1188 | 1040×3308 → 1040×2298 | 3 → 1 |
| IAM | 11 / 44 | 1040×3046 → 1040×2902 | 1040×6484 → 1040×6958 | 13 → 13 |
| Federation | 6 / 24 | 1100×2094 → 1100×1482 | 1100×4712 → 1540×2142 | 2 → 0 |
| Configuration | 3 / 12 | 1040×1536 → 1100×1570 | 1040×2390 → 1100×2340 | 1 → 1 |

Visual review of all five small pairs found Logical type headings useful for
action/mode comparison. Independent IAM cards move principals away from their
membership/attachment rows; at four copies the diagram becomes taller. Standalone
parameters show their region and external/data identity clearly but do not improve
the small overview's size. Independent alarms and federation endpoints are compact,
yet lose the verified monitor labels and explicit reference details respectively.
Their ordinary dependency arrows cannot replace those meanings. Canvas area alone
therefore cannot justify promotion. Four-copy measurements are a bounded growth
comparison, not a performance or readability guarantee for arbitrary plans.

## Generation-time follow-up

Implementation is tracked in [#568](https://github.com/ritz-p/planorama/issues/568).

Use one exact-type allowlist for optional Spatial presentation, initially restricted
to these seven types from `registry.terraform.io/hashicorp/aws`. Proposed surfaces
are repeatable CLI `--spatial-type`, TOML `spatial_types`, library `Options` and
the corresponding versioned Wasm request. These names describe a proposal, not
currently accepted input. Reject unsupported types; foreign provider lookalikes
must remain unaffected. CLI precedence must follow existing configuration rules.
Do not add a separate preset engine or an unsupported option to the #490 UI.

Apply the option once during immutable view projection after normal selection and
context closure, before reference measurement, scope packing and routing. Never
mutate the semantic architecture or change selection because focus was used.
Refresh reference-target presentation consistently, retain verified service rows
and unresolved/unavailable diagnostics across Spatial endpoints, and use the same
result for CLI/library/Wasm SVG and embedded HTML. HTML needs a regeneration
control only after the shared API exists; DOM rearrangement is not a renderer.

Acceptance must cover known/unknown account/region/global scope, multiple states,
managed/data/current/deposed identities, action/coverage, focus/modules,
search/navigation/export and secret non-disclosure. Filtered endpoints stay
Unavailable; equal names, issuer URLs and parameter paths add no edges, ownership
or identity merges. Parameter values, policy bodies, SAML metadata, credentials and
tokens stay excluded. Run browser parity checks, SVG/HTML regeneration and final
Layout quality for the actual presentation implementation.
