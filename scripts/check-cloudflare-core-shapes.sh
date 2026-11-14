#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$#" -gt 1 ] || { [ "$#" -eq 1 ] && [ "$1" != --write ]; }; then
    echo 'Usage: sh scripts/check-cloudflare-core-shapes.sh [--write]' >&2
    exit 2
fi
test "$(terraform version -json | node -e 'let s="";process.stdin.on("data",x=>s+=x).on("end",()=>console.log(JSON.parse(s).terraform_version))')" = 1.9.8
directory=$(mktemp -d /tmp/planorama-cloudflare-core.XXXXXX)
trap 'rm -rf "$directory"' EXIT HUP INT TERM
cp tests/fixtures/cloudflare-core-shapes/main.tf "$directory/main.tf"
terraform -chdir="$directory" init -backend=false -input=false > "$directory/init.log"
terraform -chdir="$directory" plan -refresh=false -input=false -out=plan.bin > "$directory/plan.log"
terraform -chdir="$directory" show -json plan.bin > "$directory/plan.json"
node scripts/cloudflare-core-shapes.cjs "$directory/plan.json" "$@"
