#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --locked
binary=target/debug/planorama
for name in association aws-relationships checks components containment dense-architecture multi-container network-scope rds-subnet-group regions routes security-groups spanning-dense subnet-groups terraform vpc-workloads workload-security-groups; do
    "$binary" "tests/fixtures/$name-plan.json" -o "examples/$name.svg"
done
"$binary" examples/plan.json -o examples/diagram.svg
"$binary" examples/data-plan.json -o examples/data.svg
"$binary" examples/bundling-plan.json -o examples/bundling.svg
"$binary" examples/terraform-large/plan.json -o examples/terraform-large/diagram.svg
"$binary" --state network=tests/fixtures/multi/network.json --state application=tests/fixtures/multi/application.json -o examples/multi-plan.svg
"$binary" --state producer=tests/fixtures/multi/producer.json --state consumer=tests/fixtures/multi/consumer.json --remote-state consumer:data.terraform_remote_state.network=producer -o examples/cross-state.svg
