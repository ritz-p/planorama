#!/bin/sh
set -eu
if [ "${CARGO_INCREMENTAL:-}" != 1 ] && command -v sccache >/dev/null 2>&1; then
    exec sccache "$@"
fi
exec "$@"
