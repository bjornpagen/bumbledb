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

echo "==> cargo miri test (native aarch64-apple-darwin)"

cargo miri test -p bumbledb --lib -- $FILTERS $SKIPS \
    --skip exec::kernel::tests::allen

echo "==> cargo miri test --target x86_64-unknown-linux-gnu (cross-interpreted)"

CC_x86_64_unknown_linux_gnu="$(pwd)/scripts/miri-cross-cc.sh" \
AR_x86_64_unknown_linux_gnu=ar \
cargo miri test -p bumbledb --lib --target x86_64-unknown-linux-gnu -- \
    $FILTERS $SKIPS

echo "miri lane green on both targets"
