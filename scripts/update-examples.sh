#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --locked --bin planorama
binary=${CARGO_TARGET_DIR:-target}/debug/planorama

update_snapshot() {
    snapshot_test=$1
    snapshot_flag=$2
    listing=$(cargo test --locked --lib "$snapshot_test" -- --exact --list)
    if ! printf '%s\n' "$listing" | grep -F -x "$snapshot_test: test" > /dev/null; then
        printf 'Snapshot test not found: %s\n' "$snapshot_test" >&2
        exit 1
    fi
    env "$snapshot_flag=1" cargo test --locked --lib "$snapshot_test" -- --exact
}

while IFS= read -r output || [ -n "$output" ]; do
    case "$output" in
        examples/region-overrides.svg)
            "$binary" tests/fixtures/aws-region-overrides/plan.json -o "$output"
            ;;
        examples/relationship-legend-hidden.svg)
            "$binary" tests/fixtures/routes-plan.json --hide-relationship-legend -o "$output"
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
            cargo run --locked --example generate_multi_fixture
            cargo build --locked --release --bin planorama
            sh scripts/render-multi-account.sh "${CARGO_TARGET_DIR:-target}/release/planorama" svg "$output"
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
            update_snapshot layout::containers::tests::hierarchy_obstacles_and_direct_container_endpoints UPDATE_ROUTING_SNAPSHOT
            ;;
        examples/port-collisions.svg)
            update_snapshot layout::tests::ports::refined_flat_bundle_keeps_opposite_role_port UPDATE_PORT_SNAPSHOT
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
