# DynamoDB configuration references (#690)

The exact contracts below were checked against AWS Provider **6.70.0**, commit
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, its official
[DynamoDB resource documentation](https://github.com/hashicorp/terraform-provider-aws/tree/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f/website/docs/r),
and the full `terraform providers schema -json` output. The schema SHA-256 is
`f04a44109a82886c3e392ae88e73dfa533f677ccd8a1b253d7d7b0e3d3ea9857`, matching the
[inventory](aws-inventory.md). All reference attributes below are required strings;
the legacy `replica` block is a required set with required string `region_name`.

| Exact consumer type | Input → exact endpoint attribute | Presentation |
| --- | --- | --- |
| `aws_dynamodb_table_replica` | `global_table_arn` → `aws_dynamodb_table.arn` | Regional logical resource; V2 table membership, with a different known region from the main table |
| `aws_dynamodb_global_table` | `name` → `aws_dynamodb_table.name` or `.id` | Global logical V1 configuration; membership only for the explicitly referenced table whose region occurs in the known, unique `replica[].region_name` set |
| `aws_dynamodb_kinesis_streaming_destination` | `table_name` → `aws_dynamodb_table.name` or `.id`; `stream_arn` → `aws_kinesis_stream.arn` | Regional logical configuration with independently verified table and stream navigation |
| `aws_dynamodb_contributor_insights` | `table_name` → `aws_dynamodb_table.name` or `.id` | Regional logical configuration with table navigation |
| `aws_dynamodb_global_secondary_index` | `table_name` → `aws_dynamodb_table.name` or `.id` | Regional logical configuration with table navigation |
| `aws_dynamodb_resource_policy` | `resource_arn` → `aws_dynamodb_table.arn` or `aws_dynamodb_table_replica.arn` | Regional logical policy with table navigation |
| `aws_dynamodb_tag` | `resource_arn` → `aws_dynamodb_table.arn` or `aws_dynamodb_table_replica.arn` | Regional logical controller with table navigation |

Each link requires one explicit, unambiguous managed AWS endpoint, the correct
source attribute, and equal known nonempty values. Both resources must be current,
non-deposed and non-deleting. Regional configuration endpoints must have the same
known region. Provider bindings must have equal verified accounts, or be the same
binding when neither account is known; conflicting or one-sided account evidence
fails. Separate aliases without verified account evidence remain unresolved even
when their literal names or ARN account components match. Account evidence follows
the existing [saved-plan verification](provider-evidence.md); plain JSON does not
prove an account through `assume_role` text.

Unknown, missing, ambiguous, incompatible and transformed endpoints retain generic
Terraform references and an unproven-endpoint diagnostic when a reference is
present. Literal values alone do not create dedicated links. Legacy V1 literal
names, `depends_on` and region lists do not identify every regional table. V1
configuration never acts as a V2 replica endpoint. No table or stream is synthesized.

These are configuration references, with no new containment, member embedding or
routed delivery edges. Table and Kinesis nodes retain existing presentation.
Actions, import/move provenance, current/deposed identity and resource coverage
remain attached to each original resource. Named states retain separate identities;
filtered endpoints become unavailable navigation targets.

No table item data, keys, index values, policy bodies, tag keys/values or arbitrary
configuration values are emitted. Stream references do not assert delivery,
authorization, health or reachability. Contributor Insights index selection does
not invent a separate index owner.

Remaining gaps are recorded by the #718 review in the
[semantic gap register](aws-semantic-audit.md): table export/import/restore references (#744), deliberately excluded item contents,
inline replica/GSI identity expansion, stream-policy targets, transformed replica
tag ARNs and at-rest summaries remain deferred. Lambda mappings support explicit
managed Table `stream_arn` references under the [#950 contract](aws-messaging.md#additional-managed-sources-and-configuration-950).

`tests/fixtures/dynamodb-plan.json` and `tests/unit/semantic/dynamodb.rs` cover
positive and conservative cases. `examples/dynamodb.svg` is maintained by
`scripts/update-examples.sh`. Browser tests cover native/Wasm SVG/HTML parity and
presentation geometry.
