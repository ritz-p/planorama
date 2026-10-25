# Service labels and artwork migration

Planorama uses provider-aware service names in native, library, Wasm, SVG and
offline HTML output. AWS Architecture Icons are no longer bundled or embedded
(#632). AWS's response concerning distribution in third-party applications
motivated removal; this is not a claim that all use in architecture diagrams is
prohibited. No icon download, proxy or external icon URL is used.

## Configuration migration

Omit `--icon-mode`, or select `--icon-mode service-name`. TOML uses
`icon-mode = "service-name"`; Rust uses `IconMode::ServiceName`; Wasm options
use `"icon_mode":"service-name"`. All defaults select service names.
`IconMode::Icon` and the old `icon` string remain recognized only to return an
explicit invalid-options error. The `icon-artwork` Cargo feature is a deprecated
no-op for build compatibility; even `--all-features` includes no artwork.

## Classification and rendering

The existing exact resource-type and provider identity mappings remain the source
of service names. Foreign and unresolved providers do not gain AWS classification
from a type prefix. Unknown types retain their address and neutral presentation.
Service names wrap using shared measurement on spatial cards, container headings
and Logical Resources. Resource identity, action, managed/data state, relationships,
coverage and viewer operations retain their meaning.

The original Planorama Terraform state-file SVG and its MIT license/notice remain
in `assets/icons/terraform` as source assets, but are not embedded by the renderer.
Cargo dependency notices and the repository licenses remain in release packages.
AWS-specific notices are removed with the AWS artwork.

## Existing distributions and site follow-up

On 2026-10-25, GitHub Releases for ritz-p/planorama contained no releases
(`gh release list --limit 100`). Git history still contains previous artwork;
this change does not rewrite history. The Actions artifact inventory on that date contained 557 entries: 160 native
archives (40 per supported target), 168 layout-quality bundles, 92 SVG-example
bundles, 27 spacing-calibration bundles and 110 routing-regression logs. Native
archives and diagram bundles from icon-enabled revisions are retirement targets;
routing logs require a content check before deciding whether removal is needed.
This inventory is metadata-based, not a claim that every archive was extracted.
Use the repository Actions artifact API to identify IDs, source runs and expiration
before separately authorizing deletion. Old Actions artifacts and local build
outputs must not be republished as current builds. Rebuild native binaries, Wasm
bindings and archives from this revision, and retire any retained old build
artifacts through a separately authorized cleanup.

The local planorama-site checkout has unrelated uncommitted work. Its
`samples/manifest.json` references engine commit
`65782ebfbad9ad9fcf2a6cb1232cf22eda5f319d`; its `dist/licenses/AWS-ICONS-NOTICE.txt`
is a stale build output. Follow-up in that repository must update the pinned
engine and bindings together, rebuild public samples with service-name output,
remove obsolete AWS notice-copying steps and direct artwork, and replace deployed
Wasm/demo assets and cached versions. Check the deployed asset URLs after publishing;
a local rebuild alone does not retire an existing deployment or browser cache.
Existing site work and deployments are not modified by this PR.

User-supplied or remote artwork is a separate feature and permission question;
it is not an alternative implementation of removal.

## Compact service labels (#633)

Short names are explicit shared metadata, applied only after provider-aware service
classification. Every other name is preserved in full and wraps without truncation.
Unknown provider/type keeps its existing neutral fallback. Labels use bold text,
6px horizontal padding and a category background; address and operation text remain
separate. Spatial cards, container headings, embedded members and Logical Resources
use the same measurement and rendering. Formal names also remain in SVG metadata.

| Formal service name | Label |
| --- | --- |
| Amazon EC2 | EC2 |
| Amazon S3 | S3 |
| AWS Lambda | Lambda |
| Amazon VPC | VPC |
| Amazon ECS | ECS |
| Amazon EKS | EKS |
| Amazon ECR | ECR |
| Amazon RDS | RDS |
| Amazon DynamoDB | DynamoDB |
| Amazon ElastiCache | ElastiCache |
| Amazon EFS | EFS |
| Amazon EBS | EBS |
| Amazon Simple Queue Service | SQS |
| Amazon Simple Notification Service | SNS |
| AWS Identity and Access Management | IAM |
| AWS Key Management Service | KMS |
| Elastic Load Balancing | ELB |
| Amazon CloudWatch | CloudWatch |
| Amazon Route 53 | Route 53 |
| Amazon CloudFront | CloudFront |

## Neutral role marks (#634)

Role marks are small neutral shapes in the service-label row of individual spatial
cards. They describe a visual role independently of the graph's ResourceRole:
execution can include both Node and Controller, while network marks identify
selected Connector resources. Classification uses verified AWS provider identity
and an explicit resource-type allowlist, never names, ARNs or arbitrary attributes.

| Visual role | Mark | Initial examples |
| --- | --- | --- |
| Execution | Rounded rectangle | EC2 instance, Lambda function, ECS service, EKS cluster |
| Data store | Cylinder | S3 bucket, EBS volume, EFS filesystem, RDS database/cluster, DynamoDB table, ElastiCache cache, Redshift cluster, OpenSearch domain |
| Network | Hexagon | NAT/internet gateway, VPC endpoint, load balancer, CloudFront distribution, API Gateway API |

Other types, foreign/unknown providers, Identity/Policy/Association objects,
Logical rows, embedded members and containers use the existing neutral card
presentation without a role mark. A VPC/Subnet containment boundary is not a
network mark. The mark occupies a reserved 22x18px area within the header; service
text wraps in the remaining width. Connections, arrowheads and relationship
numbers attach to the card boundary, and selection uses that same enclosing card.
Marks do not add graph entities, relationships or containment. Their meaning is
also provided in SVG descriptions and accessible labels; color is not required.

[Role mark example](../examples/role-shapes.svg) includes long addresses, multiple
actions, a container, a logical identity and foreign/unknown providers.

## Service category colors (#635)

The shared service metadata assigns colors only after AWS provider-aware service
classification. The same badge renderer covers spatial cards, container headings,
embedded members and Logical Resources in SVG and HTML.

| Category | Services | Background / text |
| --- | --- | --- |
| Compute (orange) | Amazon EC2, AWS Lambda, Amazon ECS, Amazon EKS, Amazon EC2 Auto Scaling | `#ffedd5` / `#7c2d12` |
| Storage (green) | Amazon S3, Amazon EBS, Amazon EFS | `#dcfce7` / `#14532d` |
| Network (blue) | Amazon VPC, Elastic Load Balancing, Amazon Route 53, Amazon CloudFront, Amazon API Gateway | `#dbeafe` / `#1e3a8a` |
| Neutral (gray) | Every other service, unknown classification and other providers | `#e2e8f0` / `#0f172a` |

Unknown provider/types retain their neutral presentation without inventing a
service label. Category colors apply only to service-label backgrounds, never the
whole card or operation indicators. Data/import, action, focus and selection
indicators retain their own presentation. Bold service text remains readable in
grayscale; every palette pair exceeds a 4.5:1 contrast ratio. Long names wrap using
the same measurements as neutral badges. SVG descriptions explain the colors
without adding duplicate category labels. Classification does not change graph
roles, relationships, containment, or layout.
