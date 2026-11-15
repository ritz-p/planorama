# Anonymized diagram exports

`--anonymize` is an explicit, off-by-default full export mode for SVG and
self-contained HTML. Rust `Options.anonymize` and Wasm `{ "anonymize": true }`
use the same projection, including named inputs and resumable generation.

```sh
planorama plan.json --anonymize -o anonymous.svg
planorama plan.json --anonymize --format html -o anonymous.html
planorama --state environment=plan.json --anonymize --format html -o anonymous.html
```

The [synthetic example](../examples/anonymized.svg) is generated from
[anonymization-plan.json](../tests/fixtures/anonymization-plan.json).

## Output contract

After relationship inference and selection, a separate display projection replaces
resource addresses/names, nested module instances, named states, moved/deposed
identities, account/Zone identifiers, provider aliases and configuration keys with
human-readable pseudonyms. Route destinations, including IP/CIDR and prefix-list
identifiers, are pseudonyms in SVG; HTML continues to omit destination constants.
Unknown provider/type/region strings are pseudonymized. Known public resource types,
service/category labels and recognized public region names remain visible.

Equality is preserved throughout one export, including references, source metadata,
DOM IDs, links, tooltips, search and navigation. Resource identities include their
named state, so the same address in different states remains distinct. Nested
module hierarchy, node/relationship kinds, containment and lifecycle actions are
preserved. Text is measured after projection to fit pseudonyms; geometry need not
match an ordinary export. Original input/semantic graphs are not rewritten.

Mappings are deterministic for the same ordered input and settings. They are
scoped to one generation, not stable identifiers across different exports,
selections or engine versions. The mapping is never embedded in the output.
There are no partial account-only/IP-only masking options.

Arbitrary free text is withheld, including change/replace/relevance paths and
action-reason text. Check statuses outside the known vocabulary are withheld.
Original focus/module selectors and remote-state mapping settings are omitted
from HTML generation metadata; selected-view status is retained. Evaluated
attribute values such as resource IDs, URLs and ARNs are not added to the output.
Identity-bearing values that already enter presentation are replaced or withheld.

## Boundaries

This option anonymizes successful SVG/HTML output, not the source plan JSON,
terminal errors, filenames, inspection APIs or module/resource inventories.
Original plan/state files may contain secrets. Local processing does not itself
make a download anonymous. Choose a neutral output filename and review the diagram
before sharing: topology, counts, resource types, actions and retained regions can
still disclose information or help identify an environment. Pseudonyms are not
an encryption format or a promise that an architecture cannot be reidentified.

Diagnostic/coverage/layout reports, embedded HTML diagnostics, timing output and
`--list-modules` cannot be combined with anonymization. The command/API rejects
the combination before writing a diagram or report. Run those separately and
treat their output as sensitive. Ordinary generation is unchanged when this
option is omitted; `--no-anonymize` overrides a TOML `anonymize = true` setting.
