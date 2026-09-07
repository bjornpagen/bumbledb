#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

FILTERS="allen::tests:: interval::tests:: interval::sweep:: \
encoding::tests:: schema::tests::member_set exec::kernel::tests:: \
exec::wordmap:: exec::colt::tests::synthetic:: \
ir::normalize::fold::tests:: arena:: digest::"

SKIPS="--skip exhaustive_ \
--skip false_tag_rate_stays --skip a_single_multiply_hash \
--skip probe_steps_stay_near_one \
--skip iteration_is_dense_and_insertion_ordered \
--skip a_covering_hint_never_grows"

case "${1:-all}" in
    all) TARGETS="aarch64-apple-darwin x86_64-unknown-linux-gnu" ;;
    aarch64-apple-darwin|x86_64-unknown-linux-gnu) TARGETS="$1" ;;
    *) echo "usage: miri.sh [aarch64-apple-darwin|x86_64-unknown-linux-gnu]" >&2; exit 2 ;;
esac
[ "$#" -le 1 ] || { echo "miri.sh: expected at most one target" >&2; exit 2; }

for target in $TARGETS; do
    echo "==> cargo miri test --target $target"
    case "$target" in
        aarch64-apple-darwin)
            cargo miri test -p bumbledb --lib --target "$target" -- \
                $FILTERS $SKIPS --skip exec::kernel::tests::allen
            ;;
        x86_64-unknown-linux-gnu)
            CC_x86_64_unknown_linux_gnu="$(pwd)/scripts/miri-cross-cc.sh" \
            AR_x86_64_unknown_linux_gnu=ar \
            cargo miri test -p bumbledb --lib --target "$target" -- $FILTERS $SKIPS
            ;;
    esac
    echo "miri lane green: $target"
done
