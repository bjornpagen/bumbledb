#!/bin/sh
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
    if grep -E "[[:space:]](cmp|cmn|ccmp|ccmn|tst|adds|adcs|subs|sbcs|ands|bics|negs|ngcs|csel|fcmp|fccmp|bl|blr)[[:space:]]|[[:space:]]b\.[a-z]{2}[[:space:]]" "$SYM" > "$BAD"; then
        echo "check-asm: FAIL [$label] — flag writers (or b.cond/calls) inside '${sym}':"
        sed 's/^/  /' "$BAD" | head -8
        FAIL=1
    else
        echo "check-asm: ok   [$label] ${sym} free of scalar flag writers (the NZCV class, csel, b.cond, calls)"
    fi
}

no_flag_writers_inside "allen_code_batch_neon"       "allen flag-free codes"
no_flag_writers_inside "allen_code_batch_const_neon" "allen flag-free codes-const"
no_flag_writers_inside "allen_filter_batch_neon"     "allen flag-free filter"

if [ "$FAIL" -ne 0 ]; then
    echo "check-asm: FAILURES (see above)"
    exit 1
fi
echo "check-asm: all gates green"
