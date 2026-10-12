# Architecture icons

## AWS Architecture Icons

Planorama embeds official AWS Architecture Icons from the **07312026 (July 31, 2026)** package downloaded from the [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/) on 2026-10-05.

The selected original SVG files are stored unchanged under `assets/icons/aws/`. `manifest.json` records the official download URL, ZIP SHA-256, original entry paths, and individual file hashes. Only 17 required service icons are bundled; the full ZIP is not committed. Git preserves asset bytes through `.gitattributes`.

### Rights and scope

AWS owns these assets. They are **not MIT-licensed**. The official page permits customers and partners to use the assets in architecture diagrams; the [AWS Site Terms](https://aws.amazon.com/terms/) also apply. The downloaded package contains SVG/PNG assets and no separate license document. This is not a grant of unrestricted redistribution or sublicensing rights. The selected assets support this AWS architecture diagram renderer, not a standalone icon distribution service. For other uses, consult AWS's terms or obtain permission. Planorama does not imply AWS endorsement.

`assets/icons/aws/NOTICE.txt` records ownership and source, and accompanies SVGs that use AWS icons.

### Mapping

AWS selection uses provider identity and exact resource types, independently of ResourceRole. Managed/data entities share the same service icon. Related resources intentionally use service-level icons: subnet, gateway, routing, peering, and network security resources use Amazon VPC; listeners and target groups use Elastic Load Balancing. No public/private subnet status is inferred from the resource type. Unsupported types render without an icon.

## Planorama icon for Terraform built-in resources

Planorama uses an original neutral state-file symbol for its Terraform built-in
provider category. Version **1.0.0**, created on 2026-10-12, is stored at
`assets/icons/terraform/terraform-builtin-24.svg` with a 24px view box. The adjacent
`manifest.json` records the Planorama source, version, date, file hash and license
scope. This independent artwork is not the HashiCorp Terraform product logo and
does not reproduce its stylized T or brand design.

The Flight Icons logo candidate was removed because permission for bundling it
had not been established. The
[HashiCorp Trademark Policy](https://www.hashicorp.com/en/trademark-policy)
distinguishes graphic-logo permission from the software license; this replacement
does not claim permission to redistribute HashiCorp logos or imply endorsement.

### License and scope

The original Planorama icon is licensed under **MIT**. The complete license and
`Copyright (c) 2026 Planorama contributors` notice are in
`assets/icons/terraform/LICENSE.txt`. The license applies to
`terraform-builtin-24.svg` and its embedded copies, not to the repository as a
whole or to AWS assets. Its terms permit copying, modification and redistribution
with the copyright and permission notices retained.

`NOTICE.txt` records the artwork's source and independent identity. SVGs using
this icon embed that notice together with the complete MIT license, so the
permission notice accompanies distributed diagrams. Runtime rendering makes no
network request.

### Mapping

Selection requires the canonical built-in source `terraform.io/builtin/terraform`,
or the narrowly defined `InferredTerraform` fallback when provider information is
entirely absent. Explicit provider information takes precedence; unresolved
bindings, `hashicorp/terraform`, `builtin/terraform`, and foreign hosts do not
select the built-in provider icon. See [provider identity](models.md).

| Resource type | Example address | Shared icon |
| --- | --- | --- |
| `terraform_data` | `terraform_data.example` | `ResourceIcon::Terraform` |
| `terraform_remote_state` | `data.terraform_remote_state.example` | `ResourceIcon::Terraform` |

Both types use the symbol ID `planorama-icon-terraform`. Module paths and instance
keys do not alter selection. The icon is independent of managed/data mode,
ResourceRole and action; spatial data cards keep their external marker and dashed
border, while logical data rows retain external metadata and a dashed underline.
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
icon gap and heading measurements. Logical-resource groups place the same 24px
symbol in the upper-right corner of each resource row and reserve address-heading
space for it.
Notices are selected by the artwork actually used;
Terraform-only output does not receive an AWS ownership notice, and mixed output
retains both. Named-state panels namespace IDs and local fragment references.
Action colors, external markers, metadata and resource identity remain separate
from the artwork.

## Updating sources

For AWS, download a release directly from AWS and select the original service
SVGs. Preserve their original colors and paths, and update the manifest, notice
and source dates together. Review changed license, trademark and brand conditions.

For Planorama's built-in provider icon, update the original SVG and its version,
hash and source date in the manifest together. Retain the asset's MIT license and
notice, and keep the artwork independent of HashiCorp's product logos. Introducing
third-party artwork requires a separate source and rights review before bundling.

Verify source hashes and inspect SVGs for external dependencies or executable
content.
Let the [CI SVG and validation workflows](ci.md) regenerate examples and run
formatting, tests and Clippy. Original icon assets are not generated outputs.
