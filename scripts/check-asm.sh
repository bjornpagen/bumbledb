#!/bin/sh
# Disassembly gates (docs/silicon/README.md rule 7): machine-code
# properties of hot symbols, asserted mechanically. The gate is the
# machine code, not the source — an #[inline(always)] that stopped
# working fails here, not in review.
#
# Usage: scripts/check-asm.sh [binary]
# Default binary: target/release/bumbledb-bench (build it first).
set -eu

cd "$(dirname "$0")/.."

BIN="${1:-target/release/bumbledb-bench}"
if [ ! -f "$BIN" ]; then
    echo "check-asm: no binary at $BIN (cargo build -p bumbledb-bench --release)" >&2
    exit 2
fi

DUMP="$(mktemp /tmp/bumbledb-asm.XXXXXX)"
SYM="$(mktemp /tmp/bumbledb-asm-sym.XXXXXX)"
BAD="$(mktemp /tmp/bumbledb-asm-bad.XXXXXX)"
trap 'rm -f "$DUMP" "$SYM" "$BAD"' EXIT INT TERM

objdump -d "$BIN" > "$DUMP"

FAIL=0

# no_calls_inside SYMBOL_SUBSTR FORBIDDEN_TARGET_REGEX LABEL
#
# Asserts that no `bl`/`b` line inside any symbol whose name contains
# SYMBOL_SUBSTR targets a symbol matching FORBIDDEN_TARGET_REGEX. Every
# monomorphization is checked (the awk toggles per symbol header).
no_calls_inside() {
    sym="$1"; forbidden="$2"; label="$3"
    if ! grep -qE "^[0-9a-f]+ <[^>]*${sym}[^>]*>:" "$DUMP"; then
        echo "check-asm: FAIL [$label] — no symbol matching '${sym}' in $BIN"
        FAIL=1
        return
    fi
    awk -v pat="$sym" '
        /^[0-9a-f]+ <.*>:/ { insym = (index($0, pat) != 0) }
        insym { print }
    ' "$DUMP" > "$SYM"
    if grep -E "\bbl?[[:space:]].*<[^>]*(${forbidden})[^>]*>" "$SYM" | grep -vE "b\.[a-z]+" > "$BAD"; then
        echo "check-asm: FAIL [$label] — forbidden calls inside '${sym}':"
        sed 's/^/  /' "$BAD" | head -8
        FAIL=1
    else
        echo "check-asm: ok   [$label] ${sym} free of (${forbidden})"
    fi
}

# --- PRD 02 (docs/silicon/02): the probe path is call-free per element.
# The Descend arm may call run_node/pump/sink machinery; what may NOT
# appear is a call in the per-element probe class: the colt probe chain,
# the hash, and any runtime-length memory compare. `probe_walk17h`
# matches the const-generic hot monomorphs (arity 1–4) exactly;
# `probe_walk_general` is the deliberately-outlined arity>4 cold arm —
# a `bl` to it is dead weight for every real plan, like a panic call.
PROBE_CLASS='bcmp|memcmp|get_prehashed|probe_child_at|probe_hashed|probe_walk17h|hash_key|hash_words|position_matches|unpack_child|prefetch_bucket'
no_calls_inside "probe_pass" "$PROBE_CLASS" "prd02 probe_pass"
no_calls_inside "run_node"   "$PROBE_CLASS" "prd02 run_node"

if [ "$FAIL" -ne 0 ]; then
    echo "check-asm: FAILURES (see above)"
    exit 1
fi
echo "check-asm: all gates green"
