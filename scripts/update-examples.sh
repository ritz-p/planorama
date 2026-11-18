#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --locked --release --bin planorama --example generate_multi_fixture
release=${CARGO_TARGET_DIR:-target}/release
binary=$release/planorama
set --

while IFS= read -r output || [ -n "$output" ]; do
    case "$output" in
        examples/cloudflare-administration.svg)
            "$binary" tests/fixtures/cloudflare-administration-plan.json --cloudflare-groups -o "$output"
            ;;
        examples/cloudflare-imported.svg)
            "$binary" tests/fixtures/cloudflare-imported-plan.json --cloudflare-value-matches -o "$output"
            ;;
        examples/containment.svg)
            "$binary" examples/aws-emulator/containment/create.json -o "$output"
            ;;
        examples/anonymized.svg)
            "$binary" tests/fixtures/anonymization-plan.json --anonymize -o "$output"
            ;;
        examples/encryption-service-names.svg)
            "$binary" tests/fixtures/encryption-plan.json --icon-mode service-name -o "$output"
            ;;
        examples/aws-aliases-service-names.svg)
            "$binary" tests/fixtures/aws-aliases-plan.json --icon-mode service-name -o "$output"
            ;;
        examples/aws-basic-service-names.svg)
            "$binary" tests/fixtures/aws-basic-plan.json --icon-mode service-name -o "$output"
            ;;
        examples/organization-deployments.svg)
            set -- "$@" provider::aws::organization_accounts::tests::verified_deployment_example
            ;;
        examples/region-overrides.svg)
            "$binary" tests/fixtures/compatibility/1.9.8/overrides/plan.json -o "$output"
            ;;
        examples/cloudflare-captured-create.svg|examples/cloudflare-captured-update.svg)
            name=${output#examples/cloudflare-captured-}
            "$binary" "tests/fixtures/cloudflare-captured/${name%.svg}.json" -o "$output"
            ;;
        examples/relationship-legend-hidden.svg)
            "$binary" tests/fixtures/routes-plan.json --hide-relationship-legend -o "$output"
            ;;
        examples/relationship-legend-omitted.svg)
            "$binary" tests/fixtures/routes-plan.json --omit-relationship-legend -o "$output"
            ;;
        examples/service-names.svg)
            "$binary" --state services=tests/fixtures/data-icons-plan.json --state builtin=tests/fixtures/terraform-plan.json --icon-mode service-name -o "$output"
            ;;
        examples/diagram.svg)
            "$binary" examples/plan.json -o "$output" --coverage-report examples/coverage.md
            ;;
        examples/data.svg)
            "$binary" examples/data-plan.json -o "$output"
            ;;
        examples/bundling.svg)
            "$binary" examples/bundling-plan.json -o "$output"
            ;;
        examples/terraform-large/diagram.svg)
            "$binary" examples/terraform-large/plan.json -o "$output"
            ;;
        examples/multi-account/diagram.svg)
            "$release/examples/generate_multi_fixture"
            sh scripts/render-multi-account.sh "$binary" svg "$output"
            ;;
        examples/multi-plan.svg)
            "$binary" --state network=tests/fixtures/multi/network.json --state application=tests/fixtures/multi/application.json -o "$output"
            ;;
        examples/cross-state.svg)
            "$binary" --state producer=tests/fixtures/multi/producer.json --state consumer=tests/fixtures/multi/consumer.json --remote-state consumer:data.terraform_remote_state.network=producer -o "$output"
            ;;
        examples/regional-states.svg)
            "$binary" --state network=tests/fixtures/multi/network.json --state application=tests/fixtures/multi/application.json --state singleton=tests/fixtures/multi/single-region.json -o "$output"
            ;;
        examples/hierarchy-routing.svg)
            set -- "$@" layout::containers::tests::hierarchy_obstacles_and_direct_container_endpoints
            ;;
        examples/port-collisions.svg)
            set -- "$@" layout::tests::ports::refined_flat_bundle_keeps_opposite_role_port
            ;;
        examples/*.svg)
            name=${output#examples/}
            name=${name%.svg}
            case "$name" in
                ''|*[!a-z0-9-]*)
                    printf 'Unsupported generated SVG path: %s\n' "$output" >&2
                    exit 1
                    ;;
            esac
            "$binary" "tests/fixtures/$name-plan.json" -o "$output"
            ;;
        *)
            printf 'Unsupported generated SVG path: %s\n' "$output" >&2
            exit 1
            ;;
    esac
done < scripts/generated-svgs.txt

if [ "$#" -gt 0 ]; then
    listing=$(cargo test --locked --lib -- --exact --list "$@")
    for test_name do
        if ! printf '%s\n' "$listing" | grep -Fx "$test_name: test" >/dev/null; then
            printf 'Snapshot test not found: %s\n' "$test_name" >&2
            exit 1
        fi
    done
    UPDATE_ORGANIZATION_DEPLOYMENTS=1 UPDATE_ROUTING_SNAPSHOT=1 UPDATE_PORT_SNAPSHOT=1 \
        cargo test --locked --lib -- --exact "$@"
fi
