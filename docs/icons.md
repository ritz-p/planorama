# AWS Architecture Icons

Planorama embeds official AWS Architecture Icons from the **07312026 (July 31, 2026)** package downloaded from the [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/) on 2026-10-05.

The selected original SVG files are stored unchanged under `assets/icons/aws/`. `manifest.json` records the official download URL, ZIP SHA-256, original entry paths, and individual file hashes. Only 17 required service icons are bundled; the full ZIP is not committed. Git preserves asset bytes through `.gitattributes`.

## Rights and scope

AWS owns these assets. They are **not MIT-licensed**. The official page permits customers and partners to use the assets in architecture diagrams; the [AWS Site Terms](https://aws.amazon.com/terms/) also apply. The downloaded package contains SVG/PNG assets and no separate license document. This is not a grant of unrestricted redistribution or sublicensing rights. The selected assets support this AWS architecture diagram renderer, not a standalone icon distribution service. For other uses, consult AWS's terms or obtain permission. Planorama does not imply AWS endorsement.

`assets/icons/aws/NOTICE.txt` records ownership and source, and is embedded once in every diagram containing official icons.

## Rendering and mapping

The build script parses the bundled SVG files and emits reusable inline symbols.
Original geometry, groups, view boxes, gradients, clip paths and masks are preserved;
internal IDs and local references are deterministically namespaced per service.
An element/attribute allowlist rejects scripts, event handlers, CSS, images,
external references, duplicate IDs and unresolved local references at build time.
Rust's `include_str!` embeds the validated symbols, so no XML parser or asset
files are needed at runtime. Repeated services share one symbol. Cards and
container headers display the symbols at 24px beside the selected full address.
Named state panels additionally namespace all IDs and fragment references.

Selection is provider-aware and independent of ResourceRole. Managed/data entities share the same service icon. Related resources intentionally use service-level icons: subnet, gateway, routing, peering, and network security resources use Amazon VPC; listeners and target groups use Elastic Load Balancing. No public/private subnet status is inferred from the resource type. Unknown providers/types render without an icon.

## Updating

Download a release directly from AWS, select the original service SVGs, and update the manifest, notices, and release date together. Verify hashes, review the SVGs for external dependencies or executable content, regenerate all sample diagrams, and run Docker tests. Preserve the original files without recoloring or editing their paths.
