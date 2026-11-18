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

## Bedrock agents, knowledge bases and flows (#873)

Pinned Provider/source revision is the same as #872. Each of the nine managed
agent-family types retains its Logical presentation: configured helper ownership
is separate from subnet placement, and shared model, alias, storage and query
backends remain independent. No service-generated agent version node is created.

| Consumer input | Managed output and discriminator |
| --- | --- |
| Agent `agent_resource_role_arn`, flow `execution_role_arn`, knowledge base `role_arn` | IAM Role `arn` |
| Agent/flow/prompt `customer_encryption_key_arn`, data source `server_side_encryption_configuration.kms_key_arn` | KMS Key `arn`; data-source ancestor is a singleton list |
| Action group / alias / collaborator / association `agent_id` | Agent `agent_id`, never composite helper `id`; action group requires known `action_group_id` and `DRAFT` |
| Association / data source `knowledge_base_id` | Knowledge Base `id`; association requires both endpoints, `DRAFT`, and known `ENABLED` or `DISABLED` |
| Collaborator `agent_descriptor.alias_arn` | Agent Alias `agent_alias_arn`; singleton list, both alias and supervising agent required, `DRAFT` and known `collaborator_id` |
| Action group `action_group_executor.lambda` | Lambda Function `arn`; singleton list, absent/empty `custom_control`; `RETURN_CONTROL` prevents a link |
| Action group `api_schema.s3.s3_bucket_name` | S3 Bucket `id` / `bucket`; two singleton lists and absent/empty inline `payload` |
| Data source `data_source_configuration.s3_configuration.bucket_arn` | S3 Bucket `arn`; two singleton lists and exclusive `S3` type |
| Data source `data_source_configuration.{confluence,salesforce,share_point}_configuration.source_configuration.credentials_secret_arn` | Secrets Manager Secret `arn`; three singleton lists and exclusive `CONFLUENCE` / `SALESFORCE` / `SHAREPOINT` type |
| Data source `vector_ingestion_configuration.context_enrichment_configuration.bedrock_foundation_model_configuration.model_arn` | System-defined cross-Region inference profile ARN and known `type = SYSTEM_DEFINED`; three singleton lists, `BEDROCK_FOUNDATION_MODEL` enrichment type; foundation model identifiers stay external |
| Data source `vector_ingestion_configuration.custom_transformation_configuration.transformation.transformation_function.transformation_lambda_configuration.lambda_arn` | Lambda Function `arn`; five singleton lists, known `POST_CHUNKING` step |
| Knowledge base `storage_configuration.opensearch_serverless_configuration.collection_arn` / `opensearch_managed_cluster_configuration.domain_arn` / `rds_configuration.resource_arn` / `neptune_analytics_configuration.graph_arn` / `s3_vectors_configuration.index_arn` | OpenSearch Serverless Collection `arn` / OpenSearch Domain `arn` / RDS Cluster `arn` / Neptune Graph `arn` / S3 Vectors Index `index_arn`; two singleton lists, `VECTOR` knowledge-base type and exclusive matching storage type |
| Knowledge base `storage_configuration.{mongo_db_atlas,pinecone,rds,redis_enterprise_cloud}_configuration.credentials_secret_arn` | Secrets Manager Secret `arn`; two singleton lists, `VECTOR` and matching exclusive storage discriminator |
| Knowledge base `knowledge_base_configuration.kendra_knowledge_base_configuration.kendra_index_arn` | Kendra Index `arn`; two singleton lists, exclusive `KENDRA` knowledge-base type |
| Knowledge base `knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.provisioned_configuration.cluster_identifier` / `serverless_configuration.workgroup_arn` | Redshift Cluster `cluster_identifier` / Serverless Workgroup `arn`; five singleton lists, exclusive `SQL`, `REDSHIFT`, `PROVISIONED` / `SERVERLESS` branches |
| Same provisioned/serverless query branch `auth_configuration.username_password_secret_arn` | Secrets Manager Secret `arn`; additional singleton list, `USERNAME_PASSWORD` auth type |
| Prompt `variant.model_id` / `variant.gen_ai_resource.agent.agent_identifier` | Inference Profile `arn` / `id`, or Agent `agent_arn`, with mutually exclusive model / agent branches; variant list, remaining ancestors singleton lists |

Agent `guardrail_configuration` is an optional **list-of-object attribute**, not
a nested block. Its single object must have known `guardrail_identifier` and a
positive numeric `guardrail_version`. The root Terraform expression must reference
exactly a Guardrail `guardrail_id` / `guardrail_arn` and a Guardrail Version
`version`; that version must itself explicitly reference this same guardrail ARN.
Matching another guardrail's numerical version does not establish identity.

