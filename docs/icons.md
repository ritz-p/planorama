# Resource icons

The bundled artwork is **Planorama service glyphs v1 (2026-10-05)**: original
vector geometry authored in this repository, in `src/icons/artwork.rs`.
It is not the official AWS Architecture Icons set and does not reproduce or
bundle third-party artwork. AWS service names identify the resource mapping,
not endorsement. Colors identify broad service families, not Terraform actions.

The glyph artwork is available under the [MIT license](icons-LICENSE.txt).
This notice applies to the glyphs and their documentation, not as a change to
the license of the rest of the repository. Generated SVGs using glyphs embed
the full notice once in metadata, so exported diagrams carry it with them.

The provider-aware lookup in `src/icons.rs` is independent of ResourceRole.
The current plan model has no provider registry identity, so callers infer
the AWS provider from the exact `aws_` resource prefix. Other providers and
unmapped types return no icon. A future provider can supply its own mapping.
Managed resources and data sources with the same type share the same glyph.
The mapping covers the AWS types in the large example and classification table;
related resource types intentionally reuse service-family glyphs.

Each diagram defines only its used symbols, once each, and references them
with local `<use href="#...">` elements. There are no downloaded fonts, images,
remote links, or runtime network dependencies. Multiple paths and shapes per
symbol are supported. Symbols use a 24-by-24 view box and are shown at 24px
beside the first address line on ordinary cards and container headers.
Unknown resources retain their full text area. The selected address and
complete title remain independent of the icon.

Before adding any official or third-party artwork, record its exact source and
version, verify redistribution terms, and include all required notices. Do not
replace these glyphs with externally sourced paths without that review.
