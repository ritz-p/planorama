#!/bin/sh
set -eu
repo=$1 tools=$2 binary=$3 scratch=$4 artifact=$5 sample=$6
[ "$(ip -o link show | cut -d: -f2 | tr -d ' ')" = lo ] || { echo 'Capture requires a loopback-only network namespace' >&2; exit 1; }
if ! ip -o link show lo | grep -q '<[^>]*UP'; then ip link set lo up; fi
cd "$scratch"
mkdir module
cp "$repo/examples/aws-emulator/$sample/main.tf" "$repo/examples/aws-emulator/queue/.terraform.lock.hcl" module/
export TF_CLI_CONFIG_FILE="$scratch/terraform.rc" TF_IN_AUTOMATION=1 CHECKPOINT_DISABLE=1
cat >"$TF_CLI_CONFIG_FILE" <<CONF
provider_installation {
  filesystem_mirror { path = "$tools/mirror" }
}
CONF
export AWS_REGION=us-east-1 AWS_DEFAULT_REGION=us-east-1 AWS_EC2_METADATA_DISABLED=true
export AWS_ENDPOINT_URL=https://127.0.0.1:9297
export AWS_CA_BUNDLE="$HOME/.aws/emulator/9297/ca.pem"
export AWS_CONFIG_FILE="$HOME/.aws/emulator/9297/config"
export AWS_SHARED_CREDENTIALS_FILE="$HOME/.aws/emulator/9297/emulator_credentials"
"$tools/aws-emulator" --version | grep -Fx 'Version: 1.0.508'
"$tools/aws-emulator" --bind 127.0.0.1 --port 9297 --auth on --telemetry off >emulator.log 2>&1 &
pid=$!
stop_emulator() {
    if kill -0 "$pid" 2>/dev/null; then kill "$pid"; fi
    wait "$pid" || {
        status=$?
        [ "$status" -eq 143 ] || exit "$status"
    }
}
trap stop_emulator EXIT
trap 'exit 1' HUP INT TERM
for attempt in $(seq 1 60); do
    if ! kill -0 "$pid" 2>/dev/null; then echo 'Emulator exited before readiness' >&2; exit 1; fi
    if [ -s "$AWS_CA_BUNDLE" ] && curl -fsS --max-time 2 --cacert "$AWS_CA_BUNDLE" "$AWS_ENDPOINT_URL/__admin/health" >/dev/null 2>&1; then break; fi
    sleep 1
done
curl -fsS --max-time 2 --cacert "$AWS_CA_BUNDLE" "$AWS_ENDPOINT_URL/__admin/health" >/dev/null
tf() { timeout 5m "$tools/terraform" -chdir=module "$@"; }
tf init -lockfile=readonly -input=false -no-color
phases="create refreshed destroy"
if [ "$sample" = queue ]; then phases="create refreshed update destroy"; fi
for phase in $phases; do
    set --
    case $phase in update) set -- -var=delay_seconds=5;; destroy) set -- -destroy;; esac
    tf plan -input=false -no-color -out="$artifact/$phase.tfplan" "$@"
    tf show -json "$artifact/$phase.tfplan" >"$artifact/$phase.json"
    for format in svg html; do
        "$binary" "$artifact/$phase.json" --format "$format" -o "$artifact/$phase.$format"
    done
    if [ "$phase" = create ] || [ "$phase" = destroy ]; then
        tf apply -input=false -no-color "$artifact/$phase.tfplan"
    fi
done
node "$repo/tests/emulator/$sample.cjs" "$artifact"
node "$repo/scripts/aws-emulator/check-artifacts.cjs" "$artifact" "$AWS_SHARED_CREDENTIALS_FILE"