Agent `prompt_override_configuration` is an optional/computed list-of-object
attribute. The root expression must reference only a Lambda Function `arn`
matching `override_lambda`. Its `prompt_configurations` is a **set of objects**;
all selected parser, creation, state and prompt-type discriminators must be known
valid enums and at least one parser mode must be `OVERRIDDEN`. Opaque prompt text
and inference settings are never inspected for references.

Alias `routing_configuration` is an optional/computed list-of-object attribute
with at most one item. A root reference to Provisioned Model Throughput
`provisioned_model_arn` requires matching `provisioned_throughput` and a known
positive published `agent_version`; `DRAFT`, literal versions without a managed
throughput reference and computed routing alone create no extra node.

Flow `definition` is a singleton list block and `node` is a list. Configuration
lists below each node are singleton lists. Only matching node `type` and an
exclusive `configuration` branch qualify these exact paths:

| Path after `definition.node.configuration` | Node type | Required output |
| --- | --- | --- |
| `agent.agent_alias_arn` | `Agent` | Agent Alias `agent_alias_arn` |
| `knowledge_base.knowledge_base_id` | `KnowledgeBase` | Knowledge Base `id` |
| `lambda_function.lambda_arn` | `LambdaFunction` | Lambda Function `arn` |
| `prompt.source_configuration.resource.prompt_arn` | `Prompt` | Prompt `arn`, absent/empty inline source |
| `retrieval.service_configuration.s3.bucket_name` | `Retrieval` | S3 Bucket `id` / `bucket` |
| `storage.service_configuration.s3.bucket_name` | `Storage` | S3 Bucket `id` / `bucket` |

The whole collection must match its indexed configuration references; partial
node/variant evidence and unknown alternative branches fall back. Lex V2 alias
ARNs stay external because pinned Provider 6.70.0 has no managed Lex V2 alias
resource; Classic Lex alias is not interchangeable. Database/table/index names
without a selected parent contract, supplemental URI storage, embedding model
payloads, connector URLs, tenant IDs, inline code, node/connection names, prompt
templates, tool schemas and runtime status remain opaque. Safe summaries remain
deferred. None of these references assigns a knowledge base to a backend VPC.

All links require complete, unique, current managed Terraform evidence and known
non-sensitive matching output values, compatible independent provider/account/
region and named-state scope. Unknown, deleted/deposed, data, missing/filtered,
foreign-provider and ambiguous references retain fallback with independent
resource actions and coverage. See [SVG](../examples/bedrock-agents.svg), sourced
from `tests/fixtures/bedrock-agents-plan.json`.

### SageMaker Studio and notebook references

The domain has an independent card. VPC-only placement requires the declared VPC and complete subnet and default-user security-group collections to agree. A notebook can use its single verified subnet when all security groups prove the same VPC. An app selects a domain-qualified profile or space. Only private spaces link to their owner profile. Repository URLs and the generated home EFS ID do not create resources.

