#!/bin/sh
set -eu
root=$(pwd)
work=$(mktemp -d)
server=
trap 'if [ -n "$server" ]; then kill "$server" 2>/dev/null || :; wait "$server" 2>/dev/null || :; fi; rm -rf "$work"' EXIT
node tests/support/sts_fixture.cjs "$work/port" &
server=$!
attempts=0
while [ ! -f "$work/port" ]; do
  kill -0 "$server" 2>/dev/null || { echo 'STS fixture exited before startup' >&2; exit 1; }
  attempts=$((attempts + 1))
  [ "$attempts" -lt 100 ] || { echo 'STS fixture startup timed out' >&2; exit 1; }
  sleep 0.1
done
export TF_VAR_sts_endpoint="http://127.0.0.1:$(cat "$work/port")"
cp tests/fixtures/provider-plan/main.tf "$work/main.tf"
cd "$work"
terraform init -backend=false -input=false -no-color > init.log
AWS_ACCESS_KEY_ID=fixture AWS_SECRET_ACCESS_KEY=fixture AWS_EC2_METADATA_DISABLED=true \
  terraform plan -refresh=false -input=false -no-color -out=saved.plan > plan.log
terraform show -json saved.plan > plan.json
"$root/target/debug/planorama" plan.json --provider-plan saved.plan -o verified.svg
grep -q 'AWS Account: 123456789012' verified.svg
"$root/target/debug/planorama" plan.json -o unknown.svg
if grep -q 'AWS Account: 123456789012' unknown.svg; then exit 1; fi
"$root/target/debug/planorama" --state first=plan.json --state second=plan.json \
  --state-provider-plan first=saved.plan -o states.svg
test "$(grep -o 'data-account-panel="true"' states.svg | wc -l)" -eq 1
node - <<'JS'
const fs = require('node:fs');
fs.writeFileSync('main.tf', fs.readFileSync('main.tf', 'utf8').replaceAll('  assume_role {', `  dynamic "assume_role" {
    for_each = ["arn:aws:iam::210987654321:role/Dynamic"]
    content { role_arn = assume_role.value }
  }
  assume_role {`));
JS
AWS_ACCESS_KEY_ID=fixture AWS_SECRET_ACCESS_KEY=fixture AWS_EC2_METADATA_DISABLED=true \
  terraform plan -refresh=false -input=false -no-color -out=dynamic.plan > dynamic.log
terraform show -json dynamic.plan > dynamic.json
"$root/target/debug/planorama" dynamic.json --provider-plan dynamic.plan -o dynamic.svg
if grep -q 'data-account-panel="true"' dynamic.svg; then exit 1; fi
if "$root/target/debug/planorama" plan.json --provider-plan dynamic.plan -o mismatch.svg 2> mismatch.log; then exit 1; fi
grep -q 'does not match' mismatch.log
printf 'Saved-plan CLI verification passed\n'
