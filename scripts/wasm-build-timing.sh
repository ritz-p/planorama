wasm_step() {
    phase=$1
    shift
    if [ -n "${WASM_BUILD_TIMINGS:-}" ]; then
        /usr/bin/time -f "$phase\t%e\t%U\t%S\t%M" -a -o "$WASM_BUILD_TIMINGS" "$@"
    else
        "$@"
    fi
}