| Terraform resource | Configured input | Accepted endpoint outputs |
| --- | --- | --- |
| `aws_sagemaker_app` | `domain_id` | `aws_sagemaker_domain` (`id`) |
|  | `user_profile_name` | `aws_sagemaker_user_profile` (`user_profile_name`) |
|  | `space_name` | `aws_sagemaker_space` (`space_name`) |
|  | `resource_spec.lifecycle_config_arn` | `aws_sagemaker_studio_lifecycle_config` (`arn`) |
|  | `resource_spec.sagemaker_image_arn` | `aws_sagemaker_image` (`arn`) |
|  | `resource_spec.sagemaker_image_version_arn` | `aws_sagemaker_image_version` (`arn`) |
| `aws_sagemaker_code_repository` | `git_config.secret_arn` | `aws_secretsmanager_secret` (`arn`) |
| `aws_sagemaker_domain` | `vpc_id` | `aws_vpc` (`id`) |
|  | `subnet_ids` | `aws_subnet` (`id`) |
|  | `default_user_settings.execution_role` | `aws_iam_role` (`arn`) |
|  | `default_user_settings.security_groups` | `aws_security_group` (`id`) |
|  | `default_space_settings.custom_file_system_config.efs_file_system_config.file_system_id`, `default_user_settings.custom_file_system_config.efs_file_system_config.file_system_id` | `aws_efs_file_system` (`id`) |
|  | `default_space_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn`, `default_space_settings.jupyter_lab_app_settings.lifecycle_config_arns`, `default_space_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn`, `default_space_settings.jupyter_server_app_settings.lifecycle_config_arns`, `default_space_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn`, `default_space_settings.kernel_gateway_app_settings.lifecycle_config_arns`, `default_space_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.code_editor_app_settings.built_in_lifecycle_config_arn`, `default_user_settings.code_editor_app_settings.lifecycle_config_arns`, `default_user_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn`, `default_user_settings.jupyter_lab_app_settings.lifecycle_config_arns`, `default_user_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.jupyter_server_app_settings.lifecycle_config_arns`, `default_user_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.kernel_gateway_app_settings.lifecycle_config_arns`, `default_user_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.r_session_app_settings.default_resource_spec.lifecycle_config_arn`, `default_user_settings.tensor_board_app_settings.default_resource_spec.lifecycle_config_arn` | `aws_sagemaker_studio_lifecycle_config` (`arn`) |
|  | `default_space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn`, `default_space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn`, `default_space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_arn`, `default_user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_arn` | `aws_sagemaker_image` (`arn`) |
|  | `default_space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_version_arn`, `default_user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_version_arn` | `aws_sagemaker_image_version` (`arn`) |
| `aws_sagemaker_image` | `role_arn` | `aws_iam_role` (`arn`) |
| `aws_sagemaker_image_version` | `image_name` | `aws_sagemaker_image` (`image_name`) |
| `aws_sagemaker_notebook_instance` | `role_arn` | `aws_iam_role` (`arn`) |
|  | `subnet_id` | `aws_subnet` (`id`) |
|  | `security_groups` | `aws_security_group` (`id`) |
|  | `kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
|  | `lifecycle_config_name` | `aws_sagemaker_notebook_instance_lifecycle_configuration` (`name`) |
|  | `default_code_repository` | `aws_sagemaker_code_repository` (`code_repository_name`) |
| `aws_sagemaker_space` | `domain_id` | `aws_sagemaker_domain` (`id`) |
|  | `ownership_settings.owner_user_profile_name` | `aws_sagemaker_user_profile` (`user_profile_name`) |
|  | `space_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn`, `space_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn`, `space_settings.jupyter_server_app_settings.lifecycle_config_arns`, `space_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn`, `space_settings.kernel_gateway_app_settings.lifecycle_config_arns`, `space_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn` | `aws_sagemaker_studio_lifecycle_config` (`arn`) |
|  | `space_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn`, `space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn`, `space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn`, `space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn` | `aws_sagemaker_image` (`arn`) |
|  | `space_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn`, `space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn`, `space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn`, `space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn` | `aws_sagemaker_image_version` (`arn`) |
|  | `space_settings.custom_file_system.efs_file_system.file_system_id` | `aws_efs_file_system` (`id`) |
| `aws_sagemaker_user_profile` | `domain_id` | `aws_sagemaker_domain` (`id`) |
|  | `user_settings.execution_role` | `aws_iam_role` (`arn`) |
|  | `user_settings.security_groups` | `aws_security_group` (`id`) |
|  | `user_settings.code_editor_app_settings.built_in_lifecycle_config_arn`, `user_settings.code_editor_app_settings.lifecycle_config_arns`, `user_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn`, `user_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn`, `user_settings.jupyter_lab_app_settings.lifecycle_config_arns`, `user_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn`, `user_settings.jupyter_server_app_settings.lifecycle_config_arns`, `user_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn`, `user_settings.kernel_gateway_app_settings.lifecycle_config_arns`, `user_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn`, `user_settings.r_session_app_settings.default_resource_spec.lifecycle_config_arn`, `user_settings.tensor_board_app_settings.default_resource_spec.lifecycle_config_arn` | `aws_sagemaker_studio_lifecycle_config` (`arn`) |
|  | `user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn`, `user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn`, `user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn`, `user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn`, `user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_arn`, `user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_arn` | `aws_sagemaker_image` (`arn`) |
|  | `user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn`, `user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn`, `user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn`, `user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn`, `user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_version_arn`, `user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_version_arn` | `aws_sagemaker_image_version` (`arn`) |
|  | `user_settings.custom_file_system_config.efs_file_system_config.file_system_id` | `aws_efs_file_system` (`id`) |

### SageMaker Training and monitoring jobs

Job references require complete row-specific evidence. The file-system type selects EFS or FSx for Lustre. Named monitoring definitions select DataQuality; an inline job uses its own endpoints and network. Network evidence is collected within a single definition. Pipeline definition JSON and generated jobs remain opaque.

| Terraform resource | Configured input | Accepted endpoint outputs |
| --- | --- | --- |
| `aws_sagemaker_algorithm` | `validation_specification.validation_role` | `aws_iam_role` (`arn`) |
|  | `validation_specification.validation_profiles.training_job_definition.input_data_config.data_source.file_system_data_source.file_system_id` | `aws_efs_file_system` (`id`) or `aws_fsx_lustre_file_system` (`id`) |
|  | `validation_specification.validation_profiles.training_job_definition.output_data_config.kms_key_id`, `validation_specification.validation_profiles.training_job_definition.resource_config.volume_kms_key_id`, `validation_specification.validation_profiles.transform_job_definition.transform_output.kms_key_id`, `validation_specification.validation_profiles.transform_job_definition.transform_resources.volume_kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
| `aws_sagemaker_data_quality_job_definition` | `role_arn` | `aws_iam_role` (`arn`) |
|  | `data_quality_job_input.endpoint_input.endpoint_name` | `aws_sagemaker_endpoint` (`name`) |
|  | `data_quality_job_output_config.kms_key_id`, `job_resources.cluster_config.volume_kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
|  | `network_config.vpc_config.security_group_ids` | `aws_security_group` (`id`) |
|  | `network_config.vpc_config.subnets` | `aws_subnet` (`id`) |
| `aws_sagemaker_hyper_parameter_tuning_job` | `warm_start_config.parent_hyper_parameter_tuning_jobs.name` | `aws_sagemaker_hyper_parameter_tuning_job` (`name`) |
|  | `training_job_definition.role_arn`, `training_job_definitions.role_arn` | `aws_iam_role` (`arn`) |
|  | `training_job_definition.algorithm_specification.algorithm_name`, `training_job_definitions.algorithm_specification.algorithm_name` | `aws_sagemaker_algorithm` (`algorithm_name`) |
|  | `training_job_definition.hyper_parameter_tuning_resource_config.volume_kms_key_id`, `training_job_definition.output_data_config.kms_key_id`, `training_job_definition.resource_config.volume_kms_key_id`, `training_job_definitions.hyper_parameter_tuning_resource_config.volume_kms_key_id`, `training_job_definitions.output_data_config.kms_key_id`, `training_job_definitions.resource_config.volume_kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
|  | `training_job_definition.input_data_config.data_source.file_system_data_source.file_system_id`, `training_job_definitions.input_data_config.data_source.file_system_data_source.file_system_id` | `aws_efs_file_system` (`id`) or `aws_fsx_lustre_file_system` (`id`) |
|  | `training_job_definition.vpc_config.security_group_ids`, `training_job_definitions.vpc_config.security_group_ids` | `aws_security_group` (`id`) |
|  | `training_job_definition.vpc_config.subnets`, `training_job_definitions.vpc_config.subnets` | `aws_subnet` (`id`) |
| `aws_sagemaker_monitoring_schedule` | `monitoring_schedule_config.monitoring_job_definition_name` | `aws_sagemaker_data_quality_job_definition` (`name`) |
|  | `monitoring_schedule_config.monitoring_job_definition.role_arn` | `aws_iam_role` (`arn`) |
|  | `monitoring_schedule_config.monitoring_job_definition.monitoring_inputs.endpoint_input.endpoint_name` | `aws_sagemaker_endpoint` (`name`) |
|  | `monitoring_schedule_config.monitoring_job_definition.monitoring_output_config.kms_key_id`, `monitoring_schedule_config.monitoring_job_definition.monitoring_resources.cluster_config.volume_kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
|  | `monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.security_group_ids` | `aws_security_group` (`id`) |
|  | `monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.subnets` | `aws_subnet` (`id`) |
| `aws_sagemaker_pipeline` | `role_arn` | `aws_iam_role` (`arn`) |
|  | `pipeline_definition_s3_location.bucket` | `aws_s3_bucket` (`bucket`, `id`) |
| `aws_sagemaker_training_job` | `role_arn` | `aws_iam_role` (`arn`) |
|  | `algorithm_specification.algorithm_name` | `aws_sagemaker_algorithm` (`algorithm_name`) |
|  | `mlflow_config.mlflow_resource_arn` | `aws_sagemaker_mlflow_app` (`arn`) or `aws_sagemaker_mlflow_tracking_server` (`arn`) |
|  | `model_package_config.model_package_group_arn` | `aws_sagemaker_model_package_group` (`arn`) |
|  | `input_data_config.data_source.file_system_data_source.file_system_id` | `aws_efs_file_system` (`id`) or `aws_fsx_lustre_file_system` (`id`) |
|  | `output_data_config.kms_key_id`, `resource_config.volume_kms_key_id` | `aws_kms_key` (`arn`, `key_id`) |
|  | `vpc_config.security_group_ids` | `aws_security_group` (`id`) |
|  | `vpc_config.subnets` | `aws_subnet` (`id`) |
