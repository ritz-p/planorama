#!/bin/sh
set -eu

if [ "$(uname -s)/$(uname -m)" != Linux/x86_64 ]; then
    echo 'mold setup is limited to Linux x86_64; keeping the system linker'
    exit 0
fi
prefix=${1:-/usr/local}
archive=$(mktemp)
trap 'rm -f "$archive"' EXIT HUP INT TERM
curl --fail --location --retry 3 https://github.com/rui314/mold/releases/download/v2.40.4/mold-2.40.4-x86_64-linux.tar.gz -o "$archive"
echo "4c999e19ffa31afa5aa429c679b665d5e2ca5a6b6832ad4b79668e8dcf3d8ec1  $archive" | sha256sum -c -
mkdir -p "$prefix"
tar -xzf "$archive" --strip-components=1 -C "$prefix"
"$prefix/bin/mold" --version
