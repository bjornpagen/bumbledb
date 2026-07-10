#!/bin/sh
# Disassembly gates: machine-code
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

# no_flag_writers_inside SYMBOL_SUBSTR LABEL
#
# The flag-free law, structural (docs/architecture/40-execution.md, the
# configuration kernel; m2max.core.flag-port-asymmetry /
# m2max.core.flag-strand-mlp): the Allen hot path carries zero scalar
# flag-writing instructions — cmp/csel/adds/ccmp confine to the 3-port
# triad dense and halve gathered miss lanes (~28 -> ~14) — so the gate
# greps the kernel symbols' machine code for them (LLVM substitutes;
# the source proves nothing). Calls are forbidden too: a bl would
# launder a flag writer into another symbol.
no_flag_writers_inside() {
    sym="$1"; label="$2"
    if ! grep -qE "^[0-9a-f]+ <[^>]*${sym}[^>]*>:" "$DUMP"; then
        echo "check-asm: FAIL [$label] — no symbol matching '${sym}' in $BIN"
        FAIL=1
        return
    fi
    awk -v pat="$sym" '
        /^[0-9a-f]+ <.*>:/ { insym = (index($0, pat) != 0) }
        insym { print }
    ' "$DUMP" > "$SYM"
    if grep -E "[[:space:]](cmp|csel|adds|ccmp|bl)[[:space:]]" "$SYM" > "$BAD"; then
        echo "check-asm: FAIL [$label] — flag writers (or calls) inside '${sym}':"
        sed 's/^/  /' "$BAD" | head -8
        FAIL=1
    else
        echo "check-asm: ok   [$label] ${sym} free of scalar flag writers (cmp/csel/adds/ccmp)"
    fi
}

# --- The probe path is call-free per element (measured).
# The Descend arm may call run_node/pump/sink machinery; what may NOT
# appear is a call in the per-element probe class: the colt probe chain,
# the hash, and any runtime-length memory compare. `probe_walk17h`
# matches the const-generic hot monomorphs (arity 1–4) exactly;
# `probe_walk_general` is the deliberately-outlined arity>4 cold arm —
# a `bl` to it is dead weight for every real plan, like a panic call.
PROBE_CLASS='bcmp|memcmp|get_prehashed|probe_child_at|probe_hashed|probe_walk17h|hash_key|hash_words|position_matches|unpack_child|prefetch_bucket'
no_calls_inside "probe_pass" "$PROBE_CLASS" "probe probe_pass"
no_calls_inside "run_node"   "$PROBE_CLASS" "probe run_node"

# --- The sink row loops carry the const-arity insert
# chain fully inlined — no hash call, no runtime-length compare, no
# WordMap call ceremony per row. The dyn fallback (exotic widths) is
# deliberately outlined as entry_dyn_hashing — a call to IT is legal;
# calls to the hash or the general compare are not.
SINK_CLASS='bcmp|memcmp|hash_words|hash_core|get_or_insert|6insert17h|entry_core|entry_hashed_core|probe_with|key_at_matches'
no_calls_inside "emit_batch" "$SINK_CLASS" "sink emit_batch"

# --- The configuration kernel is flag-free (the Allen hot path:
# cmhi/cmeq predicate lanes, tbl nibble table, broadcast-mask tbl —
# never a scalar cmp/csel classify).
no_flag_writers_inside "allen_code_batch_neon"       "allen flag-free codes"
no_flag_writers_inside "allen_code_batch_const_neon" "allen flag-free codes-const"
no_flag_writers_inside "allen_filter_batch_neon"     "allen flag-free filter"

if [ "$FAIL" -ne 0 ]; then
    echo "check-asm: FAILURES (see above)"
    exit 1
fi
echo "check-asm: all gates green"
