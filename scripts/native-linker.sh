#!/bin/sh
set -eu
case "${PLANORAMA_LINKER:-mold}" in
    system) ;;
    mold) set -- -fuse-ld=mold -Wl,--no-fork "$@" ;;
    *) echo 'PLANORAMA_LINKER must be system or mold' >&2; exit 2 ;;
esac
if [ -n "${PLANORAMA_LINK_TIMINGS:-}" ]; then
    exec /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$PLANORAMA_LINK_TIMINGS/$$.tsv" cc "$@"
fi
exec cc "$@"
