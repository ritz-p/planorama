# At-rest encryption in plan cards

S3 buckets, SQS queues and RDS instances/clusters show **At-rest encryption (plan)**
with `enabled`, `disabled` or `unknown`. These are Terraform plan settings, not an
assessment of deployed security, access permissions or transport encryption.

The initial field contract was checked against HashiCorp AWS Provider **6.70.0**,
release commit `5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, using the schema captured
by the [inventory procedure](aws-inventory.md#reproduce-and-update). Its SHA-256 is
`f04a44109a82886c3e392ae88e73dfa533f677ccd8a1b253d7d7b0e3d3ea9857`.
The pinned official resource documentation is linked from the
[support matrix](resource-support.md). Existing verified encryption-key reference
rules retain their [separate evidence contract](aws.md).

| Resource / schema field | Decision |
| --- | --- |
| RDS instance/cluster `storage_encrypted` (bool) | Explicit `true`: enabled; explicit `false`: disabled. A key alone does not establish storage encryption. |
| SQS `sqs_managed_sse_enabled` (bool) | Explicit `true`: enabled, SSE-SQS, SQS-managed key. Conflicting nonempty KMS configuration: unknown. |
| SQS `kms_master_key_id` (string) | Known nonempty value: enabled, AWS KMS; key ownership is not inferred from the string. |
| SQS explicit SSE `false` and known null/empty KMS field | Disabled. A missing, unknown or sensitive KMS field leaves the result unknown. |
| S3 `rule.apply_server_side_encryption_by_default.sse_algorithm` (string) | `AES256`: enabled, SSE-S3, S3-managed key; `aws:kms`: enabled, AWS KMS; `aws:kms:dsse`: enabled, DSSE-KMS. Other/missing/ambiguous rules: unknown. |
| Legacy S3 bucket `server_side_encryption_configuration` (deprecated list block) | The same single-rule algorithm decision; empty blocks do not establish disabled encryption. |

Unknown and sensitive masks are checked at every ancestor of the relevant field.
Absent fields never acquire cloud defaults. Only fixed summary labels enter the
graph: raw attribute values, literal key IDs/ARNs, secrets and key material are not
added to SVG/HTML. Method and key ownership labels appear only when established by
the corresponding setting. Missing key references never imply disabled encryption.

The separate S3 encryption configuration is embedded only with a complete, unique,
verified managed-bucket reference and compatible scope. Multiple configurations,
ambiguous references, data resources, historical objects and missing scope fall
back to independent display. The configuration keeps its own address, action,
search/selection identity and coverage count. The bucket's unknown summary may
use the visible embedded configuration; filtering out that configuration restores
unknown. Filtering out the bucket leaves the selected configuration independent.
Focus and module filters never restore excluded objects.

KMS keys stay independent shared resources. Links reuse existing verified
`Encryption key reference` evidence and require a unique managed key. Literal ARN
or alias matches, external keys, unresolved expressions and sensitive key fields
do not create managed-key links. Deleted/deposed summaries read `before` values
and masks only; current configuration cannot supply their key links or bucket owner.
Data summaries describe observed plan values without changing managed/data identity.
Unchanged cached data resources recovered only from prior state use those prior
values and their sensitive masks; present planned values take precedence.
State-qualified identities and shared coverage accounting are preserved.

The [icon SVG](../examples/encryption.svg),
[service-name SVG](../examples/encryption-service-names.svg) and
[offline HTML](../examples/encryption.html) exercise enabled, disabled, unknown,
external-key and long-address cases. Both outputs use the same provider facts and
measured reference rows. In Docker, regenerate SVGs with
`sh scripts/update-examples.sh` and HTML with:

```sh
cargo run --locked -- tests/fixtures/encryption-plan.json --format html -o examples/encryption.html
```
