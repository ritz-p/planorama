# Architecture icons

## AWS Architecture Icons

Planorama embeds official AWS Architecture Icons from the **07312026 (July 31, 2026)** package downloaded from the [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/) on 2026-10-05.

The selected original SVG files are stored unchanged under `assets/icons/aws/`. `manifest.json` records the official download URL, ZIP SHA-256, original entry paths, and individual file hashes. Only 17 required service icons are bundled; the full ZIP is not committed. Git preserves asset bytes through `.gitattributes`.

### Rights and scope

AWS owns these assets. They are **not MIT-licensed**. The official page permits customers and partners to use the assets in architecture diagrams; the [AWS Site Terms](https://aws.amazon.com/terms/) also apply. The downloaded package contains SVG/PNG assets and no separate license document. This is not a grant of unrestricted redistribution or sublicensing rights. The selected assets support this AWS architecture diagram renderer, not a standalone icon distribution service. For other uses, consult AWS's terms or obtain permission. Planorama does not imply AWS endorsement.

`assets/icons/aws/NOTICE.txt` records ownership and source, and accompanies SVGs that use AWS icons.

### Mapping

AWS selection uses provider identity and exact resource types, independently of ResourceRole. Managed/data entities share the same service icon. Related resources intentionally use service-level icons: subnet, gateway, routing, peering, and network security resources use Amazon VPC; listeners and target groups use Elastic Load Balancing. No public/private subnet status is inferred from the resource type. Unsupported types render without an icon.

## Terraform product logo

The draft implementation uses `terraform-color-24.svg` from the official
[HashiCorp Flight Icons](https://github.com/hashicorp/design-system/tree/97eefa54e00c4e2a3d63417589a8151c4bf17368/packages/flight-icons)
package **5.3.0**, tagged `@hashicorp/flight-icons@5.3.0` at commit
`97eefa54e00c4e2a3d63417589a8151c4bf17368`, retrieved on 2026-10-12.
The original SVG is stored unchanged under `assets/icons/terraform/`, with its
24px view box and original `#7b42bc` color. The adjacent `manifest.json` records
the pinned source paths and URLs, SHA-256 and Git blob hashes, package version,
retrieval date and permission status. Runtime rendering makes no network request.

### License and permission status

The upstream package declares **MPL-2.0**. Its unchanged license text and copyright
notice are included as `assets/icons/terraform/LICENSE.txt`; this records the
package license separately from the rights to use the Terraform brand.
MPL-2.0 section 2.3 does not grant rights in contributor trademarks or logos.
Sections 3.1–3.4 govern source availability, executable distribution and retention
of notices for covered material. The manifest and `NOTICE.txt` identify the
original source.

**Permission to bundle and redistribute the Terraform logo in Planorama binaries
and generated SVGs is unconfirmed.** As reviewed on 2026-10-12, the
[HashiCorp Trademark Policy](https://www.hashicorp.com/en/trademark-policy)
allows current project logos on one's own website to identify and link to official
project pages; other logo uses require written approval from HashiCorp marketing.
It also states that product logos are not licensed to the public under the
software license. The [brand guidelines](https://www.hashicorp.com/en/brand/hcp-product-logos)
describe presentation requirements and do not establish permission for this
bundling use.

The asset is included as a draft implementation candidate. Its manifest retains
`permission_status: unconfirmed` and no permission evidence; the scope must be
resolved before merging or releasing the asset. Copying the package license or
adding attribution does not establish that permission. The Terraform `NOTICE.txt`
accompanies SVGs using this icon and retains that status. Terraform and the
Terraform logo are trademarks of HashiCorp; Planorama implies no HashiCorp
endorsement or unrestricted redistribution rights.

### Mapping

Selection requires the canonical built-in source `terraform.io/builtin/terraform`,
or the narrowly defined `InferredTerraform` fallback when provider information is
entirely absent. Explicit provider information takes precedence; unresolved
bindings, `hashicorp/terraform`, `builtin/terraform`, and foreign hosts do not
select the Terraform icon. See [provider identity](models.md).

| Resource type | Example address | Shared icon |
| --- | --- | --- |
| `terraform_data` | `terraform_data.example` | `ResourceIcon::Terraform` |
| `terraform_remote_state` | `data.terraform_remote_state.example` | `ResourceIcon::Terraform` |

Both types use the symbol ID `planorama-icon-terraform`. Module paths and instance
keys do not alter selection. The icon is independent of managed/data mode,
ResourceRole and action; data cards keep their external marker and dashed border.
Unsupported resource types keep their existing icon-free rendering. This mapping
adds no AWS classification, containment or connection rules.

## Shared rendering

The build script parses each bundled SVG through the same allowlisted parser and
emits reusable inline symbols. Original geometry, groups, view boxes, gradients,
clip paths and masks are preserved; internal IDs and local references are
deterministically namespaced per symbol. The allowlist rejects scripts, event
handlers, CSS, images, external references, duplicate IDs and unresolved local
references at build time. Rust's `include_str!` embeds the validated symbols, so
no XML parser or asset files are needed at runtime.

Repeated resources share their icon symbol. Spatial cards and container headers
display icons at 24px beside the selected full address, preserving the existing
icon gap and heading measurements. Logical cards place the same 24px symbol in
their upper-right corner and reserve heading space for it.
Notices are selected by the artwork actually used;
Terraform-only output does not receive an AWS ownership notice, and mixed output
retains both. Named-state panels namespace IDs and local fragment references.
Action colors, external markers, metadata and resource identity remain separate
from the artwork.

## Updating sources

For AWS, download a release directly from AWS and select the original service
SVGs. For Terraform, use the official Flight Icons repository and pin a release
tag to its commit. Update original files, manifests, notices and source dates
together; retain the complete upstream license when applicable. Review changed
license, trademark and brand conditions, and record evidence before changing a
permission status from `unconfirmed`.

Verify source hashes and inspect SVGs for external dependencies or executable
content. Preserve the original files without recoloring or editing their paths.
Let the [CI SVG and validation workflows](ci.md) regenerate examples and run
formatting, tests and Clippy. Original icon assets are not generated outputs.
