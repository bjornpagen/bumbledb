#!/bin/sh
# Stand-in C compiler for the Miri lane's cross-interpretation pass
# (scripts/miri.sh, the crucible packet (git ecec1dc3)).
#
# Why it exists: `cargo miri test --target x86_64-unknown-linux-gnu`
# still runs every build script natively, and lmdb-master-sys's build
# script compiles LMDB's C for the *requested* target — which needs a
# linux cross toolchain this host does not have. Under Miri that
# staticlib is a build-graph artifact only: cargo-miri never links a
# native binary (the "executable" it produces is interpreter metadata),
# and the lane's test filter never calls into LMDB (an mdb_* call is
# the exact FFI wall the lane is scoped around — Miri would refuse it
# by name). So a host-arch compile of the same sources satisfies the
# build graph without pretending to be a linux toolchain.
#
# Mechanism: strip the --target/-target arguments and compile with the
# host cc. Everything else (includes, -o, -c, archiving by cc-rs)
# passes through untouched.
#
# One special case: assembly units. blake3's build script hands the
# x86-64 `.S` sources (c/blake3_*_x86-64_unix.S) to whatever compiler
# it finds for the x86_64 target — and finding THIS shim is what made
# it try: absent a cross cc the script falls back to blake3's portable
# Rust body instead. The host arm64 assembler cannot accept x86-64
# mnemonics, and it never needs to: blake3 hard-guards every x86 SIMD
# dispatch behind `cfg!(miri)` (each *_detected() returns false under
# the interpreter), so the assembly is dead weight in a staticlib
# nothing calls — the same rationale as LMDB's above. When a foreign
# --target was stripped and the input is a `.S` file, emit an empty
# object: the build graph is satisfied, the symbols are never
# referenced.
args=""
skip=0
stripped=0
has_asm=0
out=""
grab_out=0
for a in "$@"; do
    if [ "$skip" -eq 1 ]; then
        skip=0
        continue
    fi
    if [ "$grab_out" -eq 1 ]; then
        out="$a"
        grab_out=0
    fi
    case "$a" in
        --target=*)
            stripped=1
            continue
            ;;
        -target | --target)
            skip=1
            stripped=1
            continue
            ;;
        -o) grab_out=1 ;;
        *.S) has_asm=1 ;;
    esac
    args="$args '$(printf %s "$a" | sed "s/'/'\\\\''/g")'"
done
if [ "$stripped" -eq 1 ] && [ "$has_asm" -eq 1 ] && [ -n "$out" ]; then
    printf '' | cc -x c -c -o "$out" -
    exit
fi
eval "exec cc $args"
