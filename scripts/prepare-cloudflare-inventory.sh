#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test "$(terraform version -json | node -e 'let s="";process.stdin.on("data",x=>s+=x).on("end",()=>console.log(JSON.parse(s).terraform_version))')" = 1.9.8
version=5.28.1
commit=9b51a582458f5c5d46d8547ed2604fcba91fb861
directory=$(mktemp -d /tmp/planorama-cloudflare-inventory.XXXXXX)
trap 'rm -rf "$directory"' EXIT HUP INT TERM
mkdir "$directory/source" "$directory/schema"
curl -fsSL "https://api.github.com/repos/cloudflare/terraform-provider-cloudflare/tarball/$commit" -o "$directory/source.tar.gz"
tar xzf "$directory/source.tar.gz" --strip-components=1 -C "$directory/source"
cat > "$directory/schema/main.tf" <<EOF
terraform {
  required_providers {
    cloudflare = {
      source = "cloudflare/cloudflare"
      version = "$version"
    }
  }
}
EOF
terraform -chdir="$directory/schema" init -backend=false -input=false
terraform -chdir="$directory/schema" providers schema -json > "$directory/schema/schema.json"
node scripts/cloudflare-inventory.cjs "$directory/source" "$directory/schema/schema.json" "$directory/inventory.json"
cmp tests/fixtures/cloudflare-inventory/5.28.1.json "$directory/inventory.json"
