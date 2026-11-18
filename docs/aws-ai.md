# AWS AI and machine learning

[AWS 共通契約・サービス一覧](aws.md)。以下は固定 Provider の型・入力・出力と証拠条件です。

## Bedrock model and invocation references (#872)

These contracts use AWS Provider 6.70.0 at
`5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f`, including its framework list blocks
and required/optional/computed distinctions. Model customization and invocation
jobs, model/profile identities, guardrails and regional logging configuration
retain their independent Logical presentation, actions and exactly-once coverage.

| Configured source path | Required managed output |
| --- | --- |
| Custom model `base_model_identifier` | Custom model `custom_model_arn`, never `job_arn` or a model name |
| Custom model / evaluation / batch `role_arn` | IAM Role `arn` with global IAM account compatibility |
| Custom model `custom_model_kms_key_id`, evaluation `customer_encryption_key_id`, guardrail `kms_key_arn` | KMS Key `arn` |
| Batch `output_data_config.s3_output_data_config.s3_encryption_key_id` | KMS Key `arn`; two singleton list ancestors |
| Guardrail version `guardrail_arn` | Guardrail `guardrail_arn` plus known published numerical `version` |
| Enforced configuration `guardrail_arn` / `guardrail_version` | Matching guardrail ARN and an explicit Guardrail Version `version` with the same parent ARN; `DRAFT`, zero or unknown versions stay unresolved |
| Inference profile `model_source.copy_from` | System-defined inference profile ARN from an explicit managed reference with known `type = SYSTEM_DEFINED`; singleton list |
| Batch `model_id` | Managed Inference Profile `arn` / `id` |
| Provisioned throughput `model_arn` | Custom model `custom_model_arn` |
| Evaluation `evaluation_config.human.human_workflow_config.flow_definition_arn` | SageMaker Flow Definition `arn`; three singleton lists |
| Evaluation `inference_config.rag_config.knowledge_base_config.retrieve_config.knowledge_base_id` or `retrieve_and_generate_config.knowledge_base_id` | Bedrock Knowledge Base `id`; four singleton lists |
| Logging `logging_config.cloudwatch_config.log_group_name` / `role_arn` | CloudWatch Log Group `name` / IAM Role `arn`; singleton lists |
| Logging `logging_config.s3_config.bucket_name` / `logging_config.cloudwatch_config.large_data_delivery_s3_config.bucket_name` | S3 Bucket `id` / `bucket`; singleton lists |

Evaluation requires exactly one known human/automated and model/RAG branch.
Human workflow references require human mode; knowledge-base references require
RAG with exactly one knowledge-base/precomputed source and retrieve/retrieve-and-
generate branch. Model comparisons can contain up to two model blocks. Opaque
instructions, prompts, datasets and precomputed source labels are not endpoints.

Custom model and batch `vpc_config` are optional singleton lists containing
required string sets `subnet_ids` and `security_group_ids`. Both collections must
be complete and each current managed Subnet/SG must explicitly reference the same
managed VPC with matching known IDs and scope. This is customization/batch **data
context**, not model or inference-serving subnet containment. The output model
and customization-job ARNs remain distinct; no extra job node is synthesized.

All links require unique explicit current managed references, matching known
non-sensitive values and independent compatible provider/account/region evidence.
Unknown, ambiguous, incomplete, deleted/deposed, data, filtered or incompatible
endpoints retain conservative fallback; named states cannot supply each other's
missing resources. Foundation models, service guardrail profiles, computed model
region lists and S3 artifact URIs do not create Terraform nodes or alter scope.
No policy effects, payloads, job status, runtime health or safe summary is inferred.
See [SVG](../examples/bedrock-models.svg); generate HTML from
`tests/fixtures/bedrock-models-plan.json` using `--format html`.
