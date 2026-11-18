#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$#" -ne 1 ] || ! printf '%s\n' "$1" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo 'Usage: sh scripts/audit-cloudflare-provider.sh X.Y.Z' >&2
    exit 2
fi
case "$1" in *[!0-9.]*) echo "Expected a numeric provider version" >&2; exit 2 ;; esac
candidate=$1
directory=$(mktemp -d)
trap 'rm -rf "$directory"' EXIT
trap 'exit 1' HUP INT TERM
capture() {
    location=$1
    release=$2
    mkdir "$location"
    cat > "$location/main.tf" <<TF
terraform {
  required_version = "= 1.9.8"
  required_providers {
    cloudflare = {
      source = "cloudflare/cloudflare"
      version = "= $release"
    }
  }
}
TF
    TF_DATA_DIR="$location/.terraform" terraform -chdir="$location" init -backend=false -input=false >&2
    TF_DATA_DIR="$location/.terraform" terraform -chdir="$location" providers schema -json > "$location/schema.json"
    TF_DATA_DIR="$location/.terraform" terraform -chdir="$location" version -json > "$location/version.json"
}
capture "$directory/pinned" 5.28.1
capture "$directory/candidate" "$candidate"
sh scripts/update-resource-support.sh cloudflare_ >&2
node scripts/cloudflare-inventory-diff.cjs "$directory/pinned/schema.json" "$directory/candidate/schema.json" \
    "$directory/candidate/version.json" output/resource-support.md
