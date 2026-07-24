#!/bin/sh
# The Miri lane (the crucible packet (git ecec1dc3)): the PURE
# modules' unit tests interpreted under Miri, natively
# (aarch64-apple-darwin) AND cross-interpreted against
# x86_64-unknown-linux-gnu — Miri interprets foreign targets, so the
# cross pass checks endianness/width assumptions in the scalar kernels
# for free. Both passes must be green. `cargo miri setup` runs once
# implicitly on first use.
#
# Scope: the honest FFI boundary. Miri cannot cross a foreign-function
# call, so the lane is the pure modules only:
#   allen::tests            the Allen algebra and the scalar classifier
#   interval::tests/sweep   the interval parse and the sweep (pure Ord)
#   encoding::tests         the canonical codecs (blake3's portable
#                           body interprets fine under Miri)
#   schema::tests::member_set     the closed-target bitset judgment
#   exec::kernel::tests     the portable std::simd kernels and their
#                           scalar reference twins (Miri interprets
#                           std::simd)
#   exec::wordmap           the ctrl-byte probe structure and the SWAR
#                           leaf primitives (exec/swar.rs, pure u64
#                           arithmetic, exercised through these probes)
#   ir::normalize::fold::tests    condition folding over encoded words
#   arena                   the bump arena (no unsafe today; interpreted
#                           here so any future unsafe growth in exactly
#                           the module shape that grows one is covered)
#   digest                  the streaming digest (blake3's portable
#                           body, same footing as encoding::tests)
#
# Exclusions, each with its reason:
#   * every Db/Environment/TempDir-touching test module (api::,
#     image::, storage::, verify_store::, exec::run, exec::sink,
#     exec::dispatch, exec::colt, exec::introspection, ir::normalize::tests,
#     ir::validate/render (schema fixtures only, but they sit beside
#     Db-touching siblings and add no pure kernel coverage), plan::,
#     the tests/ integration binaries) — FFI: they open
#     LMDB environments through heed/lmdb-sys, and Miri cannot
#     interpret the mdb_* foreign calls. colt's probe logic is pure,
#     but its test fixtures build real stores, so it is out with the
#     rest; its shared SWAR primitives are covered via exec::wordmap.
#   * the ts bridge crate (ts/crate) runs on NO Miri lane — its unsafe
#     is napi-boundary pointer laundering (lib.rs, marshal.rs), the
#     same foreign wall as mdb_*, and its Rust tests open real LMDB
#     stores. Refereed instead by the CI sdk lane: ts/crate's own
#     `cargo test` (fingerprint_lock.rs) and the SDK node-test suite.
#   * --skip exec::kernel::tests::allen on the NATIVE pass only —
#     non-interpretable intrinsics, the same wall as FFI: on aarch64
#     the Allen configuration kernel dispatches to the hand-NEON
#     module (the one sanctioned unsafe module), and Miri does not
#     interpret NEON intrinsics. The scalar classify twin still runs
#     natively via allen::tests, and the CROSS pass below RUNS the
#     kernel tests: on x86_64 the dispatch takes the scalar reference
#     twins, so the whole Allen kernel surface is interpreted there.
#   * --skip exhaustive_ on both passes — budget, not FFI: the
#     exhaustive enumerations (8,192 masks x 784 pairs; 834 x 269
#     bitset cells; the all-pairs encoding domains) are proven by
#     native `cargo test`; under the interpreter their representative_*
#     subsets run instead (the crucible packet (git ecec1dc3)).
#   * the five wordmap SCALE contracts (false_tag_rate_stays,
#     a_single_multiply_hash, probe_steps_stay_near_one,
#     iteration_is_dense_and_insertion_ordered,
#     a_covering_hint_never_grows) — budget, not FFI: they fill
#     16k–100k-entry maps to pin statistical/load/capacity contracts
#     (proven by native `cargo test`), tens of interpreter-minutes for
#     zero new code paths; the same insert/probe/grow/iterate logic
#     runs here through the small behavior tests and the 7-arity
#     differential model test (itself scaled to 256 ops per round
#     under cfg!(miri) — growth from every hint shape still covered).
#   * fold_throughput_contiguous_sum and the wordmap pins microbench
#     are #[ignore] (timing evidence, by-hand only) and never run here.
set -eu

cd "$(dirname "$0")/.."

FILTERS="allen::tests:: interval::tests:: interval::sweep:: \
encoding::tests:: schema::tests::member_set exec::kernel::tests:: \
exec::wordmap:: ir::normalize::fold::tests:: arena:: digest::"

SKIPS="--skip exhaustive_ \
--skip false_tag_rate_stays --skip a_single_multiply_hash \
--skip probe_steps_stay_near_one \
--skip iteration_is_dense_and_insertion_ordered \
--skip a_covering_hint_never_grows"

echo "==> cargo miri test (native aarch64-apple-darwin)"
# shellcheck disable=SC2086  # FILTERS/SKIPS are deliberate word lists
cargo miri test -p bumbledb --lib -- $FILTERS $SKIPS \
    --skip exec::kernel::tests::allen

echo "==> cargo miri test --target x86_64-unknown-linux-gnu (cross-interpreted)"
# lmdb-master-sys's build script compiles LMDB's C for the requested
# target, and this host has no linux cross toolchain. Under Miri that
# staticlib is a build-graph artifact only — cargo-miri never links a
# native binary, and the filter list never calls into LMDB (an mdb_*
# call IS the FFI wall this lane is scoped around) — so a host-arch
# stand-in compile satisfies the graph; blake3's x86-64 assembly units
# are stubbed to empty objects, dead under Miri by blake3's own
# cfg!(miri) dispatch guards (scripts/miri-cross-cc.sh carries the
# full rationale). Without the stand-in, blake3's build script finds
# no x86_64 cc and falls back to its portable Rust body — which is why
# a machine whose build cache predates this env var stays green while
# a fresh runner, probing successfully THROUGH the shim, was handed
# the assembly and turned the nightly cron red.
# shellcheck disable=SC2086
CC_x86_64_unknown_linux_gnu="$(pwd)/scripts/miri-cross-cc.sh" \
AR_x86_64_unknown_linux_gnu=ar \
cargo miri test -p bumbledb --lib --target x86_64-unknown-linux-gnu -- \
    $FILTERS $SKIPS

echo "miri lane green on both targets"
