#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
version=6.70.0
commit=5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f
output_dir=/tmp/aws-inventory-schema
case "$#" in
  0) ;;
  3) version=$1; commit=$2; output_dir=$3 ;;
  *) echo 'Usage: sh scripts/prepare-aws-inventory.sh [VERSION COMMIT OUTPUT_DIRECTORY]' >&2; exit 2 ;;
esac
node -e 'if (!/^\d+\.\d+\.\d+$/.test(process.argv[1]) || !/^[a-f0-9]{40}$/.test(process.argv[2])) process.exit(1)' "$version" "$commit"
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)
if [ "$#" -ne 0 ] && [ -n "$(ls -A "$output_dir")" ]; then
  echo 'Output directory must be empty for an explicit version' >&2
  exit 2
fi
source_dir=$(mktemp -d /tmp/planorama-aws-source.XXXXXX)
trap 'rm -rf "$source_dir"' EXIT HUP INT TERM
curl -fsSL "https://api.github.com/repos/hashicorp/terraform-provider-aws/tarball/$commit" -o "$source_dir/source.tar.gz"
tar xzf "$source_dir/source.tar.gz" --strip-components=1 -C "$source_dir"
cat > "$output_dir/main.tf" <<EOF
terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "$version"
    }
  }
}
EOF
terraform -chdir="$output_dir" init -backend=false -input=false
terraform -chdir="$output_dir" providers schema -json > "$output_dir/schema.json"
node scripts/aws-inventory.cjs "$source_dir" "$output_dir/schema.json" "$output_dir/inventory.json" "$version" "$commit"
if [ "$#" -eq 0 ]; then
  cmp tests/fixtures/aws-inventory/6.70.0.json "$output_dir/inventory.json"
fi
