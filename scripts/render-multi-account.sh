#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
binary=${1:-${CARGO_TARGET_DIR:-target}/release/planorama}
format=${2:-svg}
output=${3:-examples/multi-account/diagram.$format}
"$binary" \
  --state application-a=examples/multi-account/application-a.json \
  --state application-b=examples/multi-account/application-b.json \
  --state shared-services=examples/multi-account/shared-services.json \
  --remote-state application-a:module.application.data.terraform_remote_state.shared=shared-services \
  --remote-state application-b:module.application.data.terraform_remote_state.shared=shared-services \
  --remote-state application-b:data.terraform_remote_state.peer=application-a \
  --format "$format" -o "$output"
