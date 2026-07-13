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

# The former forbidden-call gates (probe_pass/run_node/emit_batch
# checked against a symbol-NAME regex of slow-path functions) were
# removed by owner ruling 2026-07-13: symbol names are a mangling
# artifact, not a machine-code property — nightly's v0 mangling
# false-positived on a closure spelling, and a gate that fails on
# spelling is ceremony. The inlining discipline those gates watched is
# owned by the #[ignore]d microbench pins, which measure the thing
# itself. The gates below assert instruction classes, never names.

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
