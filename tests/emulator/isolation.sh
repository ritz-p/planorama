#!/bin/sh
set -eu
umask 077
test_dir=$(mktemp -d)
trap 'rm -rf -- "$test_dir"' EXIT
mkdir "$test_dir/tmp"
cat >"$test_dir/fail-render" <<'RENDER'
#!/bin/sh
set -eu
[ "$(ip -o link show | cut -d: -f2 | tr -d ' ')" = lo ]
[ "$AWS_ENDPOINT_URL" = https://127.0.0.1:9297 ]
[ -z "${AWS_ACCESS_KEY_ID:-}${AWS_SECRET_ACCESS_KEY:-}${AWS_SESSION_TOKEN:-}${AWS_PROFILE:-}" ]
[ "$AWS_CONFIG_FILE" = "$HOME/.aws/emulator/9297/config" ]
[ -s "$AWS_SHARED_CREDENTIALS_FILE" ] && [ -s "$AWS_CA_BUNDLE" ]
dirname "$1" >"$(dirname "$0")/partial-output"
exit 19
RENDER
chmod 700 "$test_dir/fail-render"
if TMPDIR="$test_dir/tmp" PLANORAMA_BIN="$test_dir/fail-render" \
    AWS_PROFILE=synthetic-test-profile AWS_ACCESS_KEY_ID=synthetic-test-id \
    AWS_SECRET_ACCESS_KEY=synthetic-test-key AWS_SESSION_TOKEN=synthetic-test-token \
    sh scripts/aws-emulator/generate.sh containment --create-only >"$test_dir/log"; then
    echo 'Injected render failure did not fail the capture' >&2
    exit 1
fi
test -s "$test_dir/partial-output"
partial=$(cat "$test_dir/partial-output")
test ! -e "$partial"
test -z "$(ls -A "$test_dir/tmp")"
