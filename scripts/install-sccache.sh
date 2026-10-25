#!/bin/sh
set -eu
if [ "$(uname -s)/$(uname -m)" != Linux/x86_64 ]; then
    echo 'sccache setup is limited to Linux x86_64; keeping rustc directly'
    exit 0
fi
prefix=${1:-/usr/local}
directory=$(mktemp -d)
trap 'rm -rf "$directory"' EXIT HUP INT TERM
archive="$directory/sccache.tar.gz"
curl --fail --location --retry 3 https://github.com/mozilla/sccache/releases/download/v0.10.0/sccache-v0.10.0-x86_64-unknown-linux-musl.tar.gz -o "$archive"
echo "1fbb35e135660d04a2d5e42b59c7874d39b3deb17de56330b25b713ec59f849b  $archive" | sha256sum -c -
tar -xzf "$archive" -C "$directory"
mkdir -p "$prefix/bin"
install -m 755 "$directory/sccache-v0.10.0-x86_64-unknown-linux-musl/sccache" "$prefix/bin/sccache"
"$prefix/bin/sccache" --version
