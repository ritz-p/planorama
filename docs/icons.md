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
