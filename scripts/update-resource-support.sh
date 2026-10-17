#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
case "${1:-}" in
  '') export UPDATE_RESOURCE_SUPPORT=1 ;;
  --check) unset UPDATE_RESOURCE_SUPPORT ;;
  *) echo 'Usage: sh scripts/update-resource-support.sh [--check]' >&2; exit 2 ;;
esac
cargo test --locked --bin planorama provider::aws::resource_support::generated_resource_support_is_current -- --exact
