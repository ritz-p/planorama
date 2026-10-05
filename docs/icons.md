# AWS Architecture Icons

Planorama embeds official AWS Architecture Icons from the **07312026 (July 31, 2026)** package downloaded from the [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/) on 2026-10-05.

The selected original SVG files are stored unchanged under `assets/icons/aws/`. `manifest.json` records the official download URL, ZIP SHA-256, original entry paths, and individual file hashes. Only 17 required service icons are bundled; the full ZIP is not committed. Git preserves asset bytes through `.gitattributes`.

## Rights and scope

AWS owns these assets. They are **not MIT-licensed**. The official page permits customers and partners to use the assets in architecture diagrams; the [AWS Site Terms](https://aws.amazon.com/terms/) also apply. The downloaded package contains SVG/PNG assets and no separate license document. This is not a grant of unrestricted redistribution or sublicensing rights. The selected assets support this AWS architecture diagram renderer, not a standalone icon distribution service. For other uses, consult AWS's terms or obtain permission. Planorama does not imply AWS endorsement.

`assets/icons/aws/NOTICE.txt` records ownership and source, and is embedded once in every diagram containing official icons. The previous original-glyph MIT notice has been removed because it does not apply to these assets.

## Rendering and mapping

Rust's `include_str!` embeds the SVG files at build time. Each used asset is percent-encoded as an SVG data URI in an image inside a reusable symbol. This isolates internal IDs, styles, and gradients while preserving original colors, shapes, and aspect ratio. Repeated services share one symbol. There are no runtime downloads or external rendering dependencies. Cards and container headers display the symbols at 24px beside the selected full address.

Selection is provider-aware and independent of ResourceRole. Managed/data entities share the same service icon. Related resources intentionally use service-level icons: subnet, gateway, routing, peering, and network security resources use Amazon VPC; listeners and target groups use Elastic Load Balancing. No public/private subnet status is inferred from the resource type. Unknown providers/types render without an icon.

## Updating

Download a release directly from AWS, select the original service SVGs, and update the manifest, notices, and release date together. Verify hashes, review the SVGs for external dependencies or executable content, regenerate all sample diagrams, and run Docker tests. Preserve the original files without recoloring or editing their paths.
