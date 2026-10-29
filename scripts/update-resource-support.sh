#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$#" -gt 1 ]; then
  echo 'Usage: sh scripts/update-resource-support.sh [type-prefix]' >&2
  exit 2
fi
case "${1:-}" in
  ''|aws_*|terraform_*) export RESOURCE_SUPPORT_FILTER="${1:-}" ;;
  *) echo 'Expected an aws_ or terraform_ type prefix' >&2; exit 2 ;;
esac
cargo test --locked --lib provider::aws::resource_support::export_resource_support -- --ignored --exact
